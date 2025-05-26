use crossbeam_channel::{tick, Receiver, Sender};
use wg_2024::controller::{DroneEvent};
use wg_2024::network::{NodeId};
use wg_2024::packet::{Packet, PacketType, NodeType, Fragment};
use std::collections::{HashMap, HashSet};
use std::{fs, io, thread};
use std::time::{Duration, Instant};
use host::{ClientCommand, Message, Host, Functionality, generate_unique_session_id, ServerTypes, RecSessionData, ServerToClient, NetCommand, HostEvent, SendingCodes};
use host::ServerTypes::{Content,Chat};
use image::{DynamicImage, ImageBuffer};
use tracing_subscriber::{fmt, EnvFilter};
use host::ServerToClient::{ClientList, ServerType, FilesList, File, Media,MediaList, MessageFrom, ErrNotFound, ErrClientId};
use tracing::{info, warn, error, debug};

pub struct Client {
    host: Host,
    controller_recv:Receiver<NetCommand>,
    servers: HashMap<ServerTypes, NodeId>
}
impl Client {
    pub fn new(id:NodeId,
               type_host:NodeType,
               connected_drone_ids: Vec<NodeId>,
               controller_send:Sender<HostEvent>,
               controller_recv:Receiver<NetCommand>,
               packet_recv:Receiver<Packet>,
               packet_send:HashMap<NodeId, Sender<Packet>>) ->Self{
        //configuro un subscriber all'avvio
        // tracing_subscriber::fmt()
        //     // legge livello da RUST_LOG (es. "debug" o "info")
        //     .with_env_filter(EnvFilter::from_default_env())
        //     .try_init()
        //     .expect("failed to init tracing subscriber");
        // Uso i livelli appropriati:
        //
        // trace! per dettagli finissimi,
        //
        // debug! per flusso di dati (es. invio/ricezione frammenti),
        //
        // info! per eventi “user‑visible” (es. flood iniziato/completato),
        //
        // warn! e error! per anomalie o fallimenti.

        //Assicura che il client sia conneso a uno o due droni
        assert!(
            connected_drone_ids.len() >= 1 && connected_drone_ids.len() <= 2,
            "Client must be connected to at least one and at most two drones!"
        );

        let host = Host::new(id, type_host, controller_send,  packet_recv, packet_send);
        Self{
            host,controller_recv, servers: HashMap::new()
        }
    }

    pub fn run(&mut self) {
        let cleanup_tick   = tick(Duration::from_secs(15));
        self.host.initiate_flood();
        loop {
            crossbeam_channel::select_biased! {
                // 1) always prioritize controller command
                recv(self.controller_recv) -> cmd => {
                    if let Ok(command) = cmd {
                        self.handle_command(command);
                    }
                }

                // 2) and then arriving packets
                recv(self.host.packet_recv) -> pkt => {
                    if let Ok(packet) = pkt {
                        if let Err(e) = self.handle_packet(packet) {
                            error!("Error processing packet: {}", e);
                        }
                    }
                }

                // 3) timer for cleaning
                recv(cleanup_tick) -> _ => {
                    // ... dove vuoi vedere la rete stampata, ad esempio alla fine di process_flood_response …
                    // println!("=== Discovered Network (outgoing edges) ===");
                    // for (&node_id, &(_, ref edges)) in &self.host.network.nodes {
                    //     let neighs: Vec<_> = edges.iter().map(|e| e.to).collect();
                    //     println!("Node {} -> {:?}", node_id, neighs);
                    // }
                    //
                    // println!("=== Discovered Network (incoming edges) ===");
                    // for (&node_id, _) in &self.host.network.nodes {
                    //     // trova tutti i nodi che puntano a `node_id`
                    //     let incoming: Vec<_> = self
                    //         .host
                    //         .network
                    //         .nodes
                    //         .iter()
                    //         .filter_map(|(&other, &(_, ref edges))| {
                    //             if edges.iter().any(|e| e.to == node_id) {
                    //                 Some(other)
                    //             } else {
                    //                 None
                    //             }
                    //         })
                    //         .collect();
                    //     println!("Node {} -> {:?}", node_id, incoming);
                    // }

                    // println!("=== All links FROM CLIENT VIEW ===");
                    // for (&from, &(_, ref edges)) in &self.host.network.nodes {
                    //     for edge in edges {
                    //         println!("{} -> {}", from, edge.to);
                    //     }
                    // }
                    //
                    // println!("=== find_best_route TEST ===");
                    // if let Some(route) = self.host.network.find_best_route(106, 207){
                    //     println!("✅ Route: {:?}", route);
                    // }else{
                    //     println!("❌ No route found!");
                    // }
                    // each 60s clean session
                    let timeout  = Duration::from_secs(300);
                    self.host.cleanup_expired_sessions(timeout);

                }

                // 4) otherwise wait a bit to avoid busy‑spin
                default(Duration::from_millis(10)) => {}
            }
        }

    }
    /// Se non ho ancora scoperto *tutti* i server,
    /// mando la discovery e aspetto che `self.servers` contenga
    /// tutti gli ID di `self.host.network.servers`.
    fn ensure_all_servers_discovered(&mut self) {
        let expected_count = self.host.network.servers.len();
        if self.servers.len() != expected_count {
            tracing::info!(
                "Discovering server types from {} nodes...",
                expected_count
            );
            self.request_server_type();

            // rimani in polling finché non hai ricevuto tutte le risposte
            while self.servers.len() != expected_count {
                thread::sleep(Duration::from_millis(100));
            }

            tracing::info!("Discovered all {} servers", expected_count);
        }
    }


