use crossbeam_channel::{select_biased, tick, Receiver, Sender};
use wg_2024::network::{NodeId};
use wg_2024::packet::{Packet, PacketType, NodeType, Fragment};
use std::collections::{HashMap};
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use host::{HostCommand, Message, Host, Functionality, generate_unique_session_id, ServerTypes, RecSessionData, ServerToClient, HostEvent, ClientNetCommand, Edge};
use host::ServerTypes::{Content,Chat};
use host::ServerToClient::{ClientList, ServerType, FilesList, File, Media, MediaList, MessageFrom, ErrNotFound, ErrClientId, FileFrom, MediaFrom, WrongServer};
use std::io::{Cursor};
use zstd::{decode_all, encode_all};

pub struct Client {
    host: Host,
    controller_recv:Receiver<ClientCommand>,
    servers: HashMap<ServerTypes, Vec<NodeId>>,
    servers_discovered: usize,
    tx_bytes: Arc<AtomicUsize>,
    rx_bytes: Arc<AtomicUsize>,
    active_downloads: HashMap<String, TransferProgress>,
    active_uploads: HashMap<String, TransferProgress>,
}
pub struct TransferProgress {
    pub total_bytes: usize,
    pub transferred_bytes: Arc<AtomicUsize>,
    pub start_time: Instant,
    pub transfer_id: String, // FileId or MediaId as string
}
impl Client {
    pub fn new(id:NodeId,
               type_host:NodeType,
               controller_send:Sender<HostEvent>,
               controller_recv:Receiver<ClientCommand>,
               packet_recv:Receiver<Packet>,
               packet_send:HashMap<NodeId, Sender<Packet>>,
               log_sender: Sender<String>) ->Self{

        let host = Host::new(id, type_host, controller_send,  packet_recv, packet_send, log_sender);
        Self{
            host,
            controller_recv,
            servers: HashMap::new(),
            servers_discovered: 0,
            tx_bytes: Arc::new(AtomicUsize::new(0)),
            rx_bytes: Arc::new(AtomicUsize::new(0)),
            active_downloads: HashMap::new(),
            active_uploads: HashMap::new()
        }
    }

    pub fn run(&mut self) {
        let cleanup_tick = tick(Duration::from_secs(60));

        let speed_tick = tick(Duration::from_millis(100));
        let mut prev_tx = 0;
        let mut prev_rx = 0;

        self.host.initiate_flood();
        loop {
            select_biased!{
                // 1) comandi dal controller
                recv(self.controller_recv) -> cmd => {
                    if let Ok(command) = cmd {
                        self.handle_command(command);
                    }
                },

                // 2) pacchetti in arrivo
                recv(self.host.packet_recv) -> pkt => {
                    if let Ok(packet) = pkt {
                        if let Err(e) = self.handle_packet(packet) {
                            self.host.send_logging(format!("Error processing packet: {}", e));
                        }
                    }
                },
                recv(cleanup_tick) -> _ => {
                    let timeout = Duration::from_secs(300);
                    self.host.cleanup_expired_sessions(timeout);
                },
                recv(speed_tick) -> _ => {
                    match self.host.controller_send.send(HostEvent::ExpectedPDR(self.host.id, self.host.network.drone_pdr.clone())) {
                        Ok(_) => {}
                        Err(e) => {self.host.send_logging(format!("Error sending controller_send: {}", e));}
                    }
                    let total_tx = self.tx_bytes.load(Ordering::Relaxed);
                    let total_rx = self.rx_bytes.load(Ordering::Relaxed);
                    let up_speed = total_tx - prev_tx;   // B/s
                    let down_speed = total_rx - prev_rx;
                    prev_tx = total_tx;
                    prev_rx = total_rx;

                    // conversione in KiB/s
                    let up_kb  = up_speed as f64 / 1024.0;
                    let down_kb = down_speed as f64 / 1024.0;

                    match self.host.controller_send.send(HostEvent::NetSpeed(self.host.id, up_kb, down_kb)) {
                        Ok(_) => {}
                        Err(e) => {self.host.send_logging(format!("Error sending controller_send: {}", e));}
                    }
                },
            }
        }
    }

