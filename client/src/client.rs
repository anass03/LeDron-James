use crossbeam_channel::{select_biased, tick, Receiver, Sender};
use wg_2024::network::{NodeId};
use wg_2024::packet::{Packet, PacketType, NodeType, Fragment};
use std::collections::{HashMap};
use std::error::Error;
use std::sync::atomic::{Ordering};
use std::time::{Duration, Instant};
use host::{HostCommand, Message, Host, ServerTypes, ServerToClient, HostEvent, ClientNetCommand, ContentID, FileId, MediaId, Image};
use host::network::Edge;
use host::ServerTypes::{Content, Chat};
use host::ServerToClient::{ClientList, ServerType, FilesList, File, Media, MediaList, MessageFrom, ErrNotFound, ErrClientId, FileFrom, MediaFrom, WrongServer, Sending};
use host::session::{generate_unique_session_id, RecSessionData};
use crate::compression::{compress_data, decompress_data};
use crate::transfer_tracker::TransferTracker;
use crate::network_metrics::NetMetrics;
use host::flood::FloodControl;
use host::functionality::Functionality;
use host::logging::{LogControl, LogEntry, LogModule, LogType};

/// Top‐level client wrapping a Host.
///
/// Issues high‐level commands (FilesList, Chat messages, uploads)
/// and forwards reconstructed `ServerToClient` events back to the simulation controller.
pub struct Client {
    pub(crate) host: Host,
    controller_recv:Receiver<ClientCommand>,
    servers: HashMap<ServerTypes, Vec<NodeId>>,
    servers_discovered: usize,
    net_metrics: NetMetrics,
    tracker: TransferTracker,
    pub(crate) pending_fragments: HashMap<ContentID, (usize, usize)>, // ContentID → (totali, ricevuti finora)
}
impl Client {
    /// Creates a new `Client`.
    ///
    /// # Parameters
    ///
    /// - `id`: the node identifier for this client.
    /// - `type_host`: whether this host is a client or server.
    /// - `controller_send`: channel for sending `HostEvent`s.
    /// - `controller_recv`: channel for receiving `ClientCommand`s.
    /// - `packet_recv`: channel for incoming low‐level `Packet`s.
    /// - `packet_send`: map of neighbor `NodeId` to their send‐channels.
    /// - `log_sender`: channel to emit structured `LogEntry`s.
    pub fn new(id:NodeId,
               type_host:NodeType,
               controller_send:Sender<HostEvent>,
               controller_recv:Receiver<ClientCommand>,
               packet_recv:Receiver<Packet>,
               packet_send:HashMap<NodeId, Sender<Packet>>,
               log_sender: Sender<LogEntry>) ->Self{

        let host = Host::new(id, type_host, controller_send,  packet_recv, packet_send, log_sender);
        Self{
            host,
            controller_recv,
            servers: HashMap::new(),
            servers_discovered: 0,
            net_metrics: NetMetrics::new(),
            tracker: TransferTracker::new(),
            pending_fragments: HashMap::new(),
        }
    }

    /// Starts the client's main event loop.
    ///
    /// 1. Initiates a flood discovery to learn network topology.
    /// 2. Selects over:
    ///    - incoming `ClientCommand`s,
    ///    - incoming raw `Packet`s,
    ///    - a 60s cleanup ticker,
    ///    - a 100ms PDR & throughput reporting ticker.
    pub fn run(&mut self) {
        let cleanup_tick = tick(Duration::from_secs(60));
        let speed_tick = tick(Duration::from_millis(5000));

        self.host.initiate_flood();
        loop {
            select_biased!{
                recv(self.controller_recv) -> cmd => {
                    if let Ok(command) = cmd {
                        self.handle_command(command);
                    }
                },
                recv(self.host.packet_recv) -> pkt => {
                    if let Ok(packet) = pkt {
                        if let Err(e) = self.handle_packet(packet) {
                            self.host.send_logging(
                                LogType::ERROR, LogModule::SESSION,
                                format!("Error processing packet: {}", e)
                            );
                        }
                    }
                },
                recv(cleanup_tick) -> _ => {
                    let timeout = Duration::from_secs(300);
                    self.cleanup_expired_sessions(timeout);
                },
                recv(speed_tick) -> _ => {
                    //sending expected pdr to simulation
                    // match self.host.controller_send.send(HostEvent::ExpectedPDR(self.host.network.drone_pdr.clone())) {
                    //     Ok(_) => {}
                    //     Err(e) => {self.host.send_logging(LogType::ERROR, LogModule::NETWORK,format!("Error reporting PDR: {}", e));}
                    // }
                    //
                    // //sending info on network speed
                    // let (up, down) = self.net_metrics.update();
                    // self.host.send_logging(
                    //         LogType::DEBUG, LogModule::NETWORK,
                    //         format!("✅ Reporting net speed: {:?}", (up, down))
                    //     );
                    // if let Err(e) = self.host.controller_send.send(HostEvent::NetSpeed(up, down)) {
                    //     self.host.send_logging(
                    //         LogType::ERROR, LogModule::NETWORK,
                    //         format!("Error reporting net speed: {}", e)
                    //     );
                    // }
                },
            }
        }
    }

