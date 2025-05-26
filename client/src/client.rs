use crossbeam_channel::{tick, Receiver, Sender};
use wg_2024::network::{NodeId};
use wg_2024::packet::{Packet, PacketType, NodeType, Fragment};
use std::collections::{HashMap};
use std::{thread};
use std::time::{Duration, Instant};
use host::{ClientCommand, Message, Host, Functionality, generate_unique_session_id, ServerTypes, RecSessionData, ServerToClient, NetCommand, HostEvent, SendingCodes};
use host::ServerTypes::{Content,Chat};
use host::ServerToClient::{ClientList, ServerType, FilesList, File, Media,MediaList, MessageFrom, ErrNotFound, ErrClientId};

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
               packet_send:HashMap<NodeId, Sender<Packet>>,
               log_sender: Sender<String>) ->Self{

        let host = Host::new(id, type_host, controller_send,  packet_recv, packet_send, log_sender);
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
                            self.host.send_logging(format!("Error processing packet: {}", e));
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

                    // self.host.send_logging(format!("=== All links FROM CLIENT VIEW host={} ===",self.host.id));
                    // for (&from, &(_, ref edges)) in &self.host.network.nodes {
                    //     for edge in edges {
                    //         self.host.send_logging(format!("{} - {}", from, edge.to));
                    //     }
                    // }
                    //
                    // self.host.send_logging("=== find_best_route 7-9 TEST ===".to_string());
                    // if let Some(route) = self.host.network.find_best_route(7, 9){
                    //     self.host.send_logging(format!("✅ Route: {:?} host={}", route, self.host.id));
                    // }else{
                    //     self.host.send_logging(format!("❌ No route found! host={}", self.host.id));
                    // }
                    // thread::sleep(Duration::from_secs(2));
                    // self.host.send_logging("=== find_best_route 6-8 TEST ===".to_string());
                    // if let Some(route) = self.host.network.find_best_route(6, 8){
                    //     self.host.send_logging(format!("✅ Route: {:?} host={}", route, self.host.id));
                    // }else{
                    //     self.host.send_logging(format!("❌ No route found! host={}", self.host.id));
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
    fn ensure_all_servers_discovered(&mut self) -> Result<(), String> {
        let expected = self.host.network.servers.len();
        if self.servers.len() != expected {
            self.host.send_logging(format!(
                "Discovering server types from {} nodes, host={}",
                expected, self.host.id
            ));
            // invio subito la prima ondata di request
            self.request_server_type();

            // resto in discovery finché non ho tutte le risposte
            while self.servers.len() != expected {
                crossbeam_channel::select! {
                // 1) arrivo di un FloodResponse o di un ServerType: lo processo
                recv(self.host.packet_recv) -> pkt => {
                    let packet = pkt.map_err(|e| e.to_string())?;
                    // la chiamata interna aggiornerà `self.servers` quando riceve un ServerType
                    self.handle_packet(packet)?;
                }

                // 2) se arriva un comando nuovo mentre sto scoprendo
                recv(self.controller_recv) -> _cmd => {
                    // ad esempio: loggo e poi continuo a scoprire
                    self.host.send_logging("Got a command while discovering".to_string());
                }

                // 3) dopo un po’ riprovo a inviare le ServerType request
                default(Duration::from_millis(200)) => {
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



    fn handle_command(&mut self, net_comm: NetCommand) {
        if let Err(e) = self.ensure_all_servers_discovered() {
            self.host.send_logging(format!("Error during discovery: {}", e));
            return;
        }

        match net_comm {
            // Command network serializable
            NetCommand::Net(cmd) => {
                match cmd {
                    ClientCommand::FilesList => {
                        self.host.send_logging("Preparing FilesList request".to_string());
                        if let Some(&server_id) = self.servers.get(&Content) {
                            match serde_json::to_vec(&ClientCommand::FilesList) {
                                Ok(content) => {
                                    let message = Message {
                                        session_id: generate_unique_session_id(),
                                        content,
                                    };
                                    if let Err(err) = self.host.build_msg_fragment(server_id, message) {

                                        self.host.send_logging(format!("Failed to send FilesList to server {}: {}", server_id, err));
                                    } else {
                                        self.host.send_logging(format!("FilesList sent to server {}", server_id));
                                    }
                                }
                                Err(e) => {
                                    self.host.send_logging(format!("Serialization of FilesList failed: {}", e));
                                }
                            }
                        } else {
                            self.host.send_logging("No content server available for FilesList".to_string());
                        }
                    }

                    ClientCommand::File(id) => {
                        self.host.send_logging(format!("Preparing File({}) request", id));
                        if let Some(&server_id) = self.servers.get(&Content) {
                            match serde_json::to_vec(&ClientCommand::File(id)) {
                                Ok(content) => {
                                    let message = Message {
                                        session_id: generate_unique_session_id(),
                                        content,
                                    };
                                    if let Err(err) = self.host.build_msg_fragment(server_id, message) {
                                        self.host.send_logging(format!("Failed to send File({}) to server {}: {}", id, server_id, err));
                                    } else {
                                        self.host.send_logging(format!("File({}) request sent to server {}", id, server_id));
                                    }
                                }
                                Err(e) => {
                                    self.host.send_logging(format!("Serialization of File({}) failed: {}", id, e));
                                }
                            }
                        } else {
                            self.host.send_logging(format!("No content server available for File({})", id));
                        }
                    }

                    ClientCommand::MediaList => {
                        self.host.send_logging("Preparing MediaList request".to_string());
                        if let Some(&server_id) = self.servers.get(&Content) {
                            match serde_json::to_vec(&ClientCommand::MediaList) {
                                Ok(content) => {
                                    let message = Message {
                                        session_id: generate_unique_session_id(),
                                        content,
                                    };
                                    if let Err(err) = self.host.build_msg_fragment(server_id, message) {
                                        self.host.send_logging(format!("Failed to send MediaList to server {}: {}", server_id, err));
                                    } else {
                                        self.host.send_logging(format!("MediaList sent to server {}", server_id));
                                    }
                                }
                                Err(e) => {
                                    self.host.send_logging(format!("Serialization of MediaList failed: {}", e));
                                }
                            }
                        } else {
                            self.host.send_logging("No content server available for MediaList".to_string());
                        }
                    }

                    ClientCommand::Media(id) => {
                        self.host.send_logging(format!("Preparing Media({}) request", id));
                        if let Some(&server_id) = self.servers.get(&Content) {
                            match serde_json::to_vec(&ClientCommand::Media(id)) {
                                Ok(content) => {
                                    let message = Message {
                                        session_id: generate_unique_session_id(),
                                        content,
                                    };
                                    if let Err(err) = self.host.build_msg_fragment(server_id, message) {
                                        self.host.send_logging(format!("Failed to send Media({}) to server {}: {}", id, server_id, err));
                                    } else {
                                        self.host.send_logging(format!("Media({}) request sent to server {}", id, server_id));
                                    }
                                }
                                Err(e) => {
                                    self.host.send_logging(format!("Serialization of Media({}) failed: {}", id, e));
                                }
                            }
                        } else {
                            self.host.send_logging(format!("No content server available for Media({})", id));
                        }
                    }

                    ClientCommand::RegistrationToChat => {
                        self.host.send_logging("Preparing RegistrationToChat request".to_string());
                        if let Some(&server_id) = self.servers.get(&Chat) {
                        match serde_json::to_vec(&ClientCommand::RegistrationToChat) {
                            Ok(content) => {
                                let message = Message {
                                    session_id: generate_unique_session_id(),
                                    content,
                                };
                                if let Err(err) = self.host.build_msg_fragment(server_id, message) {
                                    self.host.send_logging(format!("Failed to send RegistrationToChat to server {}: {}", server_id, err));
                                } else {
                                    self.host.send_logging(format!("RegistrationToChat sent to server {}", server_id));
                                }
                            }
                            Err(e) => {
                                self.host.send_logging(format!("Serialization of RegistrationToChat failed: {}", e));
                            }
                        }
                        } else {
                            self.host.send_logging(format!("No chat server available for RegistrationToChat host={}", self.host.id));
                        }
                    }

                    ClientCommand::ClientList => {
                        self.host.send_logging("Preparing ClientList request".to_string());
                        if let Some(&server_id) = self.servers.get(&Chat) {
                            match serde_json::to_vec(&ClientCommand::ClientList) {
                                Ok(content) => {
                                    let message = Message {
                                        session_id: generate_unique_session_id(),
                                        content,
                                    };
                                    if let Err(err) = self.host.build_msg_fragment(server_id, message) {
                                        self.host.send_logging(format!("Failed to send ClientList to server {}: {}", server_id, err));
                                    } else {
                                        self.host.send_logging(format!("ClientList sent to server {}", server_id));
                                    }
                                }
                                Err(e) => {
                                    self.host.send_logging(format!("Serialization of ClientList failed: {}", e));
                                }
                            }
                        } else {
                            self.host.send_logging("No chat server available for ClientList".to_string());
                        }
                    }

                    ClientCommand::MessageFor(client_id, msg) => {
                        self.host.send_logging(format!("Preparing MessageFor({}, …) request", client_id));
                        if let Some(&server_id) = self.servers.get(&Chat) {
                            match serde_json::to_vec(&ClientCommand::MessageFor(client_id, String::from(msg))) {
                                Ok(content) => {
                                    let message = Message {
                                        session_id: generate_unique_session_id(),
                                        content,
                                    };
                                    if let Err(err) = self.host.build_msg_fragment(server_id, message) {
                                        self.host.send_logging(format!("Failed to send MessageFor({}, …) to server {}: {}", client_id, server_id, err));
                                    } else {
                                        self.host.send_logging(format!("MessageFor({}, …) sent to server {}", client_id, server_id));
                                    }
                                }
                                Err(e) => {
                                    self.host.send_logging(format!("Serialization of MessageFor failed: {}", e));
                                }
                            }
                        } else {
                            self.host.send_logging(format!("No chat server available for MessageFor({})", client_id));
                        }
                    }
                    _ => {
                        self.host.send_logging(format!("Received NetCommand variant with no action: {:?}", cmd));
                    }
                }
            }

            // Shortcut inviato dal controller interno
            NetCommand::ControllerShortcut(packet) => {
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
                                                    self.host.send_logging(format!("Content server registered: {:?}", src));
                                                }
                                                Chat => {
                                                    self.servers.insert(Chat, src);
                                                    self.host.send_logging(format!("Chat server registered: {:?}", src));
                                                }
                                            }
                                        }
                                        None => self.host.send_logging("ServerType response must have source".to_string())
                                    }
                                }
                                ClientList(clients) => {
                                    self.host.send_logging(format!("Received Client List: {:?}", clients));
                                    send_reconstructed(ClientList(clients))?;
                                }

                                MessageFrom(from, msg) => {
                                    self.host.send_logging(format!("Message from {}: {:?}", from, msg.clone()));
                                    send_reconstructed(MessageFrom(from, msg))?;
                                }
                                FilesList(files) => {
                                    self.host.send_logging(format!("Files available: {:?}", files));
                                    send_reconstructed(FilesList(files))?;
                                }
                                File(file_id, data) => {
                                    self.host.send_logging(format!("Received file {} of {} bytes",file_id,data.len()));
                                    send_reconstructed(File(file_id, data))?;
                                }
                                MediaList(media_list) => {
                                    self.host.send_logging(format!("Media available: {:?}", media_list));
                                    send_reconstructed(MediaList(media_list))?;
                                }
                                Media(media_id, media) => {
                                    self.host.send_logging(format!("Received media {} of {} bytes",media_id, media.data.len()));
                                    send_reconstructed(Media(media_id, media))?;
                                }
                                ErrClientId => {
                                    self.host.send_logging("Server responded: ErrClientId".to_string());
                                    send_reconstructed(ErrClientId)?;
                                }
                                ErrNotFound => {
                                    self.host.send_logging("Server responded: ErrNotFound".to_string());
                                    send_reconstructed(ErrNotFound)?;
                                }
                            }
                            Ok(())
                        }
                        Err(e) => {
                            // Not a valid JSON of ServerToClient: read and put
                            self.host.send_logging(format!("Invalid ServerToClient JSON: {}", e));
                            Ok(())
                        }
                    }
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
                self.host.handle_ack(ack, packet.session_id);
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

            match serde_json::to_vec(&ClientCommand::ServerType) {
                Ok(content) => {
                    let message = Message {
                        session_id: generate_unique_session_id(),
                        content,
                    };
                    if let Err(err) = self.host.build_msg_fragment(server_id, message) {
                        self.host.send_logging(format!("Failed to send ServerType request to {}: {}",server_id,err));
                    } else {
                        self.host.send_logging(format!("Sent ServerType request to {}", server_id));
                    }
                }
                Err(e) => {
                    self.host.send_logging(format!("Serialization of ServerType request failed: {}", e));
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

            self.host.send_logging(format!(
                "Received fragment, pending: {}, host: {}, session: {}",
                self.host.id,
                session_id,session.pending
            ));
        }

        // invia ACK
        let ack = self.host.build_packet_ack(packet.clone(), Some(fragment.fragment_index));
        let _ = self.host.send_packet(ack, None);

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

}