    /// Se non ho ancora scoperto *tutti* i server,
    /// mando la discovery e aspetto che `self.servers` contenga
    /// tutti gli ID di `self.host.network.servers`.
    fn ensure_all_servers_discovered(&mut self) -> Result<(), String> {
        let expected = self.host.network.servers.len();

        if self.servers_discovered != expected {
            self.host.send_logging(format!(
                "Discovering server types from {} nodes, host={}",
                expected, self.host.id
            ));
            self.request_server_type();

            while self.servers_discovered < expected {
                crossbeam_channel::select! {
                    recv(self.host.packet_recv) -> pkt => {
                         if let Ok(packet) = pkt {
                            if let Err(e) = self.handle_packet(packet) {
                                self.host.send_logging(format!("Error processing packet: {}", e));
                            }
                        }
                    },

                    recv(self.controller_recv) -> _cmd => {
                        self.host.send_logging("Got a command while discovering".to_string());
                    },

                    default(Duration::from_millis(200)) => {
                        // Se non arriva nulla per 200 ms, rilancio una nuova ondata di ServerType
                        self.request_server_type();
                    }
                }
            }

            self.host.send_logging(format!(
                "Discovered all {} servers, host={}",
                expected, self.host.id
            ));
        }
        Ok(())
    }


    fn send_net_cmd(&mut self, srv_type: ServerTypes, server_id: NodeId, cmd: ClientNetCommand, label: &str, ) {
        self.host.send_logging(format!("Preparing {} request", label));
        match self.servers.get(&srv_type) {
            Some(servers) if servers.contains(&server_id) => {
                let raw = match serde_json::to_vec(&cmd) {
                    Ok(buf) => buf,
                    Err(e) => {
                        self.host.send_logging(format!("Serialization failed: {}", e));
                        return;
                    }
                };

                let payload = match compress_data(&raw) {
                    Ok(c) => c,
                    Err(err) => {
                        self.host.send_logging(format!("Compression failed, invio raw: {}", err));
                        raw.clone()
                    }
                };

                // aggiorno il contatore di upload con la lunghezza del payload effettivo
                self.tx_bytes.fetch_add(payload.len(), Ordering::Relaxed);

                let message = Message {
                    session_id: generate_unique_session_id(),
                    content: payload,
                };

                if let Err(err) = self.host.build_msg_fragment(server_id, message) {
                    self.host.send_logging(format!("Failed to send {}: {}", label, err));
                } else {
                    self.host.send_logging(format!("{} sent to server {}", label, server_id));
                }
            }
            Some(_) => {
                self.host.send_logging(format!(
                    "Node {} is NOT a valid {:?}-SERVER",
                    server_id, srv_type
                ));
            }
            None => {
                self.host.send_logging(format!("No {:?} server available for {}", srv_type, label));
            }
        }
    }