    /// Ensures all servers have been discovered before sending network commands.
    ///
    /// Repeatedly sends `ServerType` discovery requests until
    /// `servers_discovered == host.network.servers.len()`.
    fn ensure_all_servers_discovered(&mut self) -> Result<(), String> {
        let expected = self.host.network.servers.len();

        if self.servers_discovered != expected {
            self.host.send_logging(
                LogType::INFO, LogModule::SESSION,
                format!("Discovering {} servers (host={})", expected, self.host.id),
            );
            self.request_server_type();

            while self.servers_discovered < expected {
                crossbeam_channel::select! {
                    recv(self.host.packet_recv) -> pkt => {
                         if let Ok(packet) = pkt {
                            if let Err(e) = self.handle_packet(packet) {
                                self.host.send_logging(
                                    LogType::ERROR, LogModule::SESSION,
                                    format!("Error processing packet: {}", e),
                                );
                            }
                        }
                    },

                    recv(self.controller_recv) -> _cmd => {
                        self.host.send_logging(
                            LogType::WARN, LogModule::SESSION,
                            "Received command during discovery".to_string(),
                        );
                    },

                    default(Duration::from_millis(200)) => {
                        // Se non arriva nulla per 200 ms, rilancio una nuova ondata di ServerType
                        self.request_server_type();
                    }
                }
            }

            self.host.send_logging(
                LogType::INFO, LogModule::SESSION,
                format!("Discovered all {} servers (host={})", expected, self.host.id),
            );
        }
        Ok(())
    }

    /// Sends a `ServerType` discovery request to every server node.
    fn request_server_type(&mut self) {
        for server_id in self.host.network.servers.clone() {
            self.host.send_logging(
                LogType::DEBUG, LogModule::SESSION,
                format!("Requesting ServerType from {}", server_id),
            );
            let raw = match serde_json::to_vec(&ClientNetCommand::ServerType) {
                Ok(buf) => buf,
                Err(e) => {
                    self.host.send_logging(
                        LogType::ERROR, LogModule::SESSION,
                        format!("Serialization failed: {}", e),
                    );
                    return;
                }
            };
            let payload = match compress_data(&raw) {
                Ok(c) => c,
                Err(err) => {
                    self.host.send_logging(
                        LogType::WARN, LogModule::SESSION,
                        format!("Compression failed, sending raw: {}", err),
                    );
                    raw
                }
            };

            self.net_metrics.tx_bytes.fetch_add(payload.len(), Ordering::Relaxed);

            let message = Message {
                session_id: generate_unique_session_id(),
                content: payload,
            };
            let cb = Box::new(|_: usize| {});
            if let Err(err) = self.host.build_msg_fragment(server_id, message, cb) {
                self.host.send_logging(
                    LogType::ERROR, LogModule::SESSION,
                    format!("Failed to send ServerType request: {}", err),
                );
            } else {
                self.host.send_logging(
                    LogType::INFO, LogModule::SESSION,
                    format!("Sent ServerType request to {}", server_id),
                );
            }
        }
    }

