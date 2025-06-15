use crate::{GenServer, GenServerBehavior};
use crossbeam_channel::{Receiver, Sender};
use delegate::delegate;
use host::{ServerToClient, Message, Functionality, ServerTypes, generate_unique_session_id, HostEvent, HostCommand,ClientNetCommand};
use std::collections::HashMap;
use std::error::Error;
use host::Image as LeImage;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use wg_2024::network::NodeId;
use wg_2024::packet::{Ack, FloodRequest, Fragment, Nack, NodeType, Packet, PacketType};

pub struct ChatServer {
    info: GenServer,
    client_list: Vec<NodeId>,
    chats: Vec<Chat>
}


impl ChatServer {
    delegate! {
        to self.info.host {
            fn initiate_flood(&mut self);
            fn process_flood_request(&mut self, flood_request: FloodRequest, packet: Packet);
            fn process_flood_response(&mut self, packet: Packet);
            fn handle_ack(&mut self, ack: Ack, hops: &[NodeId], session_id: u64);
            fn handle_nack(&mut self, nack: Nack, nack_generator: NodeId, session_id: u64);
            fn send_logging(&self, message: String);
        }
        to self.info {
            fn handle_sim_command(&mut self, cmd: HostCommand);
            fn servertype(&mut self) -> Result<ServerTypes, String>;
            fn handle_msg_fragment(&mut self, packet: Packet, fragment: Fragment) -> Option<Vec<u8>>;
            fn compress_anything(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>>;
        }
    }


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

                default(Duration::from_millis(10)) => {}
            }
        }
    }


    fn send_response(&mut self, response: ServerToClient, destination: NodeId) {
        match serde_json::to_vec(&response) {
            Ok(data) => {
                if let Ok(compressed_data)=self.compress_anything(data.as_slice()){
                    let msg = Message {
                        session_id: generate_unique_session_id(),
                        content: compressed_data,
                    };
                    if let Err(e) = self.info.build_msg_fragment(destination, msg) {
                        self.send_logging(format!("Failed to send response to {}: {}", destination, e));
                    }
                }
            }
            Err(e) => {
                self.send_logging(format!(" Failed to serialize ServerToClient: {}", e));
            }
        }
    }

    fn handle_command(&mut self, cmd: ClientNetCommand, source: NodeId) {
        match cmd {
            ClientNetCommand::ServerType => {
                if let Ok(server_type) = self.servertype() {
                    self.send_logging("Sending server type".to_string());
                    self.send_response(ServerToClient::ServerType(server_type), source);
                } else {
                    self.send_logging(" Failed to send ServerToClient::ServerType".to_string());
                }
            }
            ClientNetCommand::RegistrationToChat(receiver_id) => {
                match receiver_id==self.info.host.id {
                    true => {
                        if !self.client_list.contains(&source) {
                            self.client_list.push(source);
                            self.send_logging(format!(" Registered new client to chat: {}", source));
                        }else{
                            self.send_logging(format!(" Registration from {} already exists!", source));
                        }}
                    false => {self.send_response(ServerToClient::WrongServer(self.info.host.id), source);}
                }
            }
            ClientNetCommand::ClientList(receiver_id) => {
                match receiver_id==self.info.host.id {
                    true => {
                        self.send_response(ServerToClient::ClientList(self.info.host.id,self.client_list.clone()), source);
                        self.send_logging("Sending client list".to_string());
                    }
                    false => {self.send_response(ServerToClient::WrongServer(self.info.host.id), source);}
                }
            }
            ClientNetCommand::MessageFor(destination, receiver_id,msg ) => {
                match receiver_id ==self.info.host.id {
                    true => {
                        match self.check_if_in_client_list(source) {
                            true => {
                                match self.check_if_in_client_list(destination) {
                                    true => {
                                        self.save_message(ClientNetCommand::MessageFor(destination,receiver_id,msg.clone()), source);
                                        self.send_logging(format!("Sending message to {} , and messaage is : {}", destination, msg));
                                        self.send_response(ServerToClient::MessageFrom(self.info.host.id,source, msg), destination);},
                                    false => {
                                        self.send_logging(format!("{} is not registered to client chat", destination));
                                        self.send_response(ServerToClient::ErrClientId(self.info.host.id), destination);}
                                }}
                            false => {self.send_logging(" You're not registered to client chat".to_string())}
                        }
                    }
                    false => {self.send_response(ServerToClient::WrongServer(self.info.host.id), source);}
                }

            }
            ClientNetCommand::FileFor(destination, receiver_id, file_id, extension, message) => {
                match receiver_id ==self.info.host.id {
                    true => {
                        match self.check_if_in_client_list(source) {
                            true => {
                                match self.check_if_in_client_list(destination) {
                                    true => {
                                        self.save_message(ClientNetCommand::FileFor(destination,receiver_id,file_id,extension.clone(),message.clone()),source.clone());
                                        self.send_logging(format!("Sending file to {}", destination));
                                        self.send_response(ServerToClient::FileFrom(self.info.host.id,source,file_id,extension,message), destination);
                                    }
                                    false => {
                                        self.send_logging(format!("{} is not registered to client chat", destination));
                                        self.send_response(ServerToClient::ErrClientId(self.info.host.id), destination);
                                    }
                                }
                            }
                            false => {self.send_logging(" You're not registered to client chat".to_string())}
                        }
                    }
                    false => {self.send_response(ServerToClient::WrongServer(self.info.host.id), source);}
                }
            }
            ClientNetCommand::MediaFor(destination, receiver_id, media_id, extension, message) => {
                match receiver_id ==self.info.host.id {
                    true => {
                        match self.check_if_in_client_list(source) {
                            true => {
                                match self.check_if_in_client_list(destination) {
                                    true => {
                                        self.save_message(ClientNetCommand::MediaFor(destination, receiver_id, media_id, extension.clone(), message.clone()),source.clone());
                                        self.send_logging(format!("Sending media to {}", destination));
                                        self.send_response(ServerToClient::MediaFrom(self.info.host.id,source,media_id,extension,message), destination);
                                    }
                                    false => {
                                        self.send_logging(" The destination is not registered to client chat".to_string());
                                    }
                                }
                            }
                            false => {
                                self.send_logging(" You're not registered to client chat".to_string());
                                self.send_response(ServerToClient::ErrClientId(self.info.host.id), destination);
                            }
                        }
                    }
                    false => {self.send_response(ServerToClient::WrongServer(self.info.host.id), source);}
                }
            }
            _ => {
                self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
            }
        }
    }


    fn check_if_in_client_list(&mut self, id: NodeId) -> bool {
        self.client_list.contains(&id)
    }


    fn handling_packets(&mut self, packet:Packet) {
        match packet.clone().pack_type {
            PacketType::Nack(nack) => self.handle_nack(nack, packet.routing_header.hops[0], packet.session_id),
            PacketType::Ack(ack) => self.handle_ack(ack, &*packet.routing_header.hops, packet.session_id),
            PacketType::FloodResponse(_) => self.process_flood_response(packet),
            PacketType::FloodRequest(r) => self.process_flood_request(r, packet),
            PacketType::MsgFragment(fragment) => {
                if let Some(data) = self.handle_msg_fragment(packet.clone(), fragment) {
                    if let Ok(cmd) = serde_json::from_slice::<ClientNetCommand>(&data) {
                        self.handle_command(cmd, packet.routing_header.hops[0]);
                    }
                }else {
                    self.send_logging("The packet is not complete and server is waiting".to_string());
                }
            }
        }
    }
    fn check_if_chat_exists(&self, user1: NodeId, user2: NodeId) -> Option<usize> {
        // Ordiniamo sempre i NodeId per evitare duplicati A-B e B-A
        let (min_user, max_user) = if user1 < user2 { (user1, user2) } else { (user2, user1) };

        self.chats.iter().position(|chat| {
            chat.users.user1 == min_user && chat.users.user2 == max_user
        })
    }

    // Funzione per salvare i messaggi (testo, media, file) nella chat corrispondente
    fn save_message(&mut self, cmd: ClientNetCommand, source: NodeId) {
        match cmd {
            ClientNetCommand::MessageFor(destination, _, msg) => {
                self.insert_message(
                    source,
                    destination,
                    TextMessage::Message(msg)
                );
            }

            ClientNetCommand::MediaFor(destination, _receiver_id, media_id, extension, message) => {
                // Qui puoi estendere se vuoi salvare i contenuti multimediali
                self.insert_message(
                    source,
                    destination,
                    TextMessage::Media(MediaSC::Image(media_id,extension,message))
                );
            }

            ClientNetCommand::FileFor(destination, _receiver_id, file_id, extension, message) => {
                self.insert_message(
                    source,
                    destination,
                    TextMessage::Media(MediaSC::File(file_id,extension,message))  // placeholder, da migliorare
                );
            }

            _ => {
                self.send_logging("Panicked because message arrived in the wrong function".to_string());
            }
        }
    }

    // Funzione di supporto per inserire il messaggio nella chat corretta
    fn insert_message(&mut self, source: NodeId, destination: NodeId, message: TextMessage) {
        let (min_user, max_user) = if source < destination { (source, destination) } else { (destination, source) };
        
        let now = chrono::Utc::now().timestamp_millis() as u64;

        let metadata = MessageMetaData {
            id: source,
            timestamp: now,
        };

        match self.check_if_chat_exists(min_user, max_user) {
            Some(index) => {
                self.chats[index].chat.push((message, metadata));
                self.send_chat_to_simcontroller(&self.chats[index]);
            }
            None => {
                let new_chat = Chat {
                    chat: vec![(message, metadata)],
                    users: ChatUsers {
                        user1: min_user,
                        user2: max_user,
                    },
                };
                self.send_chat_to_simcontroller(&new_chat);
                self.chats.push(new_chat);
            }
        }
    }
    fn send_chat_to_simcontroller(&self, chat: &Chat) {
        match serde_json::to_string(chat) {
            Ok(serialized_chat) => {
                // let _ = self.info.host.controller_send.send(HostEvent::ChatUpdate(serialized_chat)); 
            }
            Err(e) => {
                self.send_logging(format!("Failed to serialize chat: {}", e));
            }
        }
    }
}




/// Struct Temporanee
#[derive(Serialize, Deserialize, Clone)]
pub struct Chat{
    pub chat:Vec<(TextMessage, MessageMetaData)>,
    pub users:ChatUsers,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct ChatUsers{
    pub user1:NodeId,
    pub user2:NodeId,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct MessageMetaData{
    pub id:NodeId,
    pub timestamp:u64,
}
#[derive(Serialize, Deserialize, Clone)]
pub enum TextMessage{
    Message(String), Media(MediaSC)
}
#[derive(Serialize, Deserialize, Clone)]
pub enum MediaSC{
    Image(usize,String,LeImage),
    File(usize,String,Vec<u8>)
}