    fn handle_command(&mut self, client_command: ClientCommand) {
        match client_command {
            ClientCommand::Net(net_command) => {
                if let Err(e) = self.ensure_all_servers_discovered(){
                    self.host.send_logging(format!("Error during discovery: {}", e));
                    return;
                }
                match net_command {
                    // rimane speciale
                    ClientNetCommand::ServerType => {
                        self.host.send_logging("ServerType request received".to_string());
                        let snapshot = self.servers.clone();
                        self.host.send_logging(format!("Servers discovered: {:?}", snapshot));
                        let _ = self.host.controller_send.send(HostEvent::ServersDiscovered(self.host.id, snapshot));
                    }

                    // Content-server
                    ClientNetCommand::FilesList(sid) => {
                        self.send_net_cmd(Content, sid, ClientNetCommand::FilesList(sid), "FilesList");
                    }
                    ClientNetCommand::File(sid, id) => {
                        let label = format!("File({})", id);
                        self.send_net_cmd(Content, sid, ClientNetCommand::File(sid, id), &label);
                    }
                    ClientNetCommand::NewFile(sid, id, data) => {
                        let label = format!("Adding new file({}) for server {}", id, sid);
                        self.send_net_cmd(Content, sid, ClientNetCommand::NewFile(sid, id, data), &label);
                    }
                    ClientNetCommand::MediaList(sid) => {
                        self.send_net_cmd(Content, sid, ClientNetCommand::MediaList(sid), "MediaList");
                    }
                    ClientNetCommand::Media(sid, id) => {
                        let label = format!("Media({})", id);
                        self.send_net_cmd(Content, sid, ClientNetCommand::Media(sid, id), &label);
                    }
                    ClientNetCommand::NewMedia(sid, id, data) => {
                        let label = format!("Adding new media({}) for server {}", id, sid);
                        self.send_net_cmd(Content, sid, ClientNetCommand::NewMedia(sid, id, data), &label);
                    }
                    // Chat-server
                    ClientNetCommand::RegistrationToChat(sid) => {
                        self.send_net_cmd(Chat, sid, ClientNetCommand::RegistrationToChat(sid), "RegistrationToChat");
                    }
                    ClientNetCommand::ClientList(sid) => {
                        self.send_net_cmd(Chat, sid, ClientNetCommand::ClientList(sid), "ClientList");
                    }
                    ClientNetCommand::MessageFor(cid, sid, msg) => {
                        let label = format!("MessageFor({})", cid);
                        self.send_net_cmd(Chat, sid, ClientNetCommand::MessageFor(cid, sid, msg), &label);
                    }
                    ClientNetCommand::FileFor(cid, sid, fid, file) => {
                        let label = format!("FileFor({})", cid);
                        self.send_net_cmd(Chat, sid,ClientNetCommand::FileFor(cid, sid, fid, file), &label, );
                    }
                    ClientNetCommand::MediaFor(cid, sid, mid, img) => {
                        let label = format!("MediaFor({})", cid);
                        self.send_net_cmd(Chat, sid, ClientNetCommand::MediaFor(cid, sid, mid, img), &label, );
                    }
                }
            }
            ClientCommand::Internal(host_command) => {
                match host_command {
                    HostCommand::AddSender(id, sender) => {
                        let current = self.host.packet_send.len();
                        if self.host.packet_send.contains_key(&id) {
                            self.host.send_logging("Channel already present!".to_string());
                        } else if current >= 2 {
                            self.host.send_logging("Cannot add more than 2 channels: limit reached".to_string());
                        } else {
                            // 1) Mantieni packet_send
                            self.host.packet_send.insert(id, sender);
                            self.host.send_logging(format!("Added sender for {}", id));

                            // Aggiungo l'arco dal client al drone
                            let entry = self.host.network.nodes
                                .entry(self.host.id)
                                .or_insert_with(|| (NodeType::Client, Vec::new()));
                            entry.1.push(Edge { to: id, weight: 0.0 });

                            // Aggiungo anche l'arco inverso: dal drone al client
                            let entry = self.host.network.nodes
                                .entry(id)
                                .or_insert_with(|| (NodeType::Drone, Vec::new()));
                            entry.1.push(Edge { to: self.host.id, weight: 0.0 });
                        }
                    }
                    HostCommand::RemoveSender(id) => {
                        let current = self.host.packet_send.len();
                        if !self.host.packet_send.contains_key(&id) {
                            self.host.send_logging("Channel not found!".to_string());
                        } else if current <= 1 {
                            self.host.send_logging("Cannot remove last channel: at least one connection required".to_string());
                        } else {
                            self.host.packet_send.remove(&id);
                            self.host.send_logging(format!("Removed sender for {}", id));

                            // 2) Elimina l’arco dalla topologia
                            if let Some((_, edges)) = self.host.network.nodes.get_mut(&self.host.id) {
                                edges.retain(|e| e.to != id);
                            }
                            // Rimuovi anche l'arco inverso dal drone
                            if let Some((_, edges)) = self.host.network.nodes.get_mut(&id) {
                                edges.retain(|e| e.to != self.host.id);
                            }
                        }
                    }
                }
            }
            ClientCommand::ControllerShortcut(packet) => {
                self.host.send_logging("Handling ControllerShortcut packet".to_string());
                if let Err(e) = self.handle_packet(packet) {
                    self.host.send_logging(format!("Error processing ControllerShortcut packet: {}", e));
                }
                // Una volta shortcuttato, rifaccio il flood
                self.host.initiate_flood();
            }
        }
    }