    /// Sends a high‐level `ClientNetCommand` to a specified server.
    ///
    /// # Returns
    ///
    /// `Some((server_id, message))` if the server was valid and fragmentation started;
    /// otherwise `None`.
    pub fn send_net_cmd(&mut self, srv_type: ServerTypes, server_id: NodeId, cmd: ClientNetCommand, label: &str, option: Option<Box<dyn FnMut(usize) + Send + 'static>>) -> Option<(NodeId, Message)>{
        self.host.send_logging(
            LogType::INFO, LogModule::SESSION,
            format!("Preparing {} request", label),
        );
        match self.servers.get(&srv_type) {
            Some(servers) if servers.contains(&server_id) => {
                let raw = match serde_json::to_vec(&cmd) {
                    Ok(buf) => buf,
                    Err(e) => {
                        self.host.send_logging(
                            LogType::ERROR, LogModule::SESSION,
                            format!("Serialization failed: {}", e),
                        );
                        return None;
                    }
                };

                let payload = match compress_data(&raw) {
                    Ok(c) => c,
                    Err(err) => {
                        self.host.send_logging(
                            LogType::WARN, LogModule::SESSION,
                            format!("Compression failed, sending raw: {}", err),
                        );
                        raw.clone()
                    }
                };

                // aggiorno il contatore di upload con la lunghezza del payload effettivo
                self.net_metrics.tx_bytes.fetch_add(payload.len(), Ordering::Relaxed);

                let message = Message {
                    session_id: generate_unique_session_id(),
                    content: payload,
                };
                let mut cb: Box<dyn FnMut(usize) + Send> =
                    option.unwrap_or_else(|| Box::new(|_: usize| {}));
                if let Err(err) =
                    self.host.build_msg_fragment(server_id, message.clone(), cb)
                {
                    self.host.send_logging(
                        LogType::ERROR, LogModule::SESSION,
                        format!("Failed to send {}: {}", label, err),
                    );
                } else {
                    self.host.send_logging(
                        LogType::INFO, LogModule::SESSION,
                        format!("{} sent to server {}", label, server_id),
                    );
                }
                Some((server_id,message))
            }
            Some(_) => {
                self.host.send_logging(
                    LogType::WARN, LogModule::SESSION,
                    format!("{} is not a {:?} server", server_id, srv_type),
                );
                None
            }
            None => {
                self.host.send_logging(
                    LogType::WARN, LogModule::SESSION,
                    format!("No {:?} server available for {}", srv_type, label),
                );
                None
            }
        }
    }

