use crate::{delegate_to_host_mut, delegate_to_host_ref, GenServer};
use crossbeam_channel::{tick, Receiver, Sender};
use delegate::delegate;
use host::{ServerToClient, ServerTypes, HostEvent, HostCommand, ClientNetCommand, Image};
use host::functionality::Functionality;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;
use chrono::Local;
use host::flood::FloodControl;
use host::logging::{LogControl, LogModule, LogType};
use toml::value::Datetime;
use serde::{Deserialize, Serialize};
use wg_2024::network::NodeId;
use wg_2024::packet::{Ack, FloodRequest, Fragment, Nack, NodeType, Packet, PacketType};

/// A chat server implementation that supports text, media, and file communication between clients.
pub struct ChatServer {
    info: GenServer,
    client_list: Vec<NodeId>,
    chats: Vec<Chat>
}

impl ChatServer {
    delegate_to_host_ref!(info => {
        pub fn send_logging(&self, log_type: LogType, module: LogModule, msg: String);
    });
    delegate_to_host_mut!(info => {
        fn initiate_flood(&mut self);
        fn handle_ack(&mut self, ack: Ack, hops: &[NodeId], session_id: u64);
        fn handle_nack(&mut self, nack: Nack, generator: NodeId, session_id: u64);
        fn process_flood_request(&mut self, flood_request: FloodRequest, packet: Packet);
        fn process_flood_response(&mut self, packet: Packet);
    });

    delegate! {
        to self.info {
            fn send_response(&mut self, response: ServerToClient, destination: NodeId);
            fn handle_sim_command(&mut self, cmd: HostCommand);
            fn servertype(&mut self) -> Result<ServerTypes, String>;
            fn handle_msg_fragment(&mut self, packet: Packet, fragment: Fragment) -> Option<Vec<u8>>;
        }
    }

