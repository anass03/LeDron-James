use crate::utils::handle_server_control_loop;
use crate::{GenServer, GenServerBehavior};
use crossbeam_channel::{Receiver, Sender};
use delegate::delegate;
use host::{ClientCommand, ServerToClient, Message, Functionality, ServerTypes, generate_unique_session_id, NetCommand, HostEvent};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use tracing_subscriber::fmt::format;
use wg_2024::network::NodeId;
use wg_2024::packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, NodeType, Packet, PacketType};

pub struct ChatServer {
    info: GenServer,
    client_list: Vec<NodeId>,
}

impl ChatServer {
    delegate! {
        to self.info.host {
            fn initiate_flood(&mut self);
            fn process_flood_request(&mut self, flood_request: FloodRequest, packet: Packet);
            fn process_flood_response(&mut self, response: FloodResponse);
            fn handle_ack(&self, ack: Ack, session_id: u64);
            fn handle_nack(&mut self, nack: Nack, nack_generator: NodeId, session_id: u64);
            fn send_logging(&self, message: String);
        }
        to self.info {
            fn create_message(&mut self, data: Vec<u8>, destination: NodeId);
            fn servertype(&mut self) -> Result<ServerTypes, String>;
            fn handle_msg_fragment(&mut self, packet: Packet, fragment: Fragment) -> Option<Vec<u8>>;
        }
    }

    pub fn new(
        id: NodeId,
        type_host: NodeType,
        connected_drone_ids: Vec<NodeId>,
        controller_send: Sender<HostEvent>,
        controller_recv: Receiver<NetCommand>,
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
        self.send_logging(format!("Starting Chat Server..."));
        loop {
            crossbeam_channel::select_biased! {
                recv(self.info.controller_recv) -> cmd => {
                    if let Ok(command) = cmd {
                       // self.handle_command(command);
                    }
                }
                
                recv(self.info.host.packet_recv) -> pkt => {
                    if let Ok(packet) = pkt {
                         self.handling_packets(Ok(packet)) 
                    }
                }
                
                default(Duration::from_millis(10)) => {}
            }
        }
    }

    fn send_response(&mut self, response: ServerToClient, destination: NodeId) {
        match serde_json::to_vec(&response) {
            Ok(data) => {
                let msg = Message {
                    session_id: generate_unique_session_id(),
                    content: data,
                };
                if let Err(e) = self.info.build_msg_fragment(destination, msg) {
                    self.send_logging(format!("[Server id={}]  Failed to send response to {}: {}",self.info.host.id, destination, e));
                }
            }
            Err(e) => {
                self.send_logging(format!("[Server id={}] Failed to serialize ServerToClient: {}",self.info.host.id, e));
            }
        }
    }

    fn handle_command(&mut self, cmd: ClientCommand, destination: NodeId) {
        match cmd {
            ClientCommand::ServerType => {
                if let Ok(server_type) = self.servertype() {
                    self.send_response(ServerToClient::ServerType(server_type), destination);
                } else {
                    self.send_logging(format!("[Server id={}] Failed to send ServerToClient::ServerType", self.info.host.id));
                }
            }
            ClientCommand::RegistrationToChat => {
                if !self.client_list.contains(&destination) {
                    self.client_list.push(destination);
                    self.send_logging(format!("[Server id={}] Registered new client to chat: {}",self.info.host.id, destination));
                }else{
                    self.send_logging(format!("[Server id={}] Registration from {} already exists!",self.info.host.id, destination));
                }
            }
            ClientCommand::ClientList => {
                self.send_response(ServerToClient::ClientList(self.client_list.clone()), destination);
            }
            ClientCommand::MessageFor(dest, msg) => {
                match self.check_if_in_client_list(destination) {
                    true => {match self.check_if_in_client_list(dest) {
                        true => {self.send_response(ServerToClient::MessageFrom(destination, msg), dest)}
                        false => {self.send_logging(format!("[Server id={}] The destination is not registered to client chat",self.info.host.id));}
                    }}
                    false => {self.send_logging(format!("[Server id={}] You're not registered to client chat",self.info.host.id))}
                }
            }
            _ => {
                self.send_response(ServerToClient::ErrNotFound, destination);
            }
        }
    }
    fn check_if_in_client_list(&mut self, id: NodeId) -> bool {
        self.client_list.contains(&id)
    }

    fn handling_packets(&mut self, packet: Result<Packet, crossbeam_channel::RecvError>) {
        if let Ok(packet) = packet {
            match packet.clone().pack_type {
                PacketType::Nack(nack) => self.handle_nack(nack, packet.routing_header.hops[0], packet.session_id),
                PacketType::Ack(ack) => self.handle_ack(ack, packet.session_id),
                PacketType::FloodResponse(r) => self.process_flood_response(r),
                PacketType::FloodRequest(r) => self.process_flood_request(r, packet),
                PacketType::MsgFragment(fragment) => {
                    if let Some(data) = self.handle_msg_fragment(packet.clone(), fragment) {
                        if let Ok(cmd) = serde_json::from_slice::<ClientCommand>(&data) {
                            self.handle_command(cmd, packet.routing_header.hops[0]);
                        }
                    }
                }
            }
        }
    }
}