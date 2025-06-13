use crate::{GenServer, GenServerBehavior};
use crossbeam_channel::{Receiver, Sender};
use delegate::delegate;
use host::{ServerToClient, Message, Functionality, ServerTypes, generate_unique_session_id, HostEvent, HostCommand,ClientNetCommand};
use std::collections::HashMap;
use std::time::Duration;
use wg_2024::network::NodeId;
use wg_2024::packet::{Ack, FloodRequest, Fragment, Nack, NodeType, Packet, PacketType};


pub struct ChatServer {
    info: GenServer,
    client_list: Vec<NodeId>,
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
    
    
    fn send_response(&mut self, response: ServerToClient, source: NodeId) {
        match serde_json::to_vec(&response) {
            Ok(data) => {
                let msg = Message {
                    session_id: generate_unique_session_id(),
                    content: data,
                };
                if let Err(e) = self.info.build_msg_fragment(source, msg) {
                    self.send_logging(format!(" Failed to send response to {}: {}", source, e));
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
}