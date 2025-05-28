use std::collections::HashMap;
use std::{env, fs, io};
use std::path::{Path, PathBuf};
use std::time::Duration;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::network::NodeId;
use wg_2024::packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, Packet, PacketType};
use crate::{GenServerBehavior, MediaServer, TextServer, GenServer};
use host::{ClientCommand, ServerToClient, ServerTypes, generate_unique_session_id, Message, NetCommand, Functionality, HostEvent};
use delegate::delegate;
use regex::Regex;

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

    pub fn count_files(&mut self) {
        // 1) prendo il percorso del crate corrente, es. ".../mio-progetto/src"
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // 2) risalgo di un livello fino alla radice del workspace/progetto
        let project_root = crate_dir
            .parent()
            .expect("Impossibile risalire al project root");

        // 3) scendo dentro server/src/images e server/src/files
        let images_dir = project_root.join("server").join("src").join("images");
        let files_dir  = project_root.join("server").join("src").join("files");

        // il resto rimane invariato
        if let Ok(files) = self.list_media_ids(&images_dir) {
            self.media = files;
            println!("media -> {:?}", self.media);
        }
        if let Ok(files) = self.list_media_ids(&files_dir) {
            self.files = files;
            println!("files -> {:?}", self.files);
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
                // self.count_files();
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
                // self.count_files();
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
fn main() {
    // let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // 
    // let images_dir = crate_root.join("src").join("images");
    // let files_dir  = crate_root.join("src").join("files");
    // 
    // match ContentServer::list_media_ids(&images_dir) {
    //     Ok(ids) => {
    //         println!("Trovati {} file in {:?}: {:?}", ids.len(), images_dir, ids);
    //         // self.media = ids;
    //     }
    //     Err(e) => {
    //         eprintln!("Errore leggendo {:?}: {}", images_dir, e);
    //     }
    // }
    // 
    // match ContentServer::list_media_ids(&files_dir) {
    //     Ok(ids) => {
    //         println!("Trovati {} file in {:?}: {:?}", ids.len(), files_dir, ids);
    //         // self.files = ids;
    //     }
    //     Err(e) => {
    //         eprintln!("Errore leggendo {:?}: {}", files_dir, e);
    //     }
    // }

}
