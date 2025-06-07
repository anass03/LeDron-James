use crossbeam_channel::{select_biased, tick, unbounded, Receiver, Sender};
use wg_2024::network::{NodeId};
use wg_2024::packet::{Packet, PacketType, NodeType, Fragment};
use std::collections::{HashMap};
use std::{thread};
use std::fmt::format;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use host::{HostCommand, Message, Host, Functionality, generate_unique_session_id, ServerTypes, RecSessionData, ServerToClient, HostEvent, FileId, MediaId, ClientId, ClientNetCommand};
use host::ServerTypes::{Content,Chat};
use host::ServerToClient::{ClientList, ServerType, FilesList, File, Media, MediaList, MessageFrom, ErrNotFound, ErrClientId, FileFrom, MediaFrom, WrongServer};

pub struct Client {
    host: Host,
    controller_recv:Receiver<ClientCommand>,
    servers: HashMap<ServerTypes, Vec<NodeId>>,
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
            host,controller_recv, servers: HashMap::new(),
        }
    }

    pub fn run(&mut self) {
        let cleanup_tick = tick(Duration::from_secs(15));
        // “never‐timer” che non invierà mai un Instant
        let (_never_tx, never_timer): (Sender<Instant>, Receiver<Instant>) = unbounded();

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
                // 3) debounce‐timer (fire solo se flood_scheduled è true)
                // recv(self
                // .host
                // .flood_timer
                // .as_ref()
                // .cloned()
                // .unwrap_or_else(|| never_timer.clone())) -> _ => {
                //     if self.host.flood_scheduled{
                //          self.host.flood_scheduled = false;
                //         self.host.flood_timer = None;
                //         self.host.send_logging("Debounced flood triggered".to_string());
                //         self.host.initiate_flood();
                //     }
                // },

                // 4) cleanup periodico
                recv(cleanup_tick) -> _ => {
                    let timeout = Duration::from_secs(300);
                    // self.host.cleanup_expired_sessions(timeout);
                },
            }
        }
    }

    /// Se non ho ancora scoperto *tutti* i server,
    /// mando la discovery e aspetto che `self.servers` contenga
    /// tutti gli ID di `self.host.network.servers`.
    fn ensure_all_servers_discovered(&mut self) -> Result<(), String> {
        let expected = self.host.network.servers.len();
        let mut servers_discovered = 0;

        if servers_discovered != expected {
            self.host.send_logging(format!(
                "Discovering server types from {} nodes, host={}",
                expected, self.host.id
            ));
            self.request_server_type();

            while servers_discovered < expected {
                crossbeam_channel::select! {
                recv(self.host.packet_recv) -> pkt => {
                    let packet = pkt.map_err(|e| e.to_string())?;
                    // Processing del packet: estraggo il payload, ricompongo il messaggio e provo a
                    // interpretarlo come ServerToClient.
                    if let PacketType::MsgFragment(fragment) = packet.pack_type.clone() {
                        if let Some(raw_bytes) = self.handle_msg_fragment(packet.clone(), fragment) {
                            // qui provo a parsare esattamente un JSON ServerToClient
                            if let Ok(cmd) = serde_json::from_slice::<ServerToClient>(&raw_bytes) {
                                match cmd {
                                    ServerType(st) => {
                                        // Se è un ServerType valido, registro l’ID del sender
                                        if let Some(src) = packet.routing_header.source() {
                                            let entry =
                                                self.servers.entry(st.clone()).or_insert_with(Vec::new);
                                            if !entry.contains(&src) {
                                                entry.push(src);
                                                self.host.send_logging(format!(
                                                    "Registered {:?} server: {:?}",
                                                    st, src
                                                ));
                                            }
                                            // Incremento solo qui: ho effettivamente ricevuto un ServerType
                                            servers_discovered += 1;
                                        }
                                    }
                                    _ => {
                                        // È una qualche altra risposta (File, Media, ecc.) che
                                        // non ci interessa per il conteggio dei server.
                                    }
                                }
                            }
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




    fn handle_command(&mut self, client_command: ClientCommand) {
        if let Err(e) = self.ensure_all_servers_discovered() {
            self.host.send_logging(format!("Error during discovery: {}", e));
            return;
        }

        match client_command {
            ClientCommand::Net(net_command) => {
                match net_command {
                    ClientNetCommand::ServerType =>{
                        self.host.send_logging("ServerType request received".to_string());
                        self.host.send_logging(format!("SERVERS ID {:?}", self.host.network.servers));
                        self.host.send_logging(format!("✅ {:?}", self.host.network.nodes));
                        let servers_snapshot = self.servers.clone();
                        self.host.send_logging(format!(
                            "Servers discovered: {:?}",
                            servers_snapshot.clone()
                        ));
                        if let Err(err) = self.host.controller_send.send(HostEvent::ServersDiscovered(servers_snapshot)) {
                            self.host.send_logging(format!(
                                "Error sending ServersDiscovered to controller: {}",
                                err
                            ));
                        }

                    }
                    ClientNetCommand::FilesList(server_id) => {
                        self.host.send_logging("Preparing FilesList request".to_string());
                        if let Some(&ref content_serv) = self.servers.get(&Content) {
                            if content_serv.contains(&server_id) {
                                match serde_json::to_vec(&ClientNetCommand::FilesList(server_id)) {
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
                            }else { self.host.send_logging(format!("Err, ServerID: {} , is a chat Server", server_id)); }
                        } else {
                            self.host.send_logging("No content server available for FilesList".to_string());
                        }
                    }

                    ClientNetCommand::File(server_id,id) => {
                        self.host.send_logging(format!("Preparing File({}) request", id));
                        if let Some(&ref content_serv) = self.servers.get(&Content) {
                            if content_serv.contains(&server_id) {
                                match serde_json::to_vec(&ClientNetCommand::File(server_id,id)) {
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
                            }else {
                                // target_id non è nella lista dei Content-server
                                self.host.send_logging(format!(
                                    "Node {} is NOT a valid CONTENT-SERVER",
                                    server_id
                                ));
                            }
                        } else {
                            self.host.send_logging(format!("No content server available for File({})", id));
                        }
                    }

                    ClientNetCommand::MediaList(server_id) => {
                        self.host.send_logging("Preparing MediaList request".to_string());
                        if let Some(&ref content_serv) = self.servers.get(&Content) {
                            if content_serv.contains(&server_id) {
                                match serde_json::to_vec(&ClientNetCommand::MediaList(server_id)) {
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
                            }else {
                                // target_id non è nella lista dei Content-server
                                self.host.send_logging(format!(
                                    "Node {} is NOT a valid CONTENT-SERVER",
                                    server_id
                                ));
                            }
                        } else {
                            self.host.send_logging("No content server available for MediaList".to_string());
                        }
                    }

                    ClientNetCommand::Media(server_id,id) => {
                        self.host.send_logging(format!("Preparing Media({}) request", id));
                        if let Some(&ref content_serv) = self.servers.get(&Content) {
                            if content_serv.contains(&server_id) {
                                match serde_json::to_vec(&ClientNetCommand::Media(server_id,id)) {
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
                            }else {
                                // target_id non è nella lista dei Content-server
                                self.host.send_logging(format!(
                                    "Node {} is NOT a valid CONTENT-SERVER",
                                    server_id
                                ));
                            }
                        } else {
                            self.host.send_logging(format!("No content server available for Media({})", id));
                        }
                    }

                    ClientNetCommand::RegistrationToChat(server_id) => {
                        self.host.send_logging("Preparing RegistrationToChat request".to_string());
                        if let Some(&ref chat_serv) = self.servers.get(&Chat) {
                            if chat_serv.contains(&server_id) {
                                match serde_json::to_vec(&ClientNetCommand::RegistrationToChat(server_id)) {
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
                            }else {
                                // target_id non è nella lista dei Content-server
                                self.host.send_logging(format!(
                                    "Node {} is NOT a valid CHAT-SERVER",
                                    server_id
                                ));
                            }
                        } else {
                            self.host.send_logging(format!("No chat server available for RegistrationToChat host={}", self.host.id));
                        }
                    }

                    ClientNetCommand::ClientList(server_id) => {
                        self.host.send_logging("Preparing ClientList request".to_string());
                        if let Some(&ref chat_serv) = self.servers.get(&Chat) {
                            if chat_serv.contains(&server_id) {
                                match serde_json::to_vec(&ClientNetCommand::ClientList(server_id)) {
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
                            }else {
                                // target_id non è nella lista dei Content-server
                                self.host.send_logging(format!(
                                    "Node {} is NOT a valid CHAT-SERVER",
                                    server_id
                                ));
                            }
                        } else {
                            self.host.send_logging("No chat server available for ClientList".to_string());
                        }
                    }

                    ClientNetCommand::MessageFor(client_id,server_id, msg) => {
                        self.host.send_logging(format!("Preparing MessageFor({}, …) request", client_id));
                        if let Some(&ref chat_serv) = self.servers.get(&Chat) {
                            if chat_serv.contains(&server_id) {
                                match serde_json::to_vec(&ClientNetCommand::MessageFor(client_id,server_id, String::from(msg))) {
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
                            }else {
                                // target_id non è nella lista dei Content-server
                                self.host.send_logging(format!(
                                    "Node {} is NOT a valid CHAT-SERVER",
                                    server_id
                                ));
                            }
                        } else {
                            self.host.send_logging(format!("No chat server available for MessageFor({})", client_id));
                        }
                    }
                    ClientNetCommand::FileFor(client_id,server_id, file_id,ext, file) => {
                        self.host.send_logging(format!("Preparing FileFor({}, …) request", client_id));
                        if let Some(&ref chat_serv) = self.servers.get(&Chat) {
                            if chat_serv.contains(&server_id) {
                                match serde_json::to_vec(&ClientNetCommand::FileFor(client_id,server_id,file_id, ext, file)) {
                                    Ok(content) => {
                                        let message = Message {
                                            session_id: generate_unique_session_id(),
                                            content,
                                        };
                                        if let Err(err) = self.host.build_msg_fragment(server_id, message) {
                                            self.host.send_logging(format!("Failed to send FileFor({}, …) to server {}: {}", client_id, server_id, err));
                                        } else {
                                            self.host.send_logging(format!("FileFor({}, …) sent to server {}", client_id, server_id));
                                        }
                                    }
                                    Err(e) => {
                                        self.host.send_logging(format!("Serialization of FileFor failed: {}", e));
                                    }
                                }
                            }else {
                                // target_id non è nella lista dei Content-server
                                self.host.send_logging(format!(
                                    "Node {} is NOT a valid CHAT-SERVER",
                                    server_id
                                ));
                            }
                        } else {
                            self.host.send_logging(format!("No chat server available for FileFor({})", client_id));
                        }
                    }
                    ClientNetCommand::MediaFor(client_id,server_id,media_id, ext, image) => {
                        self.host.send_logging(format!("Preparing FileFor({}, …) request", client_id));
                        if let Some(&ref chat_serv) = self.servers.get(&Chat) {
                            if chat_serv.contains(&server_id) {
                                match serde_json::to_vec(&ClientNetCommand::MediaFor(client_id, server_id, media_id, ext, image)) {
                                    Ok(content) => {
                                        let message = Message {
                                            session_id: generate_unique_session_id(),
                                            content,
                                        };
                                        if let Err(err) = self.host.build_msg_fragment(server_id, message) {
                                            self.host.send_logging(format!("Failed to send MediaFor({}, …) to server {}: {}", client_id, server_id, err));
                                        } else {
                                            self.host.send_logging(format!("MediaFor({}, …) sent to server {}", client_id, server_id));
                                        }
                                    }
                                    Err(e) => {
                                        self.host.send_logging(format!("Serialization of MediaFor failed: {}", e));
                                    }
                                }
                            }else {
                                // target_id non è nella lista dei Content-server
                                self.host.send_logging(format!(
                                    "Node {} is NOT a valid CHAT-SERVER",
                                    server_id
                                ));
                            }
                        } else {
                            self.host.send_logging(format!("No chat server available for MediaFor({})", client_id));
                        }
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
                            self.host.packet_send.insert(id, sender);
                            self.host.send_logging("Successfully added sender!".to_string());
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
                            self.host.send_logging("Successfully removed sender!".to_string());
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
            PacketType::FloodResponse(response) => {
                self.host.process_flood_response(packet);
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
                                    if let Some(src) = packet.routing_header.source() {
                                        let entry = self.servers.entry(st.clone()).or_insert_with(Vec::new);
                                        if !entry.contains(&src) {
                                            entry.push(src);
                                            self.host.send_logging(format!("Registered {:?} server: {:?}", st, src));
                                        }
                                    } else {
                                        self.host.send_logging("ServerType response without source".to_string());
                                    }
                                }
                                ClientList(server_id,clients) => {
                                    self.host.send_logging(format!("Received Client List: {:?}", clients));
                                    send_reconstructed(ClientList(server_id,clients))?;
                                }

                                MessageFrom(server_id,from, msg) => {
                                    self.host.send_logging(format!("Message from {}: {:?}", from, msg.clone()));
                                    send_reconstructed(MessageFrom(server_id,from, msg))?;
                                }
                                FileFrom(server_id,from,file_id, ext, data) => {
                                    self.host.send_logging(format!("Received {}.{} from client {}", file_id, ext,from));
                                    send_reconstructed(FileFrom(server_id,from,file_id, ext, data))?;
                                }
                                MediaFrom(server_id,from, media_id, ext, data) => {
                                    self.host.send_logging(format!("Received {}.{} from client {}", media_id, ext,from));
                                    send_reconstructed(MediaFrom(server_id, from, media_id, ext, data))?;
                                }
                                FilesList(server_id, files) => {
                                    self.host.send_logging(format!("Files available: {:?}", files));
                                    send_reconstructed(FilesList(server_id, files))?;
                                }
                                File(server_id,file_id,ext, data) => {
                                    self.host.send_logging(format!("Received {}.{} of {} bytes",file_id,ext, data.len()));
                                    send_reconstructed(File(server_id,file_id,ext, data))?;
                                }
                                MediaList(server_id,media_list) => {
                                    self.host.send_logging(format!("Media available: {:?}", media_list));
                                    send_reconstructed(MediaList(server_id,media_list))?;
                                }
                                Media(server_id,media_id,ext, media) => {
                                    self.host.send_logging(format!("Received {}.{} of {} bytes",media_id,ext, media.data.len()));
                                    send_reconstructed(Media(server_id,media_id,ext, media))?;
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

            match serde_json::to_vec(&ClientNetCommand::ServerType) {
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

}

pub enum ClientCommand {
    /// Wraps a network-level command
    Net(ClientNetCommand),
    /// A local internal management command
    Internal(HostCommand),
    /// Packet shortcut delivered by the SimulationController
    ControllerShortcut(Packet),
}