    /// Processes a high‐level `ClientCommand` from the controller.
    ///
    /// - For `Net(_)`, ensures all servers are discovered before dispatching to
    ///   the appropriate network command (FilesList, File, Media, Chat, etc.).
    /// - For `Internal(_)`, adds or removes packet‐send channels and updates topology.
    /// - For `ControllerShortcut(_)`, processes the packet directly and retriggers flood.
    fn handle_command(&mut self, client_command: ClientCommand) {
        match client_command {
            ClientCommand::Net(net_command) => {
                if let Err(e) = self.ensure_all_servers_discovered(){
                    self.host.send_logging(
                        LogType::ERROR,
                        LogModule::SESSION,
                        format!("Error during discovery: {}", e),
                    );
                    return;
                }
                match net_command {
                    ClientNetCommand::ServerType => {
                        self.host.send_logging(
                            LogType::INFO,
                            LogModule::SESSION,
                            "ServerType request received".into(),
                        );
                        let snapshot = self.servers.clone();
                        self.host.send_logging(
                            LogType::INFO,
                            LogModule::SESSION,
                            format!("Servers discovered: {:?}", snapshot),
                        );
                        let _ = self.host.controller_send.send(HostEvent::ServersDiscovered(self.host.id, snapshot));
                    }

                    // Content-server
                    ClientNetCommand::FilesList(sid) => {
                        self.send_net_cmd(Content, sid, ClientNetCommand::FilesList(sid), "FilesList", None);
                    }
                    ClientNetCommand::File(sid, id) => {
                        let label = format!("File({})", id);
                        self.send_net_cmd(Content, sid, ClientNetCommand::File(sid, id), &label, None);
                    }
                    ClientNetCommand::NewFile(sid, id, data) => {
                        self.add_new_file(sid, id, data);
                    }
                    ClientNetCommand::MediaList(sid) => {
                        self.send_net_cmd(Content, sid, ClientNetCommand::MediaList(sid), "MediaList", None);
                    }
                    ClientNetCommand::Media(sid, id) => {
                        let label = format!("Media({})", id);
                        self.send_net_cmd(Content, sid, ClientNetCommand::Media(sid, id), &label, None);
                    }
                    ClientNetCommand::NewMedia(sid, id, img) => {
                       self.add_new_media(sid, id, img);
                    }
                    // Chat-server
                    ClientNetCommand::RegistrationToChat(sid) => {
                        self.send_net_cmd(Chat, sid, ClientNetCommand::RegistrationToChat(sid), "RegistrationToChat", None);
                    }
                    ClientNetCommand::ClientList(sid) => {
                        self.send_net_cmd(Chat, sid, ClientNetCommand::ClientList(sid), "ClientList", None);
                    }
                    ClientNetCommand::MessageFor(cid, sid, msg) => {
                       self.handle_message_for(cid, sid, msg);
                    }
                    ClientNetCommand::FileFor(cid, sid, fid, data) => {
                        self.handle_file_for(cid, sid, fid, data);
                    }
                    ClientNetCommand::MediaFor(cid, sid, mid, img) => {
                        self.handle_media_for(cid, sid, mid, img);
                    }
                }
            }
            ClientCommand::Internal(host_command) => {
                match host_command {
                    HostCommand::AddSender(id, sender) => {
                        let current = self.host.packet_send.len();
                        if self.host.packet_send.contains_key(&id) {
                            self.host.send_logging(
                                LogType::WARN,
                                LogModule::SESSION,
                                "Channel already present".into(),
                            );
                        } else if current >= 2 {
                            self.host.send_logging(
                                LogType::ERROR,
                                LogModule::SESSION,
                                "Cannot add more than 2 channels: limit reached".into(),
                            );
                        } else {
                            self.host.packet_send.insert(id, sender);
                            self.host.send_logging(
                                LogType::INFO,
                                LogModule::SESSION,
                                format!("Added sender for {}", id),
                            );
                            let entry = self.host.network.nodes
                                .entry(self.host.id)
                                .or_insert_with(|| (NodeType::Client, Vec::new()));
                            entry.1.push(Edge { to: id, weight: 0.0 });

                            let entry = self.host.network.nodes
                                .entry(id)
                                .or_insert_with(|| (NodeType::Drone, Vec::new()));
                            entry.1.push(Edge { to: self.host.id, weight: 0.0 });
                        }
                    }
                    HostCommand::RemoveSender(id) => {
                        let current = self.host.packet_send.len();
                        if !self.host.packet_send.contains_key(&id) {
                            self.host.send_logging(
                                LogType::WARN,
                                LogModule::SESSION,
                                "Channel not found".into(),
                            );
                        } else if current <= 1 {
                            self.host.send_logging(
                                LogType::ERROR,
                                LogModule::SESSION,
                                "Cannot remove last channel: at least one required".into(),
                            );
                        } else {
                            self.host.packet_send.remove(&id);
                            self.host.send_logging(
                                LogType::INFO,
                                LogModule::SESSION,
                                format!("Removed sender for {}", id),
                            );

                            if let Some((_, edges)) = self.host.network.nodes.get_mut(&self.host.id) {
                                edges.retain(|e| e.to != id);
                            }
                            if let Some((_, edges)) = self.host.network.nodes.get_mut(&id) {
                                edges.retain(|e| e.to != self.host.id);
                            }
                        }
                    }
                }
            }
            ClientCommand::ControllerShortcut(packet) => {
                self.host.send_logging(
                    LogType::INFO,
                    LogModule::SESSION,
                    "Handling ControllerShortcut packet".into(),
                );
                if let Err(e) = self.handle_packet(packet) {
                    self.host.send_logging(
                        LogType::ERROR,
                        LogModule::SESSION,
                        format!("Error processing shortcut packet: {}", e),
                    );
                }
                self.host.initiate_flood();
            }
        }
    }