    fn handle_packet(&mut self, packet: Packet) -> Result<(), String> {
        match packet.pack_type.clone() {
            PacketType::FloodResponse(_) => {
                self.host.process_flood_response(packet);
                Ok(())
            }
            PacketType::MsgFragment(fragment) => {
                self.rx_bytes.fetch_add(fragment.length as usize, Ordering::Relaxed);
                if let Some(compressed_bytes) = self.handle_msg_fragment(packet.clone(), fragment) {
                    // 1) prima decomprimo
                    let raw = decompress_data(&compressed_bytes).unwrap_or_else(|_| compressed_bytes);

                    // 2) poi deserializzo
                    let cmd = match serde_json::from_slice::<ServerToClient>(&raw) {
                        Ok(c) => c,
                        Err(err) => {
                            self.host.send_logging(format!("JSON parse failed: {}", err));
                            return Ok(());
                        }
                    };
                    let send_reconstructed =
                        |evt: ServerToClient| {
                            self.host
                                .controller_send
                                .send(HostEvent::PacketReconstructed(evt))
                                .map_err(|e| e.to_string())
                        };
                    match cmd {
                        ServerType(st) => {
                            if let Some(src) = packet.routing_header.source() {
                                self.servers_discovered += 1;
                                let entry = self.servers.entry(st.clone()).or_insert_with(Vec::new);
                                if !entry.contains(&src) {
                                    entry.push(src);
                                    self.host.send_logging(format!("Registered {:?} server: {:?}", st, src));
                                }
                            } else {
                                self.host.send_logging("ServerType response without source".to_string());
                            }
                        }
                        ClientList(client_id, server_id,clients) => {
                            self.host.send_logging(format!("Received Client List: {:?}", clients));
                            send_reconstructed(ClientList(client_id, server_id,clients))?;
                        }

                        MessageFrom(server_id,from, msg) => {
                            self.host.send_logging(format!("Message from {}: {:?}", from, msg.clone()));
                            send_reconstructed(MessageFrom(server_id,from, msg))?;
                        }
                        FileFrom(server_id,from,file_id, data) => {
                            self.host.send_logging(format!("Received {} from client {}", file_id, from));
                            send_reconstructed(FileFrom(server_id,from,file_id, data))?;
                        }
                        MediaFrom(server_id,from, media_id, data) => {
                            self.host.send_logging(format!("Received {} from client {}", media_id, from));
                            send_reconstructed(MediaFrom(server_id, from, media_id, data))?;
                        }
                        FilesList(client_id, server_id, files) => {
                            self.host.send_logging(format!("Files available: {:?}", files));
                            send_reconstructed(FilesList(client_id,server_id, files))?;
                        }
                        File(server_id,file_id, data) => {
                            self.host.send_logging(format!("Received {} of {} bytes",file_id, data.len()));
                            send_reconstructed(File(server_id,file_id, data))?;
                        }
                        MediaList(client_id, server_id,media_list) => {
                            self.host.send_logging(format!("Media available: {:?}", media_list));
                            send_reconstructed(MediaList(client_id,server_id,media_list))?;
                        }
                        Media(server_id,media_id, media) => {
                            self.host.send_logging(format!("Received {} of {} bytes",media_id, media.data.len()));
                            send_reconstructed(Media(server_id,media_id, media))?;
                        }
                        ErrClientId(server_id) => {
                            self.host.send_logging("Server responded: ErrClientId".to_string());
                            send_reconstructed(ErrClientId(server_id))?;
                        }
                        ErrNotFound(server_id) => {
                            self.host.send_logging("Server responded: ErrNotFound".to_string());
                            send_reconstructed(ErrNotFound(server_id))?;
                        }
                        WrongServer(server_id) => {
                            self.host.send_logging("Server responded: WrongServer".to_string());
                            send_reconstructed(WrongServer(server_id))?;
                        }
                    }
                    Ok(())
                } else {
                    // Message not yet assembled: wait for other fragments
                    self.host.send_logging(format!(
                        "Fragment received but message not yet fully reassembled (session {})",
                        packet.session_id
                    ));
                    Ok(())
                }
            }

            PacketType::FloodRequest(flood_request) => {
                self.host.process_flood_request(flood_request, packet);
                Ok(())
            }

            PacketType::Ack(ack) => {
                self.host.handle_ack(ack,packet.routing_header.hops.as_slice(), packet.session_id);
                Ok(())
            }

            PacketType::Nack(nack) => {
                if let Some(src) = packet.routing_header.source() {
                    self.host.handle_nack(nack, src, packet.session_id);
                }else { self.host.send_logging("Packet must have source!".to_string()); }
                Ok(())
            }
        }
    }
    fn request_server_type(&mut self) {
        // for each known server, i send ServerType query
        for server_id in self.host.network.servers.clone() {
            self.host.send_logging(format!("Requesting server type from node {}", server_id));
            let raw = match serde_json::to_vec(&ClientNetCommand::ServerType) {
                Ok(buf) => buf,
                Err(e) => {
                    self.host.send_logging(format!("Serialization failed: {}", e));
                    return;
                }
            };

            let payload = match compress_data(&raw) {
                Ok(c) => c,
                Err(err) => {
                    self.host.send_logging(format!("Compression failed, invio raw: {}", err));
                    raw.clone()
                }
            };

            // aggiorno il contatore di upload con la lunghezza del payload effettivo
            self.tx_bytes.fetch_add(payload.len(), Ordering::Relaxed);

            let message = Message {
                session_id: generate_unique_session_id(),
                content: payload,
            };

            if let Err(err) = self.host.build_msg_fragment(server_id, message) {
                self.host.send_logging(format!("Failed to send ServerType request: {}", err));
            } else {
                self.host.send_logging(format!("ServerType request sent to server {}", server_id));
            }
        }
    }
    fn handle_msg_fragment(&mut self, packet: Packet, fragment: Fragment) -> Option<Vec<u8>> {
        let session_id = packet.session_id;
        let idx = fragment.fragment_index as usize;
        let total = fragment.total_n_fragments as usize;

        // inizializza se necessario
        let mut received = self.host.received_fragments.lock().unwrap();
        let session = received.entry(session_id).or_insert_with(|| {
            RecSessionData {
                buffer: vec![None; total],
                last_activity: Instant::now(),
                pending: total,  // <— inizializzo qui
            }
        });

        // aggiorna timestamp
        session.last_activity = Instant::now();

        // memorizza solo se nuovo
        if idx < session.buffer.len() && session.buffer[idx].is_none() {
            session.buffer[idx] = Some(fragment.clone());
            session.pending = session.pending.saturating_sub(1);

            self.host.send_logging(format!(
                "Received fragment, pending: {}, host: {}, session: {}",
                session.pending,self.host.id, session_id,
            ));
        }

        // invia ACK
        let mut ack = self.host.build_packet_ack(packet.clone(), Some(fragment.fragment_index));
        ack.routing_header.hop_index += 1;
        let _ = self.host.send_packet(ack, None);

        let drone_hops: Vec<NodeId> = packet.routing_header.hops.iter()
            .filter(|&&nid| {
                matches!(
                        self.host.network.nodes.get(&nid).map(|(ty, _)| ty),
                        Some(NodeType::Drone))}
            )
            .copied()
            .collect();
        self.host.network.pdr_helper(&drone_hops);

        self.host.send_logging(format!(
            "PDR updated host={} header hops={:?}",
            self.host.id,
            packet.routing_header.hops,
        ));
        // se finito, reassembly e rimozione
        if session.pending == 0 {
            self.host.send_logging( format!(
                "All fragments received host={} session={}",
                self.host.id,
                session_id,
            ));

            self.host.send_logging(
                format!("host: {}, session: {}, All fragments received",
                        self.host.id, session_id));

            let data = session.buffer
                .iter()
                .filter_map(|f| f.as_ref())
                .flat_map(|f| f.data[..f.length as usize].iter().cloned())
                .collect::<Vec<u8>>();

            received.remove(&session_id);
            return Some(data);
        }
        None
    }
    pub fn start_download_tracking(&mut self, transfer_id: String, total_bytes: usize) {
        let progress = TransferProgress {
            total_bytes,
            transferred_bytes: Arc::new(AtomicUsize::new(0)),
            start_time: Instant::now(),
            transfer_id: transfer_id.clone(),
        };

        self.active_downloads.insert(transfer_id.clone(), progress);
        self.host.send_logging(format!("Started tracking download: {} ({} bytes)", transfer_id, total_bytes));
    }
    pub fn start_upload_tracking(&mut self, transfer_id: String, total_bytes: usize) {
        let progress = TransferProgress {
            total_bytes,
            transferred_bytes: Arc::new(AtomicUsize::new(0)),
            start_time: Instant::now(),
            transfer_id: transfer_id.clone(),
        };

        self.active_uploads.insert(transfer_id.clone(), progress);
        self.host.send_logging(format!("Started tracking upload: {} ({} bytes)", transfer_id, total_bytes));
    }

