use std::collections::HashMap;
use std::{env, fs, io};
use std::error::Error;
use std::path::{Path, PathBuf};
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
            fn decompress_anything(&self, compressed: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
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
        let files_dir  = manifest_dir.join("src/content/files");
        let images_dir = manifest_dir.join("src/content/images");
        self.send_logging(format!(" Files directory: {}", files_dir.to_str().unwrap().to_string()));
        self.send_logging(format!(" Images directory: {}", images_dir.to_str().unwrap().to_string()));
        // il resto rimane invariato
        if let Ok(media) = self.list_media_ids(&images_dir) {
            self.media = media;
            self.send_logging(format!(" media -> {:?}", self.media));
        }
        if let Ok(files) = self.list_media_ids(&files_dir) {
            self.files = files;
            self.send_logging(format!(" files -> {:?}", self.files));
        }
    }

    
    pub fn list_media_ids<P: AsRef<Path>>(&self, folder_path: P) -> io::Result<Vec<usize>> {
        let mut ids = Vec::new();
        for entry in fs::read_dir(&folder_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(stem) = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                {
                    // Prendo solo i caratteri numerici dallo stem
                    let digits: String = stem.chars()
                        .filter(|c| c.is_ascii_digit())
                        .collect();
                    if !digits.is_empty() {
                        if let Ok(id) = digits.parse::<usize>() {
                            ids.push(id);
                        }
                    }
                }
            }
        }
        ids.sort_unstable();
        Ok(ids)
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
            ClientNetCommand::File(receiver_id,file_id) => {
                match receiver_id==self.info.host.id {
                    true => {                
                        match Self::convert_file(file_id) {
                        Ok((data, ext)) => {
                            self.send_logging("Sending file".to_string());
                            self.send_response(ServerToClient::File(self.info.host.id,file_id, ext.clone(), data), source);
                        }
                        Err(_) => {
                            self.send_response(ServerToClient::ErrNotFound(self.info.host.id), source);
                        }
                    }}
                    false => {
                        self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
                    }
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
            ClientNetCommand::Media(receiver_id,file_id) => {
                match receiver_id==self.info.host.id{
                    true => {match Self::process_image(file_id) {
                        Ok((image, ext)) => {
                            self.send_logging("Sending image".to_string());
                            self.send_response(ServerToClient::Media(self.info.host.id,file_id, ext.clone(), image), source);
                        }
                        Err(_) => {
                            self.send_response(ServerToClient::ErrNotFound(self.info.host.id), source);
                        }
                    }}
                    false => {
                        self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
                    } 
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