    fn handle_command(&mut self, net_comm: NetCommand) {
        self.ensure_all_servers_discovered();

        match net_comm {
            // Command network serializable
            NetCommand::Net(cmd) => {
                match cmd {
                    ClientCommand::FilesList => {
                        tracing::info!("Preparing FilesList request");
                        if let Some(&server_id) = self.servers.get(&Content) {
                            match serde_json::to_vec(&ClientCommand::FilesList) {
                                Ok(content) => {
                                    let message = Message {
                                        session_id: generate_unique_session_id(),
                                        content,
                                    };
                                    if let Err(err) = self.host.build_msg_fragment(server_id, message) {
                                        error!("Failed to send FilesList to server {}: {}", server_id, err);
                                    } else {
                                        debug!("FilesList sent to server {}", server_id);
                                    }
                                }
                                Err(e) => {
                                    error!("Serialization of FilesList failed: {}", e);
                                }
                            }
                        } else {
                            warn!("No content server available for FilesList");
                        }
                    }

                    ClientCommand::File(id) => {
                        info!("Preparing File({}) request", id);
                        if let Some(&server_id) = self.servers.get(&Content) {
                            match serde_json::to_vec(&ClientCommand::File(id)) {
                                Ok(content) => {
                                    let message = Message {
                                        session_id: generate_unique_session_id(),
                                        content,
                                    };
                                    if let Err(err) = self.host.build_msg_fragment(server_id, message) {
                                        error!("Failed to send File({}) to server {}: {}", id, server_id, err);
                                    } else {
                                        debug!("File({}) request sent to server {}", id, server_id);
                                    }
                                }
                                Err(e) => {
                                    error!("Serialization of File({}) failed: {}", id, e);
                                }
                            }
                        } else {
                            warn!("No content server available for File({})", id);
                        }
                    }

                    ClientCommand::MediaList => {
                        info!("Preparing MediaList request");
                        if let Some(&server_id) = self.servers.get(&Content) {
                            match serde_json::to_vec(&ClientCommand::MediaList) {
                                Ok(content) => {
                                    let message = Message {
                                        session_id: generate_unique_session_id(),
                                        content,
                                    };
                                    if let Err(err) = self.host.build_msg_fragment(server_id, message) {
                                        error!("Failed to send MediaList to server {}: {}", server_id, err);
                                    } else {
                                        debug!("MediaList sent to server {}", server_id);
                                    }
                                }
                                Err(e) => {
                                    error!("Serialization of MediaList failed: {}", e);
                                }
                            }
                        } else {
                            warn!("No content server available for MediaList");
                        }
                    }

                    ClientCommand::Media(id) => {
                        info!("Preparing Media({}) request", id);
                        if let Some(&server_id) = self.servers.get(&Content) {
                            match serde_json::to_vec(&ClientCommand::Media(id)) {
                                Ok(content) => {
                                    let message = Message {
                                        session_id: generate_unique_session_id(),
                                        content,
                                    };
                                    if let Err(err) = self.host.build_msg_fragment(server_id, message) {
                                        error!("Failed to send Media({}) to server {}: {}", id, server_id, err);
                                    } else {
                                        debug!("Media({}) request sent to server {}", id, server_id);
                                    }
                                }
                                Err(e) => {
                                    error!("Serialization of Media({}) failed: {}", id, e);
                                }
                            }
                        } else {
                            warn!("No content server available for Media({})", id);
                        }
                    }

                    ClientCommand::RegistrationToChat => {
                        info!("Preparing RegistrationToChat request");
                        if let Some(&server_id) = self.servers.get(&Chat) {
                            match serde_json::to_vec(&ClientCommand::RegistrationToChat) {
                                Ok(content) => {
                                    let message = Message {
                                        session_id: generate_unique_session_id(),
                                        content,
                                    };
                                    if let Err(err) = self.host.build_msg_fragment(server_id, message) {
                                        error!("Failed to send RegistrationToChat to server {}: {}", server_id, err);
                                    } else {
                                        debug!("RegistrationToChat sent to server {}", server_id);
                                    }
                                }
                                Err(e) => {
                                    error!("Serialization of RegistrationToChat failed: {}", e);
                                }
                            }
                        } else {
                            warn!("No chat server available for RegistrationToChat");
                        }
                    }

                    ClientCommand::ClientList => {
                        info!("Preparing ClientList request");
                        if let Some(&server_id) = self.servers.get(&Chat) {
                            match serde_json::to_vec(&ClientCommand::ClientList) {
                                Ok(content) => {
                                    let message = Message {
                                        session_id: generate_unique_session_id(),
                                        content,
                                    };
                                    if let Err(err) = self.host.build_msg_fragment(server_id, message) {
                                        error!("Failed to send ClientList to server {}: {}", server_id, err);
                                    } else {
                                        debug!("ClientList sent to server {}", server_id);
                                    }
                                }
                                Err(e) => {
                                    error!("Serialization of ClientList failed: {}", e);
                                }
                            }
                        } else {
                            warn!("No chat server available for ClientList");
                        }
                    }

                    ClientCommand::MessageFor(client_id, msg) => {
                        tracing::info!("Preparing MessageFor({}, …) request", client_id);
                        if let Some(&server_id) = self.servers.get(&Chat) {
                            match serde_json::to_vec(&ClientCommand::MessageFor(client_id, msg)) {
                                Ok(content) => {
                                    let message = Message {
                                        session_id: generate_unique_session_id(),
                                        content,
                                    };
                                    if let Err(err) = self.host.build_msg_fragment(server_id, message) {
                                        tracing::error!("Failed to send MessageFor({}, …) to server {}: {}", client_id, server_id, err);
                                    } else {
                                        tracing::debug!("MessageFor({}, …) sent to server {}", client_id, server_id);
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Serialization of MessageFor failed: {}", e);
                                }
                            }
                        } else {
                            tracing::warn!("No chat server available for MessageFor({})", client_id);
                        }
                    }
                    _ => {
                        tracing::debug!("Received NetCommand variant with no action: {:?}", cmd);
                    }
                }
            }

            // Shortcut inviato dal controller interno
            NetCommand::ControllerShortcut(packet) => {
                tracing::info!("Handling ControllerShortcut packet");
                if let Err(e) = self.handle_packet(packet) {
                    tracing::error!("Error processing ControllerShortcut packet: {}", e);
                }
                // Una volta shortcuttato, rifaccio il flood
                self.host.initiate_flood();
            }
        }
    }

    fn handle_packet(&mut self, packet: Packet) -> Result<(), String> {
        match packet.pack_type.clone() {
            PacketType::FloodResponse(response) => {
                self.host.process_flood_response(response.clone());
                Ok(())
            }
            PacketType::MsgFragment(fragment) => {
                if let Some(message) = self.handle_msg_fragment(packet.clone(), fragment) {

                    let send_reconstructed =
                        |evt: ServerToClient| {
                            self.host
                                .controller_send
                                .send(HostEvent::PacketReconstructed(evt))
                                .map_err(|e| e.to_string())
                        };
                    match serde_json::from_slice::<ServerToClient>(&message) {
                        Ok(cmd) => {
                            // Ho un comando high-level, lo dispatcho
                            match cmd {
                                ServerType(st) => {
                                    match packet.routing_header.source(){
                                        Some(src) => {
                                            match st {
                                                Content => {
                                                    self.servers.insert(Content, src);
                                                    tracing::info!("Content server registered: {:?}", src);
                                                }
                                                Chat => {
                                                    self.servers.insert(Chat, src);
                                                    tracing::info!("Chat server registered: {:?}", src);
                                                }
                                            }
                                        }
                                        None => tracing::error!("ServerType response must have source")
                                    }
                                }
                                ClientList(clients) => {
                                    tracing::info!("Received Client List: {:?}", clients);
                                    send_reconstructed(ClientList(clients))?;
                                }

                                MessageFrom(from, msg) => {
                                    tracing::info!("Message from {}: {:?}", from, msg.content);
                                    send_reconstructed(MessageFrom(from, msg))?;
                                }
                                FilesList(files) => {
                                    tracing::info!("Files available: {:?}", files);
                                    send_reconstructed(FilesList(files))?;
                                }
                                File(file_id, data) => {
                                    tracing::info!("Received file {} of {} bytes",file_id,data.len());
                                    send_reconstructed(File(file_id, data))?;
                                }
                                MediaList(media_list) => {
                                    tracing::info!("Media available: {:?}", media_list);
                                    send_reconstructed(MediaList(media_list))?;
                                }
                                Media(media_id, media) => {
                                    tracing::info!("Received media {} of {} bytes",media_id, media.data.len());
                                    send_reconstructed(Media(media_id, media))?;
                                }
                                ErrClientId => {
                                    tracing::error!("Server responded: ErrClientId");
                                    send_reconstructed(ErrClientId)?;
                                }
                                ErrNotFound => {
                                    tracing::error!("Server responded: ErrNotFound");
                                    send_reconstructed(ErrNotFound)?;
                                }
                            }
                            Ok(())
                        }
                        Err(e) => {
                            // Not a valid JSON of ServerToClient: read and put
                            tracing::error!("Invalid ServerToClient JSON: {}", e);
                            Ok(())
                        }
                    }
                } else {
                    // Message not yet assembled: wait for other fragments
                    tracing::debug!(
                    "Fragment received but message not yet fully reassembled (session {})",
                    packet.session_id
                );
                    Ok(())
                }
            }

            PacketType::FloodRequest(flood_request) => {
                self.host.process_flood_request(flood_request, packet);
                Ok(())
            }

            PacketType::Ack(ack) => {
                self.host.handle_ack(ack, packet.session_id);
                Ok(())
            }

            PacketType::Nack(nack) => {
                if let Some(src) = packet.routing_header.source() {
                    self.host.handle_nack(nack, src, packet.session_id);
                }else { tracing::warn!("Packet must have source!"); }
                Ok(())
            }
        }
    }
    fn request_server_type(&mut self) {
        // for each known server, i send ServerType query
        for server_id in self.host.network.servers.clone() {
            tracing::info!("Requesting server type from node {}", server_id);

            match serde_json::to_vec(&ClientCommand::ServerType) {
                Ok(content) => {
                    let message = Message {
                        session_id: generate_unique_session_id(),
                        content,
                    };
                    if let Err(err) = self.host.build_msg_fragment(server_id, message) {
                        tracing::error!("Failed to send ServerType request to {}: {}",server_id,err);
                    } else {
                        tracing::debug!("Sent ServerType request to {}", server_id);
                    }
                }
                Err(e) => {
                    tracing::error!("Serialization of ServerType request failed: {}", e);
                }
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
            tracing::debug!(
            host = self.host.id,
            session = session_id,
            pending = session.pending,
            "Received fragment, pending--"
        );
        }

        // invia ACK
        let ack = self.host.build_packet_ack(packet.clone(), Some(fragment.fragment_index));
        let _ = self.host.send_packet(ack, None);

        // se finito, reassembly e rimozione
        if session.pending == 0 {
            tracing::info!(
            host = self.host.id,
            session = session_id,
            "All fragments received"
        );
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

}

// fn elaborate(message: Vec<u8>) -> Result<String, String> {
//     match message.first().copied() {
//         Some(b'c') => rev_message(message).map_err(|e| e.to_string()),
//         Some(b'm') => {
//             let image = revert_image(message)?; // Decodifica l'immagine
//             image.save("output.png").map_err(|e| e.to_string())?; // Salva su file
//             Ok("Image processed and saved as output.png".into())
//         },
//         Some(b'f') => revert_file(message).map(|_| "File saved".into())
//             .map_err(|e| format!("File error: {}", e)),
//         _ => Err("Invalid message type".into()),
//     }
// }
//
// // Aggiunto tipo di ritorno esplicito e gestione errori
// fn rev_message(bytes: Vec<u8>) -> Result<String, std::string::FromUtf8Error> {
//     String::from_utf8(bytes)
// }
// fn revert_file(bytes: Vec<u8>) -> io::Result<()> {
//     // Write the bytes directly to a file
//     fs::write("output.txt", bytes)?; //Il file viene salvato con il nome "output.txt" nella directory di esecuzione del programma.
//     Ok(())
// }
//
// fn revert_image(data: Vec<u8>) -> Result<DynamicImage, String> {
//     if data.len() < 9 {
//         return Err("Invalid image header".into());
//     }
//
//     let width = u32::from_be_bytes(data[1..5].try_into().unwrap());
//     let height = u32::from_be_bytes(data[5..9].try_into().unwrap());
//     let image_data = &data[9..];
//
//     if image_data.len() != (width * height * 4) as usize {
//         return Err(format!(
//             "Dati immagine non validi: attesi {}, ottenuti {}",
//             width * height * 4,
//             image_data.len()
//         ));
//     }
//
//     ImageBuffer::from_raw(width, height, image_data.to_vec())
//         .map(DynamicImage::ImageRgba8)
//         .ok_or_else(|| "Failed to create image buffer".into())
// }