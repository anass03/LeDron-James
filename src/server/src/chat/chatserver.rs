use crate::utils::handle_server_control_loop;
use crate::{GenServer, GenServerBehavior};
use crossbeam_channel::{Receiver, Sender};
use delegate::delegate;
use host::{ClientCommand, ServerToClient, Message, Functionality, ServerTypes, generate_unique_session_id, NetCommand, HostEvent};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use wg_2024::controller::DroneEvent;
use wg_2024::network::NodeId;
use wg_2024::packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, NodeType, Packet, PacketType};
use tracing::{info, error};

pub struct ChatServer {
    info: GenServer,
    client_list: Vec<NodeId>,
}

impl ChatServer {
    delegate! {
        to self.info.host {
            //fn log<S: AsRef<str>>(&self, message: S);
            fn initiate_flood(&mut self);
            fn process_flood_request(&mut self, flood_request: FloodRequest, packet: Packet);
            fn process_flood_response(&mut self, response: FloodResponse);
            fn handle_ack(&self, ack: Ack, session_id: u64);
            fn handle_nack(&mut self, nack: Nack, nack_generator: NodeId, session_id: u64);
        }
        to self.info {
            fn create_message(&mut self, data: Vec<u8>, destination: NodeId);
            fn servertype(&mut self, request: ClientCommand, destination: NodeId);
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
            client_list: vec![],
        }
    }

    pub fn run(&mut self) {
        self.initiate_flood();
        //self.log("Starting Chat Server...");
        loop {
            crossbeam_channel::select_biased! {
                recv(self.info.host.packet_recv) -> packet => {
                    self.handling_packets(packet);
                }
                default(Duration::from_millis(100)) => {
                    let controller_recv = self.info.controller_recv.clone();
                    handle_server_control_loop(&controller_recv, |cmd| {
                        if let NetCommand::ControllerShortcut(_) = cmd {
                            self.initiate_flood();
                        }
                    });
                }
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
                    error!("Failed to send response to {}: {}", destination, e);
                }
            }
            Err(e) => {
                error!("Failed to serialize ServerToClient: {}", e);
            }
        }
    }

    fn handle_command(&mut self, cmd: ClientCommand, source: NodeId) {
        match cmd {
            ClientCommand::ServerType => {
                self.send_response(ServerToClient::ServerType(ServerTypes::Chat), source);
            }
            ClientCommand::RegistrationToChat => {
                if !self.client_list.contains(&source) {
                    self.client_list.push(source);
                    info!("Registered new client to chat: {}", source);
                }
            }
            ClientCommand::ClientList => {
                self.send_response(ServerToClient::ClientList(self.client_list.clone()), source);
            }
            ClientCommand::MessageFor(dest, msg) => {
                self.send_response(ServerToClient::MessageFrom(source, msg), dest);
            }
            _ => {
                self.send_response(ServerToClient::ErrNotFound, source);
            }
        }
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
                        match serde_json::from_slice::<ClientCommand>(&data) {
                            Ok(cmd) => self.handle_command(cmd, packet.routing_header.hops[0]),
                            Err(_) => self.send_response(ServerToClient::ErrNotFound, packet.routing_header.hops[0]),
                        }
                    } else {
                        //self.log("Failed to reassemble message fragment.");
                    }
                }
            }
        }
    }
}

#[test]
fn main() {}