    /// Constructs a new instance of `ChatServer2`.
    pub fn new(
        id: NodeId,
        type_host: NodeType,
        connected_drone_ids: Vec<NodeId>,
        controller_send: Sender<HostEvent>,
        controller_recv: Receiver<HostCommand>,
        packet_recv: Receiver<Packet>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
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
            ),
            client_list: Vec::new(),
            chats: Vec::new(),
        }
    }

    /// Starts the main event loop of the Chat Server.
    pub fn run(&mut self){
        let speed_tick = tick(Duration::from_millis(5000));

        self.initiate_flood();
        self.send_logging(LogType::INFO,LogModule::CHAT,"Starting Chat Server...".into());
        loop {
            crossbeam_channel::select_biased! {
                recv(self.info.controller_recv) -> cmd => match cmd {
                    Ok(c) => self.handle_sim_command(c),
                    Err(e) => self.send_logging(LogType::ERROR,LogModule::PACKET,format!("Command error: {e}")),
                },
                recv(self.info.host.packet_recv) -> pkt => match pkt {
                    Ok(p) => self.handling_packets(p),
                    Err(e) => self.send_logging(LogType::ERROR,LogModule::PACKET,format!("Packet error: {e}")),
                },
                default(Duration::from_millis(3)) => {}
                recv(speed_tick) -> _ =>{
                    match self.info.host.controller_send.send(HostEvent::ExpectedPDR(self.info.host.id, self.info.host.network.drone_pdr.clone())) {
                         Ok(_) => {}
                         Err(e) => {self.info.host.send_logging(LogType::ERROR, LogModule::NETWORK,format!("Error reporting PDR: {}", e));}
                     }
                }
            }
        }
    }

    /// Handles all incoming packets based on their packet type.
    fn handling_packets(&mut self, packet: Packet) {
        use PacketType::*;
        match packet.clone().pack_type {
            Nack(nack) => {
                if let Some(generator) = packet.routing_header.source() {
                    self.handle_nack(nack, generator, packet.session_id)
                }
            },
            Ack(ack) => {
                self.handle_ack(ack, &packet.routing_header.hops, packet.session_id)
            },
            FloodRequest(r) => {
                self.process_flood_request(r, packet)
            },
            FloodResponse(_) => {
                self.process_flood_response(packet)
            },
            MsgFragment(fragment) => {
                if let Some(data) = self.handle_msg_fragment(packet.clone(), fragment) {
                    match serde_json::from_slice::<ClientNetCommand>(&data) {
                        Ok(cmd) => {
                            if let Some(generator) = packet.routing_header.source() {
                                self.handle_command(cmd,generator);
                            }
                        },
                        Err(e) => {
                            if let Some(generator) = packet.routing_header.source() {
                                self.send_response(ServerToClient::ErrNotFound(self.info.host.id), generator);
                            }
                            self.send_logging(LogType::ERROR,LogModule::PACKET,format!("Deserialization error: {e}"))
                        },
                    }
                } else {
                    self.send_logging(LogType::WARN,LogModule::PACKET,"The packet is not complete; server is waiting.".to_string());
                }
            }
        }
    }

    /// Processes high-level commands from the client.
    fn handle_command(&mut self, cmd: ClientNetCommand, source: NodeId) {
        use ClientNetCommand::*;
        match cmd {
            ServerType => {
                self.send_server_type(source)
            },
            RegistrationToChat(id) => {
                self.register_client(id, source)
            },
            ClientList(id) => {
                self.send_client_list(id, source)
            },
            MessageFor(to, recv, msg) => {
                self.handle_chat_message(to, recv, msg, source)
            },
            FileFor(to, recv, fid, msg) => {
                self.handle_file_transfer(to, recv, fid, msg, source)
            },
            MediaFor(to, recv, mid, msg) => {
                self.handle_media_transfer(to, recv, mid, msg, source)
            },
            _ => {
                self.send_response(ServerToClient::WrongServer(self.info.host.id), source)
            },
        }
    }

    /// Saves a message or media command into the server's chat history.
    fn save_message(&mut self, cmd: ClientNetCommand, source: NodeId) {
        match cmd {
            ClientNetCommand::MessageFor(destination, _, msg) => {
                self.insert_message(source, destination, MessageSC::Message(msg));
            }
            ClientNetCommand::MediaFor(destination, _, media_id, message) => {
                self.insert_message(source, destination, MessageSC::Media(MediaSC::Image(media_id, message)));
            }
            ClientNetCommand::FileFor(destination, _, file_id, message) => {
                self.insert_message(source, destination, MessageSC::Media(MediaSC::File(file_id, message)));
            }
            _ => self.send_logging(LogType::WARN,LogModule::CHAT,"Unexpected command in save_message".into()),
        }
    }

    /// Inserts a message into the chat history, initializing a new chat if necessary.
    fn insert_message(&mut self, from: NodeId, to: NodeId, message: MessageSC) {
        let (a, b) = if from < to { (from, to) } else { (to, from) };
        let timestamp = Local::now().to_rfc3339().parse().unwrap_or_else(|_| Datetime::from_str("1970-01-01T00:00:00Z").unwrap());
        let metadata = MessageMetaData { id: from, timestamp };

        match self.chats.iter_mut().find(|chat| chat.users == ChatUsers { user1: a, user2: b }) {
            Some(chat) => {
                chat.chat.push((message, metadata));
                let snapshot = chat.clone();
                self.send_chat_to_simcontroller(&snapshot);
            },
            None => {
                let new_chat = Chat {
                    chat: vec![(message, metadata)],
                    users: ChatUsers { user1: a, user2: b },
                };
                self.send_chat_to_simcontroller(&new_chat);
                self.chats.push(new_chat);
            }
        }
    }

    /// Sends the serialized chat to the simulation controller.
    fn send_chat_to_simcontroller(&self, chat: &Chat) {
        match serde_json::to_string(chat) {
            Ok(serialized_chat) => {
                let _ = self.info.host.controller_send.send(HostEvent::ChatUpdate(self.info.host.id, serialized_chat));
            }
            Err(e) => {
                self.send_logging(LogType::ERROR,LogModule::CHAT,format!("Failed to serialize chat: {}", e));
            }
        }
    }

    /// Registers a new client to the chat server.
    fn register_client(&mut self, receiver: NodeId, source: NodeId) {
        if self.is_right_receiver(receiver) && !self.client_list.contains(&source) {
            self.client_list.push(source);
            self.send_logging(LogType::INFO,LogModule::CHAT,format!("Client {} registered", source));
        }
    }

    /// Sends the type of server to the client.
    fn send_server_type(&mut self, source: NodeId) {
        match self.servertype() {
            Ok(server_type) => {
                self.send_logging(LogType::INFO,LogModule::NETWORK,"Sending server type".to_string());
                self.send_response(ServerToClient::ServerType(server_type), source);
            }
            Err(e) => self.send_logging(LogType::ERROR,LogModule::PACKET,format!("Error in getting ServerType: {}", e)),
        }
    }

    /// Sends the list of registered clients to the requester.
    fn send_client_list(&mut self, receiver: NodeId, source: NodeId) {
        if self.is_right_receiver(receiver) {
            self.send_response(ServerToClient::ClientList(source, self.info.host.id, self.client_list.clone()), source);
             self.send_logging(LogType::INFO,LogModule::CHAT,"Sending client list".into());
        } else {
            self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
        }
    }

    /// Handles a plain text chat message between clients.
    fn handle_chat_message(&mut self, destination: NodeId, receiver_id: NodeId, msg: String, source: NodeId) {
        if self.is_right_receiver(receiver_id) {
            if self.client_list.contains(&source) {
                if self.client_list.contains(&destination) {
                    self.save_message(ClientNetCommand::MessageFor(destination, receiver_id, msg.clone()), source);
                    self.send_logging(LogType::INFO,LogModule::CHAT,format!("Sending message to {}: {}", destination, msg));
                    self.send_response(ServerToClient::MessageFrom(self.info.host.id, source, msg), destination);
                } else {
                    self.send_logging(LogType::WARN,LogModule::CHAT,format!("Destination {} not registered", destination));
                    self.send_response(ServerToClient::ErrClientId(self.info.host.id), destination);
                }
            } else {
                self.send_logging(LogType::WARN,LogModule::CHAT,"Source client not registered".into());
            }
        } else {
            self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
        }
    }

    /// Handles a file transfer between clients.
    fn handle_file_transfer(&mut self, destination: NodeId, receiver_id: NodeId, file_id: String, message: Vec<u8>, source: NodeId) {
        if self.is_right_receiver(receiver_id) {
            if self.client_list.contains(&source) {
                if self.client_list.contains(&destination) {
                    self.save_message(ClientNetCommand::FileFor(destination, receiver_id, file_id.clone(), message.clone()), source.clone());
                    self.send_logging(LogType::INFO,LogModule::CHAT,format!("Sending file to {}", destination));
                    self.send_response(ServerToClient::FileFrom(self.info.host.id, source,destination, file_id, message), destination);
                } else {
                    self.send_logging(LogType::WARN,LogModule::CHAT,format!("Destination {} not registered", destination));
                    self.send_response(ServerToClient::ErrClientId(self.info.host.id), destination);
                }
            } else {
                self.send_logging(LogType::WARN,LogModule::CHAT,"Source client not registered".into());
            }
        } else {
            self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
        }
    }

    /// Handles a media (image) transfer between clients.
    fn handle_media_transfer(&mut self, destination: NodeId, receiver_id: NodeId, media_id: String, message: Image, source: NodeId) {
        if self.is_right_receiver(receiver_id) {
            if self.client_list.contains(&source) {
                if self.client_list.contains(&destination) {
                    self.save_message(ClientNetCommand::MediaFor(destination, receiver_id, media_id.clone(), message.clone()), source.clone());
                    self.send_logging(LogType::INFO,LogModule::CHAT,format!("Sending media to {}", destination));
                    self.send_response(ServerToClient::MediaFrom(self.info.host.id, source,destination, media_id, message), destination);
                } else {
                    self.send_logging(LogType::WARN,LogModule::CHAT,"Destination client not registered".into());
                }
            } else {
                self.send_logging(LogType::WARN,LogModule::CHAT,"Source client not registered".into());
                self.send_response(ServerToClient::ErrClientId(self.info.host.id), destination);
            }
        } else {
            self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
        }
    }

    /// Checks whether the receiver ID matches this server.
    fn is_right_receiver(&self, receiver: NodeId) -> bool {
        self.info.host.id == receiver
    }
}

/// Represents a chat between two clients.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Chat {
    pub chat: Vec<(MessageSC, MessageMetaData)>,
    pub users: ChatUsers,
}

impl Chat {
    /// Returns the last message in the chat, if any.
    pub fn get_last_message(&self) -> Option<&(MessageSC, MessageMetaData)> {
        self.chat.last()
    }
}

/// Represents a pair of users in a chat.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ChatUsers {
    pub user1: NodeId,
    pub user2: NodeId,
}

/// Contains metadata about a message.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MessageMetaData {
    pub id: NodeId,
    pub timestamp: Datetime,
}

/// Enum representing a chat message.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MessageSC {
    Message(String),
    Media(MediaSC),
}

/// Enum for media types in chat.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MediaSC {
    Image(String, Image),
    File(String, Vec<u8>),
}