    /// Processes a raw `Packet` and delegates to flood, fragment,
    /// ACK, or NACK handlers as appropriate.
    ///
    /// # Errors
    ///
    /// Returns an error string if any handler fails.
    fn handle_packet(&mut self, packet: Packet) -> Result<(), String> {
        match packet.pack_type.clone() {
            PacketType::FloodResponse(_) => {
                self.host.process_flood_response(packet);
                Ok(())
            }
            PacketType::MsgFragment(fragment) => {
                self.net_metrics.rx_bytes.fetch_add(fragment.length as usize, Ordering::Relaxed);
                if let Some(compressed_bytes) = self.process_fragment(packet.clone(), fragment) {
                    // 1) prima decomprimo
                    let raw = decompress_data(&compressed_bytes).unwrap_or_else(|_| compressed_bytes);

                    // 2) poi deserializzo
                    let cmd = match serde_json::from_slice::<ServerToClient>(&raw) {
                        Ok(c) => c,
                        Err(err) => {
                            self.host.send_logging(
                                LogType::ERROR,
                                LogModule::SESSION,
                                format!("JSON parse failed: {}", err),
                            );
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
                                    self.host.send_logging(
                                        LogType::INFO,
                                        LogModule::SESSION,
                                        format!("Registered {:?} server at {}", st, src),
                                    );
                                }
                            } else {
                                self.host.send_logging(
                                    LogType::WARN,
                                    LogModule::SESSION,
                                    "ServerType without source".into(),
                                );
                            }
                        }
                        Sending(content_id, total_fragments) => {
                            self.pending_fragments.insert(content_id.clone(), (total_fragments, 0));
                            self.host.send_logging(
                                LogType::INFO,
                                LogModule::SESSION,
                                format!("Incoming content {:?}: {} fragments", content_id, total_fragments),
                            );
                        }
                        ClientList(client_id, server_id,clients) => {
                            self.host.send_logging(
                                LogType::INFO,
                                LogModule::SESSION,
                                format!("Client list from {}: {:?}", server_id, clients),
                            );
                            send_reconstructed(ClientList(client_id, server_id,clients))?;
                        }

                        MessageFrom(server_id,from, msg) => {
                            self.host.send_logging(
                                LogType::INFO,
                                LogModule::SESSION,
                                format!("Message from {}: {}", from, msg),
                            );
                            send_reconstructed(MessageFrom(server_id,from, msg))?;
                        }
                        FileFrom(server_id, from, file_id, data) => {
                            self.host.send_logging(
                                LogType::INFO,
                                LogModule::FILE,
                                format!("Received file `{}` from client `{}`", file_id, from),
                            );
                            send_reconstructed(FileFrom(server_id, from, file_id, data))?;
                        }

                        MediaFrom(server_id, from, media_id, data) => {
                            self.host.send_logging(
                                LogType::INFO,
                                LogModule::MEDIA,
                                format!("Received media `{}` from client `{}`", media_id, from),
                            );
                            send_reconstructed(MediaFrom(server_id, from, media_id, data))?;
                        }
                        FilesList(client_id, server_id, files) => {
                            self.host.send_logging(
                                LogType::INFO,
                                LogModule::FILE,
                                format!("Files available on server `{}`: {:?}", server_id, files),
                            );
                            send_reconstructed(FilesList(client_id, server_id, files))?;
                        }

                        File(server_id, file_id, data) => {
                            self.host.send_logging(
                                LogType::INFO,
                                LogModule::FILE,
                                format!("Received file `{}` ({} bytes) from server `{}`", file_id, data.len(), server_id),
                            );
                            send_reconstructed(File(server_id, file_id, data))?;
                        }

                        MediaList(client_id, server_id, media_list) => {
                            self.host.send_logging(
                                LogType::INFO,
                                LogModule::MEDIA,
                                format!("Media list from server `{}`: {:?}", server_id, media_list),
                            );
                            send_reconstructed(MediaList(client_id, server_id, media_list))?;
                        }
                        Media(server_id, media_id, media) => {
                            self.host.send_logging(
                                LogType::INFO,
                                LogModule::MEDIA,
                                format!("Received media `{}` ({} bytes) from server `{}`", media_id, media.data.len(), server_id),
                            );
                            send_reconstructed(Media(server_id, media_id, media))?;
                        }

                        ErrClientId(server_id) => {
                            self.host.send_logging(
                                LogType::ERROR,
                                LogModule::SESSION,
                                format!("Server `{}` responded with ErrClientId", server_id),
                            );
                            send_reconstructed(ErrClientId(server_id))?;
                        }

                        ErrNotFound(server_id) => {
                            self.host.send_logging(
                                LogType::ERROR,
                                LogModule::SESSION,
                                format!("Server `{}` responded with ErrNotFound", server_id),
                            );
                            send_reconstructed(ErrNotFound(server_id))?;
                        }
                        WrongServer(server_id) => {
                            self.host.send_logging(
                                LogType::ERROR,
                                LogModule::SESSION,
                                format!("Server `{}` responded with WrongServer", server_id),
                            );
                            send_reconstructed(WrongServer(server_id))?;
                        }
                    }
                    Ok(())
                } else {
                    // Message not yet assembled: wait for other fragments
                    self.host.send_logging(
                        LogType::DEBUG,
                        LogModule::SESSION,
                        format!(
                            "Fragment {} pending assembly",
                            packet.session_id
                        ),
                    );
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
                }else { self.host.send_logging(
                    LogType::ERROR,
                    LogModule::SESSION,
                    "NACK packet missing source".into(),
                ); }
                Ok(())
            }
        }
    }

    /// Cleans up expired or fully‐confirmed send sessions after `timeout`.
    fn cleanup_expired_sessions(&self, timeout: Duration) {
        let host = self.host.id;
        self.host.send_logging(
            LogType::DEBUG, LogModule::SESSION,
            format!("Starting cleanup host={} timeout={:?}", host, timeout),
        );
        let now = Instant::now();
        let mut to_remove = Vec::new();

        if let Ok(sent) = self.host.sent_fragments.lock() {
            for (&sid, sess) in sent.iter() {
                let expired = now.duration_since(sess.last_activity) > timeout;
                let complete = sess.buffer.iter().all(Option::is_none);
                if expired || complete {
                    to_remove.push((sid, expired));
                }
            }
        } else {
            self.host.send_logging(
                LogType::ERROR, LogModule::SESSION,
                "Failed to lock sent_fragments for cleanup".into(),
            );
            return;
        }

        if let Ok(mut sent) = self.host.sent_fragments.lock() {
            for (sid, expired) in to_remove {
                sent.remove(&sid);
                let reason = if expired { "expired" } else { "completed" };
                self.host.send_logging(
                    LogType::INFO, LogModule::SESSION,
                    format!("Removed {} session {}", reason, sid),
                );
            }
        } else {
            self.host.send_logging(
                LogType::ERROR, LogModule::SESSION,
                "Failed to lock sent_fragments for removal".into(),
            );
        }
    }

    /// Handles receipt and reassembly of a single `Fragment`.
    ///
    /// Stores the fragment, updates download progress, sends an ACK,
    /// and returns the full reassembled payload when complete.
    fn process_fragment(&mut self, packet: Packet, fragment: Fragment) -> Option<Vec<u8>> {
        let session_id = packet.session_id;
        let idx = fragment.fragment_index as usize;
        let total = fragment.total_n_fragments as usize;

        // inizializza se necessario
        let mut received = match self.host.received_fragments.lock() {
            Ok(r) => r,
            Err(e) => {
                self.host.send_logging(
                    LogType::ERROR, LogModule::SESSION,
                    "Failed to lock received_fragments".into(),
                );
                return None;
            }
        };

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

            self.host.send_logging(
                LogType::INFO, LogModule::SESSION,
                format!("Fragment {} stored, pending {}", idx, session.pending),
            );

            if let Some(content_id) = self.pending_fragments.iter_mut().find_map(|(cid, (total, received))| {
                // Criterio: la sessione è ancora attiva e non completata
                if *received < *total {
                    *received += 1;
                    Some(cid.clone())
                } else {
                    None
                }
            }) {

                // Notifica il Simulation Controller
                let (total, received) = self.pending_fragments[&content_id];
                let percent = (received as f32 / total as f32) * 100.0;

                let _ = self.host.controller_send.send(HostEvent::DownloadingInfo(content_id.clone(), percent));
                self.host.send_logging(
                    LogType::DEBUG, LogModule::SESSION,
                    format!("💥 Downloading info: {:?}", percent),
                );
                // Rimuovi se completato
                if received == total {
                    self.pending_fragments.remove(&content_id);
                    self.host.send_logging(
                        LogType::INFO, LogModule::SESSION,
                        format!("Completed download {:?}", content_id),
                    );
                }
            }

        }

        // invia ACK
        let mut ack = self.host.build_packet_ack(packet.clone(), Some(fragment.fragment_index));
        ack.routing_header.hop_index += 1;
        let _ = self.host.send_packet(ack, None);

        let drone_hops: Vec<NodeId> = packet.routing_header.hops.iter()
            .filter(|&&nid| { matches!(
                    self.host.network.nodes.get(&nid).map(|(ty, _)| ty),
                    Some(NodeType::Drone))}
            )
            .copied()
            .collect();
        self.host.network.pdr_helper(&drone_hops, &self.host.log_sender);

        // se finito, reassembly e rimozione
        if session.pending == 0 {
            self.host.send_logging(
                LogType::INFO, LogModule::SESSION,
                format!("All fragments received for {}", session_id),
            );

            let data = session.buffer.iter()
                .filter_map(|f| f.as_ref())
                .flat_map(|f| f.data[..f.length as usize].iter().cloned())
                .collect::<Vec<u8>>();

            received.remove(&session_id);
            return Some(data);
        }
        None
    }

    /// Adds a new file to specified content server.
    fn add_new_file(&mut self, sid: NodeId, id: FileId, data: Vec<u8>) {
        let label = format!("Adding new file({}) for server {}", id, sid);
        let content_id = ContentID::File(id.clone());
        self.tracker.start(content_id.clone(), data.len());
        let closure = self.tracker.get_closure(
            content_id,
            self.host.controller_send.clone(),
        );
        let boxed = Box::new(closure);
        self.send_net_cmd(Content, sid, ClientNetCommand::NewFile(sid, id, data.clone()), &label, Some(boxed));
    }

    /// Adds a new media to a specified content server.
    fn add_new_media(&mut self, sid: NodeId, id: MediaId, img: Image) {
        let label = format!("Adding new media({}) for server {}", id, sid);
        let content_id = ContentID::Media(id.clone());

        self.tracker.start(content_id.clone(), img.data.len());
        let closure = self.tracker.get_closure(
            content_id,
            self.host.controller_send.clone(),
        );
        let boxed = Box::new(closure);
        self.send_net_cmd(Content, sid, ClientNetCommand::NewMedia(sid, id.clone(), img.clone()), &label, Some(boxed));
    }

    /// Forwards a chat message to another client.
    fn handle_message_for(&mut self, cid: NodeId, sid: NodeId, msg: String) {
        let label = format!("MessageFor({})", cid);
        self.send_net_cmd(Chat, sid, ClientNetCommand::MessageFor(cid, sid, msg), &label, None);
    }

    /// Forwards a file via chat.
    fn handle_file_for(&mut self, cid: NodeId, sid: NodeId, fid: FileId, data: Vec<u8>) {
        let label = format!("FileFor({})", cid);
        let content_id = ContentID::File(fid.clone());
        self.tracker.start(content_id.clone(), data.len());
        let closure = self.tracker.get_closure(
            content_id,
            self.host.controller_send.clone(),
        );
        let boxed = Box::new(closure);
        self.send_net_cmd(Chat, sid, ClientNetCommand::FileFor(cid, sid, fid, data.clone()), &label, Some(boxed));
    }

    /// Forwards a file via chat.
    fn handle_media_for(&mut self, cid: NodeId, sid: NodeId, mid: MediaId, img: Image) {
        let label = format!("MediaFor({})", cid);
        let content_id = ContentID::Media(mid.clone());

        self.tracker.start(content_id.clone(), img.data.len());
        let closure = self.tracker.get_closure(
            content_id,
            self.host.controller_send.clone(),
        );
        let boxed = Box::new(closure);
        self.send_net_cmd(Chat, sid, ClientNetCommand::MediaFor(cid, sid, mid.clone(), img.clone()), &label, Some(boxed));
    }
}
pub enum ClientCommand {
    /// Wraps a network-level command
    Net(ClientNetCommand),
    /// A local internal management command
    Internal(HostCommand),
    /// Packet shortcut delivered by the SimulationController
    ControllerShortcut(Packet),
}
