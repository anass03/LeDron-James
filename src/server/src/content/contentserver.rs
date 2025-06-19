use std::collections::HashMap;
use std::{env, fs};
use std::fmt::format;
use std::path::{Path, PathBuf};
use std::time::Duration;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::network::NodeId;
use wg_2024::packet::{Ack, FloodRequest, Fragment, Nack, NodeType, Packet, PacketType};
use crate::{GenServerBehavior, MediaServer, TextServer, GenServer};
use host::{ServerToClient, ServerTypes, Functionality, HostEvent, HostCommand, ClientNetCommand};
use delegate::delegate;

/// ContentServer implements both TextServer and MediaServer functionality.
/// It handles client requests for files and media, processes protocol packets,
/// and communicates with clients through source routing.
pub struct ContentServer {
    info: GenServer,                 // Internal GenServer used for common server logic
    files: Vec<String>,               // List of file IDs (text files)
    media: Vec<String>,               // List of media IDs (images)
    file_map: HashMap<String, PathBuf>,  // Map file ID -> file path
    media_map: HashMap<String, PathBuf>, // Map media ID -> media path
}

/// Trait defining the ContentServer behavior, composed by both MediaServer and TextServer.
pub trait ContentServerBehavior: MediaServer + TextServer {
    fn handling_packets(&mut self, packet: Packet);
}

impl ContentServer {
    delegate! {
        to self.info.host {
            /// Flood discovery logic (initialization and processing)
            fn initiate_flood(&mut self);
            fn process_flood_request(&mut self, flood_request: FloodRequest, packet: Packet);
            fn process_flood_response(&mut self, packet: Packet);

            /// Handles acknowledgements and negative acknowledgements for reliable delivery
            fn handle_ack(&mut self, ack: Ack, hops: &[NodeId], session_id: u64);
            fn handle_nack(&mut self, nack: Nack, nack_generator: NodeId, session_id: u64);
            fn send_logging(&self, message: String);
        }
        to self.info {
            /// Simulation controller interface
            fn send_response(&mut self, response: ServerToClient, destination: NodeId);
            fn handle_sim_command(&mut self, cmd: HostCommand);
            fn servertype(&mut self) -> Result<ServerTypes, String>;
            fn handle_msg_fragment(&mut self, packet: Packet, fragment: Fragment) -> Option<Vec<u8>>;
        }
    }

    /// Creates a new ContentServer instance.
    /// Validates that it connects to the proper number of drones as required by the protocol.
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

