use std::collections::HashMap;
use std::{env, fs};
use std::error::Error;
use std::path::{PathBuf};
use std::time::Duration;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::network::NodeId;
use wg_2024::packet::{Ack, FloodRequest, Fragment, Nack, NodeType, Packet, PacketType};
use crate::{GenServerBehavior, MediaServer, TextServer, GenServer};
use host::{ServerToClient, ServerTypes, generate_unique_session_id, Message, Functionality, HostEvent, HostCommand,ClientNetCommand};
use delegate::delegate;


pub struct ContentServer {
    info: GenServer,
    files: Vec<usize>,
    media: Vec<usize>,
    file_map: HashMap<usize, PathBuf>,
    media_map: HashMap<usize, PathBuf>,  
}

pub trait ContentServerBehavior: MediaServer + TextServer {
    fn handling_packets(&mut self, packet: Packet);
}

impl ContentServer {
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
                ServerTypes::Content,
                log_sender
            ),
            files: vec![],
            media: vec![],
            file_map: Default::default(),
            media_map: Default::default(),
        }
    }

    
    pub fn run(&mut self) {
        self.initiate_flood();
        self.send_logging("Starting Content Server...".to_string());
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


    pub fn count_files(&mut self) {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let files_dir = manifest_dir.join("src/content/files");
        let images_dir = manifest_dir.join("src/content/images");

        self.send_logging(format!("Scanning files: {}", files_dir.display()));
        self.send_logging(format!("Scanning images: {}", images_dir.display()));

        self.file_map.clear();
        self.media_map.clear();
        self.files.clear();
        self.media.clear();

        if let Ok(entries) = fs::read_dir(&files_dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        let digits: String = stem.chars().filter(|c| c.is_ascii_digit()).collect();
                        if let Ok(id) = digits.parse::<usize>() {
                            self.file_map.insert(id, path.clone());
                            self.files.push(id);
                        }
                    }
                }
            }
        }

        if let Ok(entries) = fs::read_dir(&images_dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        let digits: String = stem.chars().filter(|c| c.is_ascii_digit()).collect();
                        if let Ok(id) = digits.parse::<usize>() {
                            self.media_map.insert(id, path.clone());
                            self.media.push(id);
                        }
                    }
                }
            }
        }

        self.files.sort_unstable();
        self.media.sort_unstable();
    }
    fn handle_command(&mut self, cmd: ClientNetCommand, source: NodeId) {
        match cmd {
            ClientNetCommand::ServerType => {
                if let Ok(server_type)=self.servertype(){
                    self.send_logging("Sending server type".to_string());
                    self.send_response(ServerToClient::ServerType(server_type),source);
                }else{
                    self.send_logging("Failed to send ServerToClient::ServerType".to_string());
                }
            }
            ClientNetCommand::FilesList(receiver_id) => { 
                match receiver_id==self.info.host.id{
                    true => {                 
                        self.count_files();
                        self.send_logging("Checking files ids".to_string());
                        self.send_response(ServerToClient::FilesList(self.info.host.id,self.files.clone()), source);
                    }
                    false => {
                        self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
                    } 
                }
            }
            ClientNetCommand::File(receiver_id, file_id) => {
                if receiver_id == self.info.host.id {
                    if let Some(path) = self.file_map.get(&file_id) {
                        match Self::convert_file_path(path) {
                            Ok((data, ext)) => {
                                self.send_logging("Sending file".to_string());
                                self.send_response(ServerToClient::File(self.info.host.id, file_id, ext, data), source);
                            }
                            Err(_) => {
                                self.send_response(ServerToClient::ErrNotFound(self.info.host.id), source);
                            }
                        }
                    } else {
                        self.send_response(ServerToClient::ErrNotFound(self.info.host.id), source);
                    }
                } else {
                    self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
                }
            }
            ClientNetCommand::MediaList(receiver_id) => {
                match receiver_id==self.info.host.id{
                    true => {
                        self.count_files();
                        self.send_logging("Checking Medias ids".to_string());
                        self.send_response(ServerToClient::MediaList(self.info.host.id,self.media.clone()), source);}
                    false => {
                        self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
                    }
                }
            }
            ClientNetCommand::Media(receiver_id, file_id) => {
                if receiver_id == self.info.host.id {
                    if let Some(path) = self.media_map.get(&file_id) {
                        match Self::process_image_path(path) {
                            Ok((image, ext)) => {
                                self.send_logging(format!("Image {} loaded: {}x{}", file_id, image.width, image.height));
                                self.send_response(ServerToClient::Media(self.info.host.id, file_id, ext, image), source);
                            }
                            Err(_) => {
                                self.send_response(ServerToClient::ErrNotFound(self.info.host.id), source);
                            }
                        }
                    } else {
                        self.send_response(ServerToClient::ErrNotFound(self.info.host.id), source);
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
    
    fn handling_packets(&mut self, packet: Packet) {
        match packet.clone().pack_type {
            PacketType::Nack(nack) => {
                self.handle_nack(nack, packet.routing_header.hops[0], packet.session_id)
            }
            PacketType::Ack(ack) => self.handle_ack(ack, &*packet.routing_header.hops, packet.session_id),
            PacketType::FloodResponse(_) => self.process_flood_response(packet),
            PacketType::FloodRequest(r) => self.process_flood_request(r, packet),
            PacketType::MsgFragment(fragment) => {
                if let Some(data) = self.handle_msg_fragment(packet.clone(), fragment) {
                    match serde_json::from_slice::<ClientNetCommand>(&data) {
                        Ok(cmd) => self.handle_command(cmd, packet.routing_header.hops[0]),
                        Err(_) => self.send_response(ServerToClient::ErrNotFound(self.info.host.id), packet.routing_header.hops[0]),
                    }
                }else {
                    self.send_logging("The packet is not complete and server is waiting".to_string());
                }
            }
        }
    }
}

impl MediaServer for ContentServer {}
impl TextServer for ContentServer {}


#[test]
fn main() {
}