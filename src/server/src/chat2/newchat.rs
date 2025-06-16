use crate::{GenServer, GenServerBehavior};
use crossbeam_channel::{Receiver, Sender};
use delegate::delegate;
use host::{ServerToClient, Functionality, ServerTypes, HostEvent, HostCommand, ClientNetCommand};
use std::collections::HashMap;
use host::Image as LeImage;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use wg_2024::network::NodeId;
use wg_2024::packet::{Ack, FloodRequest, Fragment, Nack, NodeType, Packet, PacketType};

/// ChatServer2 implements the server-side chat functionality:
/// - Registers clients
/// - Handles chat sessions and messages
/// - Supports flood discovery, packet processing, and fragment reassembly
pub struct ChatServer2 {
    info: GenServer,            // Underlying generic server infrastructure
    client_list: Vec<NodeId>,   // Registered clients list
    chats: Vec<Chat>            // Stored chat sessions with message histories
}

impl ChatServer2 {
    delegate! {
        to self.info.host {
            /// Flood discovery protocol delegation
            fn initiate_flood(&mut self);
            fn process_flood_request(&mut self, flood_request: FloodRequest, packet: Packet);
            fn process_flood_response(&mut self, packet: Packet);

            /// Routing reliability (ACK/NACK) delegation
            fn handle_ack(&mut self, ack: Ack, hops: &[NodeId], session_id: u64);
            fn handle_nack(&mut self, nack: Nack, nack_generator: NodeId, session_id: u64);
            fn send_logging(&self, message: String);
        }
        to self.info {
            /// Simulation controller and protocol logic delegation
            fn send_response(&mut self, response: ServerToClient, destination: NodeId);
            fn handle_sim_command(&mut self, cmd: HostCommand);
            fn servertype(&mut self) -> Result<ServerTypes, String>;
            fn handle_msg_fragment(&mut self, packet: Packet, fragment: Fragment) -> Option<Vec<u8>>;
        }
    }

    /// Create a new chat server instance
    pub fn new(
        id: NodeId,
        type_host: NodeType,
        connected_drone_ids: Vec<NodeId>,
        controller_send: Sender<HostEvent>,
        controller_recv: Receiver<HostCommand>,
        packet_recv: Receiver<Packet>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        log_sender: Sender<String>,
    ) -> Self {
        Self {
            info: GenServer::new(
                id,
                type_host,
                connected_drone_ids,
                controller_send,
                controller_recv,
                packet_recv,
                packet_send,
                ServerTypes::Chat,
                log_sender,
            ),
            client_list: vec![],
            chats: vec![],
        }
    }

    /// Main server loop: processes simulation commands and incoming network packets
    pub fn run(&mut self) {
        self.initiate_flood();
        self.send_logging("Starting Chat Server...".to_string());
        loop {
            crossbeam_channel::select_biased! {
                recv(self.info.controller_recv) -> cmd => {
                    if let Ok(command) = cmd {
                       self.handle_sim_command(command);
                    }
                }
                recv(self.info.host.packet_recv) -> pkt => {
                    if let Ok(packet) = pkt {
                         self.handling_packets(packet)
                    }
                }
                default(Duration::from_millis(3)) => {}
            }
        }
    }
    /// Handles high-level ClientNetCommand requests after fragment reassembly
    fn handle_command(&mut self, cmd: ClientNetCommand, source: NodeId) {
        match cmd {
            ClientNetCommand::ServerType => {
                if let Ok(server_type) = self.servertype() {
                    self.send_logging("Sending server type".to_string());
                    self.send_response(ServerToClient::ServerType(server_type), source);
                } else {
                    self.send_logging("Failed to send ServerToClient::ServerType".to_string());
                }
            }

            ClientNetCommand::RegistrationToChat(receiver_id) => {
                if receiver_id == self.info.host.id {
                    if !self.client_list.contains(&source) {
                        self.client_list.push(source);
                        self.send_logging(format!("Registered new client to chat: {}", source));
                    } else {
                        self.send_logging(format!("Duplicate registration from: {}", source));
                    }
                } else {
                    self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
                }
            }

            ClientNetCommand::ClientList(receiver_id) => {
                if receiver_id == self.info.host.id {
                    self.send_response(ServerToClient::ClientList(self.info.host.id, self.client_list.clone()), source);
                    self.send_logging("Sending client list".to_string());
                } else {
                    self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
                }
            }

            ClientNetCommand::MessageFor(destination, receiver_id, msg) => {
                if receiver_id == self.info.host.id {
                    if self.check_if_in_client_list(source) {
                        if self.check_if_in_client_list(destination) {
                            self.save_message(ClientNetCommand::MessageFor(destination, receiver_id, msg.clone()), source);
                            self.send_logging(format!("Sending message to {}: {}", destination, msg));
                            self.send_response(ServerToClient::MessageFrom(self.info.host.id, source, msg), destination);
                        } else {
                            self.send_logging(format!("Destination {} not registered", destination));
                            self.send_response(ServerToClient::ErrClientId(self.info.host.id), destination);
                        }
                    } else {
                        self.send_logging("Source client not registered".to_string());
                    }
                } else {
                    self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
                }
            }

            ClientNetCommand::FileFor(destination, receiver_id, file_id, extension, message) => {
                if receiver_id == self.info.host.id {
                    if self.check_if_in_client_list(source) {
                        if self.check_if_in_client_list(destination) {
                            self.save_message(ClientNetCommand::FileFor(destination, receiver_id, file_id, extension.clone(), message.clone()), source.clone());
                            self.send_logging(format!("Sending file to {}", destination));
                            self.send_response(ServerToClient::FileFrom(self.info.host.id, source, file_id, extension, message), destination);
                        } else {
                            self.send_logging(format!("Destination {} not registered", destination));
                            self.send_response(ServerToClient::ErrClientId(self.info.host.id), destination);
                        }
                    } else {
                        self.send_logging("Source client not registered".to_string());
                    }
                } else {
                    self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
                }
            }

            ClientNetCommand::MediaFor(destination, receiver_id, media_id, extension, message) => {
                if receiver_id == self.info.host.id {
                    if self.check_if_in_client_list(source) {
                        if self.check_if_in_client_list(destination) {
                            self.save_message(ClientNetCommand::MediaFor(destination, receiver_id, media_id, extension.clone(), message.clone()), source.clone());
                            self.send_logging(format!("Sending media to {}", destination));
                            self.send_response(ServerToClient::MediaFrom(self.info.host.id, source, media_id, extension, message), destination);
                        } else {
                            self.send_logging("Destination client not registered".to_string());
                        }
                    } else {
                        self.send_logging("Source client not registered".to_string());
                        self.send_response(ServerToClient::ErrClientId(self.info.host.id), destination);
                    }
                } else {
                    self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
                }
            }

            _ => {
                self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
            }
        }
    }