    pub fn update_download_progress(&mut self, transfer_id: &str, bytes_received: usize) {
        if let Some(progress) = self.active_downloads.get(transfer_id) {
            let current = progress.transferred_bytes.fetch_add(bytes_received, Ordering::Relaxed) + bytes_received;
            let percentage = (current as f64 / progress.total_bytes as f64) * 100.0;
            let elapsed = progress.start_time.elapsed();
            let speed = if elapsed.as_secs() > 0 {
                current as f64 / elapsed.as_secs() as f64 / 1024.0 // KB/s
            } else {
                0.0
            };

            // Invia aggiornamento ogni 10% o quando completato
            if current >= progress.total_bytes || (current * 10 / progress.total_bytes) > ((current - bytes_received) * 10 / progress.total_bytes) {
                let info = format!("{}|{:.1}%|{:.1}KB/s|{}/{}bytes",
                                   transfer_id, percentage, speed, current, progress.total_bytes);

                match self.host.controller_send.send(HostEvent::DownloadingInfo(self.host.id, info, percentage)) {
                    Ok(_) => {
                        self.host.send_logging(format!("Download progress: {} - {:.1}%", transfer_id, percentage));
                    }
                    Err(e) => {
                        self.host.send_logging(format!("Error sending download info: {}", e));
                    }
                }

                // Rimuovi il tracking se completato
                if current >= progress.total_bytes {
                    self.active_downloads.remove(transfer_id);
                    self.host.send_logging(format!("Download completed: {}", transfer_id));
                }
            }
        }
    }

