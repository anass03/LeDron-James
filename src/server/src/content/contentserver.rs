use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use std::time::Duration;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::network::NodeId;
use wg_2024::packet::{Ack, FloodRequest, Fragment, Nack, NodeType, Packet, PacketType};
use crate::{GenServerBehavior, MediaServer, TextServer, GenServer};
use host::{ServerToClient, ServerTypes, Functionality, HostEvent, HostCommand, ClientNetCommand, Image};
use delegate::delegate;

/// ContentServer implements both TextServer and MediaServer functionality.
/// It handles client requests for files and media, processes protocol packets,
/// and communicates with clients through source routing.
pub struct ContentServer {
    private_dir: TempDir,
    private_files_dir: PathBuf,
    private_images_dir: PathBuf,
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
        let private_dir = TempDir::new().expect("Failed to create private dir");
        let private_files_dir = private_dir.path().join("files");
        let private_images_dir = private_dir.path().join("images");

        fs::create_dir_all(&private_files_dir).expect("Failed to create private files dir");
        fs::create_dir_all(&private_images_dir).expect("Failed to create private images dir");

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
            private_dir,
            private_files_dir,
            private_images_dir,
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
                    match cmd{
                        Ok(command) => {
                            self.handle_sim_command(command);
                        }
                        Err(e) => {
                            self.send_logging(format!("Command not recognized: {}",e));
                        }
                    }
                }
                recv(self.info.host.packet_recv) -> pkt => {
                    match pkt{
                        Ok(packet) => {
                            self.handling_packets(packet);
                        }
                        Err(e) => {
                            self.send_logging(format!("Packet not recognized: {}",e));
                        }
                    }
                }
                default(Duration::from_millis(3)) => {}
            }
        }
    }
    /// Handles high-level client commands after full message reassembly.
    fn handle_command(&mut self, cmd: ClientNetCommand, source: NodeId) {
        match cmd {
            ClientNetCommand::ServerType => self.send_server_type(source),
            ClientNetCommand::FilesList(id) => self.handle_files_list(id, source),
            ClientNetCommand::File(id, file_id) => self.handle_file_request(id, file_id, source),
            ClientNetCommand::MediaList(id) => self.handle_media_list(id, source),
            ClientNetCommand::Media(id, media_id) => self.handle_media_request(id, media_id, source),
            ClientNetCommand::NewMedia(id, file_id, image) => self.handle_new_media(id, file_id, image, source),
            ClientNetCommand::NewFile(id, file_id, data) => self.handle_new_file(id, file_id, data, source),
            _ => self.send_response(ServerToClient::WrongServer(self.info.host.id), source),
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
    fn send_server_type(&mut self, source: NodeId) {
        match self.servertype() {
            Ok(server_type) => {
                self.send_logging("Sending server type".to_string());
                self.send_response(ServerToClient::ServerType(server_type), source);
            }
            Err(e) => {
                self.send_logging(format!("Error in getting ServerType {}", e));
            }
        }
    }
    fn handle_files_list(&mut self, receiver_id: NodeId, source: NodeId) {
        if receiver_id == self.info.host.id {
            let map = self.scan_text_files(&self.private_files_dir);
            self.file_map = map.clone();
            self.files = map.keys().cloned().collect();
            self.files.sort();
            self.send_logging("Checking file IDs".to_string());
            self.send_response(ServerToClient::FilesList(source, self.info.host.id, self.files.clone()), source);
        } else {
            self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
        }
    }
    fn handle_file_request(&mut self, receiver_id: NodeId, file_id: String, source: NodeId) {
        if receiver_id != self.info.host.id {
            self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
            return;
        }

        match self.file_map.get(&file_id) {
            Some(path) => match Self::convert_file_path(path) {
                Ok(data) => {
                    self.send_logging("Sending file".to_string());
                    self.send_response(ServerToClient::File(self.info.host.id, file_id, data), source);
                }
                Err(e) => {
                    self.send_response(ServerToClient::ErrNotFound(self.info.host.id), source);
                    self.send_logging(format!("Error converting file: {}", e));
                }
            },
            None => {
                self.send_response(ServerToClient::ErrNotFound(self.info.host.id), source);
            }
        }
    }
    fn handle_media_list(&mut self, receiver_id: NodeId, source: NodeId) {
        if receiver_id == self.info.host.id {
            let map = self.scan_media_files(&self.private_images_dir);
            self.media_map = map.clone();
            self.media = map.keys().cloned().collect();
            self.media.sort();
            self.send_logging("Checking media IDs".to_string());
            self.send_response(ServerToClient::MediaList(source, self.info.host.id, self.media.clone()), source);
        } else {
            self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
        }
    }
    fn handle_media_request(&mut self, receiver_id: NodeId, media_id: String, source: NodeId) {
        if receiver_id != self.info.host.id {
            self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
            return;
        }

        match self.media_map.get(&media_id) {
            Some(path) => match Self::process_image_path(path) {
                Ok(image) => {
                    self.send_logging(format!("Image {} loaded: {}x{}", media_id, image.width, image.height));
                    self.send_response(ServerToClient::Media(self.info.host.id, media_id, image), source);
                }
                Err(e) => {
                    self.send_response(ServerToClient::ErrNotFound(self.info.host.id), source);
                    self.send_logging(format!("Error processing image: {}", e));
                }
            },
            None => {
                self.send_response(ServerToClient::ErrNotFound(self.info.host.id), source);
            }
        }
    }

    fn handle_new_media(&mut self, receiver_id: NodeId, file_id: String, image: Image, source: NodeId) {
        if receiver_id != self.info.host.id {
            self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
            return;
        }

        let safe_file_id = Path::new(&file_id)
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("invalid_media"));
        let path = self.private_images_dir.join(safe_file_id);

        match Self::save_image_to(&path, &image) {
            Ok(_) => self.send_logging(format!("Media saved in {}", path.display())),
            Err(e) => self.send_logging(format!("Error writing media {}: {}", path.display(), e)),
        }
    }
    fn handle_new_file(&mut self, receiver_id: NodeId, file_id: String, data: Vec<u8>, source: NodeId) {
        if receiver_id != self.info.host.id {
            self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
            return;
        }

        let safe_file_id = Path::new(&file_id)
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("invalid_file"));
        let path = self.private_files_dir.join(safe_file_id);

        match Self::save_text_file(&path, &data) {
            Ok(_) => self.send_logging(format!("File saved in {}", path.display())),
            Err(e) => self.send_logging(format!("Error writing file {}: {}", path.display(), e)),
        }
    }
}

impl MediaServer for ContentServer {}
impl TextServer for ContentServer {}


