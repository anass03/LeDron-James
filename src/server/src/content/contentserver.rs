use std::collections::HashMap;
use std::{fs, thread};
use std::time::Duration;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::network::NodeId;
use wg_2024::packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, Packet, PacketType};
use crate::{GenServerBehavior, MediaServer, TextServer, GenServer};
use host::{ClientCommand, ServerToClient, ServerTypes, generate_unique_session_id, Message, NetCommand, Functionality, HostEvent};
use delegate::delegate;
use crate::utils::handle_server_control_loop;

pub struct ContentServer {
    info: GenServer,
    files: Vec<usize>,
    media: Vec<usize>,
}

pub trait ContentServerBehavior: MediaServer + TextServer {
    fn handling_packets(&mut self, packet: Result<Packet, crossbeam_channel::RecvError>);
}

impl ContentServer {
    delegate! {
        to self.info.host {
            //fn log<S: AsRef<str>>(&self, message: S);
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
        type_host: wg_2024::packet::NodeType,
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
                ServerTypes::Content,
                log_sender
            ),
            files: Vec::new(),
            media: Vec::new(),
        }
    }

    pub fn run(&mut self) {
        self.initiate_flood();
        self.send_logging("Starting Content Server...".to_string());
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
                    self.send_logging(format!("Failed to send response to {}: {}", destination, e));
                }
            }
            Err(e) => {
                self.send_logging(format!("Failed to serialize ServerToClient: {}", e));
            }
        }
    }

    fn count_media(&self, folder_path: String) -> Vec<usize> {
        match fs::read_dir(folder_path) {
            Ok(entries) => entries
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| {
                    let path = entry.path();
                    if path.is_file() {
                        path.file_name()
                            .and_then(|name| name.to_str()
                                .map(|s| s.parse::<usize>().ok()))
                            .flatten()
                    } else {
                        None
                    }
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Aggiorna `media` e `files` con i vettori di ID (Vec<usize>)
    fn count_files(&mut self) {
        self.media = self.count_media("https://raw.githubusercontent.com/leooon3/LeDron-James/tree/server/src/server/images/".into());
        self.files = self.count_media("https://raw.githubusercontent.com/leooon3/LeDron-James/tree/server/src/server/files/".into());
    }
    fn handle_command(&mut self, cmd: ClientCommand, destination: NodeId) {
        match cmd {
            ClientCommand::ServerType => {
                if let Ok(server_type)=self.servertype(){
                    self.send_response(ServerToClient::ServerType(server_type),destination);
                }else{
                    self.send_logging(format!("Failed to send ServerToClient::ServerType"));
                }
            }
            ClientCommand::FilesList => {
                self.count_files();
                self.send_response(ServerToClient::FilesList(self.files.clone()), destination);
            }
            ClientCommand::File(id) => {
                if let Ok(data) = Self::convert_file(id.to_string()) {
                    self.send_response(ServerToClient::File(id, data), destination);
                } else {
                    self.send_response(ServerToClient::ErrNotFound, destination);
                }
            }
            ClientCommand::MediaList => {
                self.count_files();
                self.send_response(ServerToClient::MediaList(self.media.clone()), destination);
            }
            ClientCommand::Media(id) => {
                if let Ok(image) = Self::process_image(id.to_string()) {
                    self.send_response(ServerToClient::Media(id, image), destination);
                } else {
                    self.send_response(ServerToClient::ErrNotFound, destination);
                }
            }
            _ => {
                self.send_response(ServerToClient::ErrNotFound, destination);
            }
        }
    }
}

impl ContentServerBehavior for ContentServer {
    fn handling_packets(&mut self, packet: Result<Packet, crossbeam_channel::RecvError>) {
        if let Ok(packet) = packet {
            match packet.clone().pack_type {
                PacketType::Nack(nack) => {
                    self.handle_nack(nack, packet.routing_header.hops[0], packet.session_id)
                }
                PacketType::Ack(ack) => self.handle_ack(ack, packet.session_id),
                PacketType::FloodResponse(r) => self.process_flood_response(r),
                PacketType::FloodRequest(r) => self.process_flood_request(r, packet),
                PacketType::MsgFragment(fragment) => {
                    if let Some(data) = self.handle_msg_fragment(packet.clone(), fragment) {
                        match serde_json::from_slice::<ClientCommand>(&data) {
                            Ok(cmd) => self.handle_command(cmd, packet.routing_header.hops[0]),
                            Err(_) => self.send_response(ServerToClient::ErrNotFound, packet.routing_header.hops[0]),
                        }
                    }
                }
            }
        }
    }
}

impl MediaServer for ContentServer {}
impl TextServer for ContentServer {}

#[test]
fn main() {}