    /// Check if the client is currently registered
    fn check_if_in_client_list(&mut self, id: NodeId) -> bool {
        self.client_list.contains(&id)
    }

    /// Main packet handler: handles fragments, flood, ACKs, and deserialization
    fn handling_packets(&mut self, packet: Packet) {
        match packet.clone().pack_type {
            PacketType::Nack(nack) => self.handle_nack(nack, packet.routing_header.hops[0], packet.session_id),
            PacketType::Ack(ack) => self.handle_ack(ack, &packet.routing_header.hops, packet.session_id),
            PacketType::FloodResponse(_) => self.process_flood_response(packet),
            PacketType::FloodRequest(r) => self.process_flood_request(r, packet),
            PacketType::MsgFragment(fragment) => {
                if let Some(data) = self.handle_msg_fragment(packet.clone(), fragment) {
                    match serde_json::from_slice::<ClientNetCommand>(&data) {
                        Ok(cmd) => self.handle_command(cmd, packet.routing_header.hops[0]),
                        Err(e) => self.send_logging(format!("Failed to parse ClientNetCommand: {}", e)),
                    }
                } else {
                    self.send_logging("Waiting for more fragments to complete message.".to_string());
                }
            }
        }
    }

    /// Search for an existing chat between two clients (unordered)
    fn check_if_chat_exists(&self, user1: NodeId, user2: NodeId) -> Option<usize> {
        let (min_user, max_user) = if user1 < user2 { (user1, user2) } else { (user2, user1) };

        self.chats.iter().position(|chat| chat.users.user1 == min_user && chat.users.user2 == max_user)
    }

    /// Save message into the appropriate chat history
    fn save_message(&mut self, cmd: ClientNetCommand, source: NodeId) {
        match cmd {
            ClientNetCommand::MessageFor(destination, _, msg) => {
                self.insert_message(source, destination, TextMessage::Message(msg));
            }
            ClientNetCommand::MediaFor(destination, _, media_id, extension, message) => {
                self.insert_message(source, destination, TextMessage::Media(MediaSC::Image(media_id, extension, message)));
            }
            ClientNetCommand::FileFor(destination, _, file_id, extension, message) => {
                self.insert_message(source, destination, TextMessage::Media(MediaSC::File(file_id, extension, message)));
            }
            _ => self.send_logging("Unexpected ClientNetCommand type received in save_message.".to_string()),
        }
    }

    /// Insert a message in the chat (creates the chat if it does not exist)
    fn insert_message(&mut self, source: NodeId, destination: NodeId, message: TextMessage) {
        let (min_user, max_user) = if source < destination { (source, destination) } else { (destination, source) };
        let now = chrono::Utc::now().timestamp_millis() as u64;

        let metadata = MessageMetaData { id: source, timestamp: now };

        match self.check_if_chat_exists(min_user, max_user) {
            Some(index) => {
                self.chats[index].chat.push((message, metadata));
                self.send_chat_to_simcontroller(&self.chats[index]);
            }
            None => {
                let new_chat = Chat {
                    chat: vec![(message, metadata)],
                    users: ChatUsers { user1: min_user, user2: max_user },
                };
                self.send_chat_to_simcontroller(&new_chat);
                self.chats.push(new_chat);
            }
        }
    }

    /// (Optional) Send chat state to Simulation Controller (currently disabled)
    fn send_chat_to_simcontroller(&self, chat: &Chat) {
        match serde_json::to_string(chat) {
            Ok(serialized_chat) => {
                // Placeholder: controller communication not used yet
            }
            Err(e) => {
                self.send_logging(format!("Failed to serialize chat: {}", e));
            }
        }
    }
}

// Chat data structures for message persistence

#[derive(Serialize, Deserialize, Clone)]
pub struct Chat {
    pub chat: Vec<(TextMessage, MessageMetaData)>,
    pub users: ChatUsers,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatUsers {
    pub user1: NodeId,
    pub user2: NodeId,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MessageMetaData {
    pub id: NodeId,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum TextMessage {
    Message(String),
    Media(MediaSC),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum MediaSC {
    Image(usize, String, LeImage),
    File(usize, String, Vec<u8>),
}