    pub fn update_upload_progress(&mut self, transfer_id: &str, bytes_sent: usize) {
        if let Some(progress) = self.active_uploads.get(transfer_id) {
            let current = progress.transferred_bytes.fetch_add(bytes_sent, Ordering::Relaxed) + bytes_sent;
            let percentage = (current as f64 / progress.total_bytes as f64) * 100.0;
            let elapsed = progress.start_time.elapsed();
            let speed = if elapsed.as_secs() > 0 {
                current as f64 / elapsed.as_secs() as f64 / 1024.0 // KB/s
            } else {
                0.0
            };

            // Invia aggiornamento ogni 10% o quando completato
            if current >= progress.total_bytes || (current * 10 / progress.total_bytes) > ((current - bytes_sent) * 10 / progress.total_bytes) {
                let info = format!("{}|{:.1}%|{:.1}KB/s|{}/{}bytes",
                                   transfer_id, percentage, speed, current, progress.total_bytes);

                match self.host.controller_send.send(HostEvent::UploadingInfo(self.host.id, info, percentage)) {
                    Ok(_) => {
                        self.host.send_logging(format!("Upload progress: {} - {:.1}%", transfer_id, percentage));
                    }
                    Err(e) => {
                        self.host.send_logging(format!("Error sending upload info: {}", e));
                    }
                }

                // Rimuovi il tracking se completato
                if current >= progress.total_bytes {
                    self.active_uploads.remove(transfer_id);
                    self.host.send_logging(format!("Upload completed: {}", transfer_id));
                }
            }
        }
    }

    pub fn cleanup_stale_transfers(&mut self) {
        let timeout = Duration::from_secs(300); // 5 minuti di timeout
        let now = Instant::now();

        // Cleanup downloads
        let stale_downloads: Vec<String> = self.active_downloads
            .iter()
            .filter(|(_, progress)| now.duration_since(progress.start_time) > timeout)
            .map(|(id, _)| id.clone())
            .collect();

        for id in stale_downloads {
            self.active_downloads.remove(&id);
            self.host.send_logging(format!("Removed stale download: {}", id));
        }

        // Cleanup uploads
        let stale_uploads: Vec<String> = self.active_uploads
            .iter()
            .filter(|(_, progress)| now.duration_since(progress.start_time) > timeout)
            .map(|(id, _)| id.clone())
            .collect();

        for id in stale_uploads {
            self.active_uploads.remove(&id);
            self.host.send_logging(format!("Removed stale upload: {}", id));
        }
    }
}
fn compress_data(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let compressed = encode_all(data, 3)?;
    Ok(compressed)
}

fn decompress_data(compressed: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let decompressed = decode_all(Cursor::new(compressed))?;
    Ok(decompressed)
}
pub enum ClientCommand {
    /// Wraps a network-level command
    Net(ClientNetCommand),
    /// A local internal management command
    Internal(HostCommand),
    /// Packet shortcut delivered by the SimulationController
    ControllerShortcut(Packet),
}
