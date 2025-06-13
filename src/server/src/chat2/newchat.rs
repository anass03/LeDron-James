use std::sync::Arc;
use crossbeam_channel::{Receiver, Sender, select};
use delegate::delegate;
use crate::{GenServer, GenServerBehavior};
use host::{ServerToClient, ClientNetCommand, HostCommand, HostEvent, Message, Functionality,ServerTypes, generate_unique_session_id};
use wg_2024::network::NodeId;
use wg_2024::packet::{Packet, PacketType, Ack, Nack, FloodRequest, FloodResponse, Fragment, NodeType};
use std::time::Duration;
use crate::chat2::SharedChatMemory;

pub struct ChatServer2 {
    info: GenServer,
    client_list: Vec<NodeId>,

    // Memoria condivisa per storico e live-feed
    chat_mem: SharedChatMemory,
    // Receiver per live-feed di questa stanza
    live_rx: Receiver<ServerToClient>,
}

impl ChatServer2 {
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
        packet_send: std::collections::HashMap<NodeId, Sender<Packet>>,
        log_sender: Sender<String>,
        chat_mem: SharedChatMemory,
    ) -> Self {
        chat_mem.ensure_broadcaster(id);
        let live_rx = chat_mem.subscribe(id);

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
            client_list: Vec::new(),
            chat_mem,
            live_rx,
        }
    }

    pub fn run(&mut self) {
        self.initiate_flood();
        self.send_logging("Starting Chat Server…".into());
        loop {
            select! {
                // Eventi live da altre istanze
                recv(self.live_rx) -> ev => if let Ok(ev) = ev {
                    for &cli in &self.client_list.clone() {
                        self.send_response(ev.clone(), cli);
                    }
                },
                // Comandi simulatore
                recv(self.info.controller_recv) -> cmd => if let Ok(cmd) = cmd {
                    self.handle_sim_command(cmd);
                },
                // Pacchetti dalla rete: priorità a MsgFragment
                recv(self.info.host.packet_recv) -> pkt => if let Ok(packet) = pkt {
                    // Prima gestiamo eventuali MsgFragment senza muovere `packet`
                    if let PacketType::MsgFragment(fragment) = packet.clone().pack_type {
                        if let Some(data) = self.handle_msg_fragment(packet.clone(), fragment.clone()) {
                            if let Ok(cmd) = serde_json::from_slice::<ClientNetCommand>(&data) {
                                self.handle_command(cmd, packet.routing_header.hops[0]);
                            }
                        }
                        continue;
                    }
                    // Altri tipi di pacchetto
                    match packet.clone().pack_type {
                        PacketType::Ack(ack) => {
                            self.handle_ack(ack, &*packet.routing_header.hops, packet.session_id);
                        }
                        PacketType::Nack(nack) => {
                            self.handle_nack(nack, packet.routing_header.hops[0], packet.session_id);
                        }
                        PacketType::FloodRequest(r) => {
                            self.process_flood_request(r, packet.clone());
                        }
                        PacketType::FloodResponse(_) => {
                            self.process_flood_response(packet.clone());
                        }
                        _ => {}
                    }
                },
                default(Duration::from_millis(10)) => {}
            }
        }
    }

    fn send_response(&mut self, response: ServerToClient, source: NodeId) {
        if let Ok(data) = serde_json::to_vec(&response) {
            let msg = Message { session_id: generate_unique_session_id(), content: data };
            if let Err(e) = self.info.build_msg_fragment(source, msg) {
                self.send_logging(format!("Failed to send response to {}: {}", source, e));
            }
        } else {
            self.send_logging("Failed to serialize response".into());
        }
    }

    fn handle_command(&mut self, cmd: ClientNetCommand, source: NodeId) {
        match cmd {
            ClientNetCommand::RegistrationToChat(receiver_id) => {
                if receiver_id == self.info.host.id && !self.client_list.contains(&source) {
                    self.client_list.push(source);
                    self.send_logging(format!("Registered new client to chat: {}", source));
                    let history = self.chat_mem.get_history(self.info.host.id);
                    for ev in history {
                        self.send_response(ev, source);
                    }
                }
            }
            ClientNetCommand::MessageFor(destination, receiver_id, msg) => {
                if receiver_id == self.info.host.id && self.client_list.contains(&source) && self.client_list.contains(&destination) {
                    self.send_logging(format!("Sending message to {}: {}", destination, msg));
                    let ev = ServerToClient::MessageFrom(self.info.host.id, source, msg.clone());
                    self.send_response(ev.clone(), destination);
                    self.chat_mem.push_and_broadcast(self.info.host.id, ev);
                }
            }
            ClientNetCommand::ServerType => {
                if let Ok(server_type) = self.servertype() {
                    self.send_logging("Sending server type".to_string());
                    self.send_response(ServerToClient::ServerType(server_type), source);
                } else {
                    self.send_logging(" Failed to send ServerToClient::ServerType".to_string());
                }
            }
            ClientNetCommand::ClientList(receiver_id) => {
                match receiver_id == self.info.host.id {
                    true => {
                        self.send_response(ServerToClient::ClientList(self.info.host.id, self.client_list.clone()), source);
                        self.send_logging("Sending client list".to_string());
                    }
                    false => {
                        self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
                    }
                }
            }
            ClientNetCommand::FileFor(destination, receiver_id, file_id, extension, message) => {
                match receiver_id == self.info.host.id {
                    true => {
                        match self.check_if_in_client_list(source) {
                            true => {
                                match self.check_if_in_client_list(destination) {
                                    true => {
                                        self.send_logging(format!("Sending file to {}", destination));
                                        self.send_response(ServerToClient::FileFrom(self.info.host.id, source, file_id, extension, message), destination);
                                    }
                                    false => {
                                        self.send_logging(format!("{} is not registered to client chat", destination));
                                        self.send_response(ServerToClient::ErrClientId(self.info.host.id), destination);
                                    }
                                }
                            }
                            false => {
                                self.send_logging(" You're not registered to client chat".to_string());
                            }
                        }
                    }
                    false => {
                        self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
                    }
                }
            }
            ClientNetCommand::MediaFor(destination, receiver_id, media_id, extension, message) => {
                match receiver_id == self.info.host.id {
                    true => {
                        match self.check_if_in_client_list(source) {
                            true => {
                                match self.check_if_in_client_list(destination) {
                                    true => {
                                        self.send_logging(format!("Sending media to {}", destination));
                                        self.send_response(ServerToClient::MediaFrom(self.info.host.id, source, media_id, extension, message), destination);
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
                    false => { self.send_response(ServerToClient::WrongServer(self.info.host.id), source); }
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