    /// Main server loop.
    /// Continuously listens for simulation commands and incoming packets,
    /// and processes them accordingly.
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
                default(Duration::from_millis(3)) => {}
            }
        }
    }
    /// Scans the file system to load available text and media files.
    /// Populates internal maps that associate file IDs to disk paths.
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

        // Scan text files
        if let Ok(entries) = fs::read_dir(&files_dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() {
                    if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                        let id = filename.to_string();
                        self.file_map.insert(id.clone(), path.clone());
                        self.files.push(id);
                    }
                }
            }
        }

        // Scan media files
        if let Ok(entries) = fs::read_dir(&images_dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() {
                    if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                        let id = filename.to_string();
                        self.media_map.insert(id.clone(), path.clone());
                        self.media.push(id);
                    }
                }
            }
        }

        self.files.sort();
        self.media.sort();
        self.send_logging(format!("Loaded {} text files and {} images.", self.files.len(), self.media.len()));
    }


    /// Handles high-level client commands after full message reassembly.
    fn handle_command(&mut self, cmd: ClientNetCommand, source: NodeId) {
        match cmd {
            ClientNetCommand::ServerType => {
                if let Ok(server_type) = self.servertype() {
                    self.send_logging("Sending server type".to_string());
                    self.send_response(ServerToClient::ServerType(server_type), source);
                } else {
                    self.send_logging("Failed to send ServerToClient::ServerType".to_string());
                }
            }
            ClientNetCommand::FilesList(receiver_id) => {
                if receiver_id == self.info.host.id {
                    self.count_files();
                    self.send_logging("Checking files ids".to_string());
                    self.send_response(ServerToClient::FilesList(self.info.host.id, self.files.clone()), source);
                } else {
                    self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
                }
            }
            ClientNetCommand::File(receiver_id, file_id) => {
                if receiver_id == self.info.host.id {
                    if let Some(path) = self.file_map.get(&file_id) {
                        match Self::convert_file_path(path) {
                            Ok(data) => {
                                self.send_logging("Sending file".to_string());
                                self.send_response(ServerToClient::File(self.info.host.id, file_id, data), source);
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
                if receiver_id == self.info.host.id {
                    self.count_files();
                    self.send_logging("Checking Medias ids".to_string());
                    self.send_response(ServerToClient::MediaList(self.info.host.id, self.media.clone()), source);
                } else {
                    self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
               }
            }
            ClientNetCommand::Media(receiver_id, file_id) => {
                if receiver_id == self.info.host.id {
                    if let Some(path) = self.media_map.get(&file_id) {
                        match Self::process_image_path(path) {
                            Ok(image) => {
                                self.send_logging(format!("Image {} loaded: {}x{}", file_id, image.width, image.height));
                                self.send_response(ServerToClient::Media(self.info.host.id, file_id, image), source);
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
            }/*
            ClientNetCommand::NewMedia(receiver_id, file_id, data) => {
                if receiver_id == self.info.host.id {
                    let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/content/images");
                    let safe_file_id = Path::new(&file_id)
                        .file_name()
                        .unwrap_or_else(|| std::ffi::OsStr::new("invalid_media"));
                    let path = base_dir.join(safe_file_id);

                    match fs::write(&path, &data) {
                        Ok(_) => {
                            self.send_logging(format!("Media salvato in {}", path.display()));
                            // self.send_response(ServerToClient::Ack, source); // da capire se aggiungere una risposta
                        },
                        Err(e) => {
                            self.send_logging(format!("Errore scrivendo media {}: {}", path.display(), e));
                            //self.send_response(ServerToClient::Nack, source); // da capire se aggiungere una risposta
                        }
                    }
                } else {
                    self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
                }
            }
            ClientNetCommand::NewFile(receiver_id, file_id, data) => {
                if receiver_id == self.info.host.id {
                    let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/content/files");
                    let safe_file_id = Path::new(&file_id)
                        .file_name()
                        .unwrap_or_else(|| std::ffi::OsStr::new("invalid_file"));
                    let path = base_dir.join(safe_file_id);

                    match fs::write(&path, &data) {
                        Ok(_) => {
                            self.send_logging(format!("File salvato in {}", path.display()));
                            //self.send_response(ServerToClient::Ack, source); // da capire se aggiungere una risposta
                        },
                        Err(e) => {
                            self.send_logging(format!("Errore scrivendo file {}: {}", path.display(), e));
                            //self.send_response(ServerToClient::Nack, source); // da capire se aggiungere una risposta
                        }
                    }
                } else {
                    self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
                }
            }*/
            _ => {
                self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
            }
        }
    }

    /// Handles any protocol packet received by the server.
    /// Distinguishes between fragments, flooding, ACK/NACK, and fully reassembled commands.
    fn handling_packets(&mut self, packet: Packet) {
        match packet.clone().pack_type {
            PacketType::Nack(nack) => {
                self.handle_nack(nack, packet.routing_header.hops[0], packet.session_id)
            }
            PacketType::Ack(ack) => {
                self.handle_ack(ack, &packet.routing_header.hops, packet.session_id)
            }
            PacketType::FloodResponse(_) => self.process_flood_response(packet),
            PacketType::FloodRequest(r) => self.process_flood_request(r, packet),
            PacketType::MsgFragment(fragment) => {
                if let Some(data) = self.handle_msg_fragment(packet.clone(), fragment) {
                    match serde_json::from_slice::<ClientNetCommand>(&data) {
                        Ok(cmd) => self.handle_command(cmd, packet.routing_header.hops[0]),
                        Err(e) => {
                            self.send_logging(format!("Failed to parse ClientNetCommand: {}", e));
                            self.send_response(ServerToClient::ErrNotFound(self.info.host.id), packet.routing_header.hops[0])
                        }
                    }
                } else {
                    self.send_logging("The packet is not complete; server is waiting.".to_string());
                }
            }
        }
    }
}

impl MediaServer for ContentServer {}
impl TextServer for ContentServer {}

