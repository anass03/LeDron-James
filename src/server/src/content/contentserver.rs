use std::{collections::HashMap, fs, path::{Path, PathBuf}, time::Duration};
use tempfile::TempDir;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::network::NodeId;
use wg_2024::packet::{Ack, FloodRequest, Fragment, Nack, NodeType, Packet, PacketType};
use crate::{delegate_to_host_mut, delegate_to_host_ref, GenServer, MediaServer, TextServer};
use host::{ServerToClient, ServerTypes, HostCommand, HostEvent, Image, ClientNetCommand};
use host::functionality::Functionality;
use delegate::delegate;
use host::flood::FloodControl;
use host::logging::{LogControl, LogEntry, LogModule, LogType};

/// A content server that integrates both text and media server capabilities.
pub struct ContentServer {
    private_dir: TempDir,
    private_files_dir: PathBuf,
    private_images_dir: PathBuf,
    info: GenServer,
    files: Vec<String>,
    media: Vec<String>,
    file_map: HashMap<String, PathBuf>,
    media_map: HashMap<String, PathBuf>,
}

impl ContentServer {
    delegate_to_host_ref!(info => {
        pub fn send_logging(&self, log_type: LogType, module: LogModule, msg: String);
    });
    delegate_to_host_mut!(info => {
        fn initiate_flood(&mut self);
        fn handle_ack(&mut self, ack: Ack, hops: &[NodeId], session_id: u64);
        fn handle_nack(&mut self, nack: Nack, generator: NodeId, session_id: u64);
        fn process_flood_request(&mut self, flood_request: FloodRequest, packet: Packet);
        fn process_flood_response(&mut self, packet: Packet);
    });
    delegate! {
        to self.info {
            fn send_response(&mut self, response: ServerToClient, destination: NodeId);
            fn handle_sim_command(&mut self, cmd: HostCommand);
            fn servertype(&mut self) -> Result<ServerTypes, String>;
            fn handle_msg_fragment(&mut self, packet: Packet, fragment: Fragment) -> Option<Vec<u8>>;
        }
    }

    /// Creates a new `ContentServer` with isolated temporary directories.
    pub fn new(
        id: NodeId,
        type_host: NodeType,
        connected_drone_ids: Vec<NodeId>,
        controller_send: Sender<HostEvent>,
        controller_recv: Receiver<HostCommand>,
        packet_recv: Receiver<Packet>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        log_sender: Sender<LogEntry>,
    ) -> Result<Self, std::io::Error> {
        let private_dir = TempDir::new()?;
        let private_files_dir = private_dir.path().join("files");
        let private_images_dir = private_dir.path().join("images");

        fs::create_dir_all(&private_files_dir)?;
        fs::create_dir_all(&private_images_dir)?;

        Ok(Self {
            info: GenServer::new(
                id,
                type_host,
                connected_drone_ids,
                controller_send,
                controller_recv,
                packet_recv,
                packet_send,
                ServerTypes::Content,
                log_sender,
            ),
            private_dir,
            private_files_dir,
            private_images_dir,
            files: Vec::new(),
            media: Vec::new(),
            file_map: HashMap::new(),
            media_map: HashMap::new(),
        })
    }

    /// Starts the main loop of the ContentServer.
    pub fn run(&mut self){
        self.initiate_flood();
        self.send_logging(LogType::INFO,LogModule::MEDIA,"Starting Content Server...".into());
        loop {
            crossbeam_channel::select_biased! {
                recv(self.info.controller_recv) -> cmd => match cmd {
                    Ok(c) => self.handle_sim_command(c),
                    Err(e) => self.send_logging(LogType::ERROR,LogModule::PACKET,format!("Command error: {e}")),
                },
                recv(self.info.host.packet_recv) -> pkt => match pkt {
                    Ok(p) => self.handling_packets(p),
                    Err(e) => self.send_logging(LogType::ERROR,LogModule::PACKET,format!("Packet error: {e}")),
                },
                default(Duration::from_millis(3)) => {}
            }
        }
    }

    /// Processes incoming packets according to their type.
    fn handling_packets(&mut self, packet: Packet) {
        use PacketType::*;
        match packet.clone().pack_type {
            Nack(nack) => self.handle_nack(nack, packet.routing_header.hops[0], packet.session_id),
            Ack(ack) => self.handle_ack(ack, &packet.routing_header.hops, packet.session_id),
            FloodResponse(_) => self.process_flood_response(packet),
            FloodRequest(r) => self.process_flood_request(r, packet),
            MsgFragment(fragment) => {
                if let Some(data) = self.handle_msg_fragment(packet.clone(), fragment) {
                    match serde_json::from_slice::<ClientNetCommand>(&data) {
                        Ok(cmd) => self.handle_command(cmd, packet.routing_header.hops[0]),
                        Err(e) => {
                            self.send_logging(LogType::ERROR,LogModule::PACKET,format!("Failed to parse ClientNetCommand: {}", e));
                            self.send_response(ServerToClient::ErrNotFound(self.info.host.id), packet.routing_header.hops[0]);
                        }
                    }
                } else {
                    self.send_logging(LogType::WARN,LogModule::PACKET,"The packet is not complete; server is waiting.".to_string());
                }
            }
        }
    }

    /// Delegates execution based on the `ClientNetCommand` received from the client.
    fn handle_command(&mut self, cmd: ClientNetCommand, source: NodeId) {
        use ClientNetCommand::*;
        match cmd {
            ServerType => self.send_server_type(source),
            FilesList(id) => self.handle_files_list(id, source),
            File(id, file_id) => self.handle_file_request(id, file_id, source),
            MediaList(id) => self.handle_media_list(id, source),
            Media(id, media_id) => self.handle_media_request(id, media_id, source),
            NewMedia(id, file_id, image) => self.handle_new_media(id, file_id, image, source),
            NewFile(id, file_id, data) => self.handle_new_file(id, file_id, data, source),
            _ => self.send_response(ServerToClient::WrongServer(self.info.host.id), source),
        }
    }

    /// Sends the type of the server back to the client.
    fn send_server_type(&mut self, source: NodeId) {
        match self.servertype() {
            Ok(server_type) => {
                //self.send_logging("Sending server type".to_string());
                self.send_response(ServerToClient::ServerType(server_type), source);
            }
            Err(e) => self.send_logging(LogType::ERROR,LogModule::PACKET,format!("Error in getting ServerType: {}", e)),
        }
    }

    /// Responds to a request for the list of text files.
    fn handle_files_list(&mut self, receiver_id: NodeId, source: NodeId) {
        if receiver_id == self.info.host.id {
            self.file_map = self.scan_text_files(&self.private_files_dir);
            self.files = self.file_map.keys().cloned().collect();
            self.files.sort();
            //self.send_logging("Checking file IDs".to_string());
            self.send_response(ServerToClient::FilesList(source, self.info.host.id, self.files.clone()), source);
        } else {
            self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
        }
    }

    /// Responds with the requested file contents.
    fn handle_file_request(&mut self, receiver_id: NodeId, file_id: String, source: NodeId) {
        if !self.is_right_receiver(receiver_id) {
            return;
        }
        match self.file_map.get(&file_id) {
            Some(path) => match Self::convert_file_path(path) {
                Ok(data) => {
                    //self.send_logging("Sending file".to_string());
                    self.send_response(ServerToClient::File(self.info.host.id, file_id, data), source);
                }
                Err(e) => {
                    self.send_response(ServerToClient::ErrNotFound(self.info.host.id), source);
                    self.send_logging(LogType::ERROR,LogModule::FILE,format!("Error converting file: {}", e));
                }
            },
            None => self.send_response(ServerToClient::ErrNotFound(self.info.host.id), source),
        }
    }

    /// Responds to a request for the list of media files.
    fn handle_media_list(&mut self, receiver_id: NodeId, source: NodeId) {
        if receiver_id == self.info.host.id {
            self.media_map = self.scan_media_files(&self.private_images_dir);
            self.send_logging(LogType::INFO,LogModule::MEDIA,format!(" media map , {:?}" , self.media_map.clone()));
            self.media = self.media_map.keys().cloned().collect();
            self.media.sort();
            //self.send_logging("Checking media IDs".to_string());
            self.send_response(ServerToClient::MediaList(source, self.info.host.id, self.media.clone()), source);
        } else {
            self.send_response(ServerToClient::WrongServer(self.info.host.id), source);
        }
    }

    /// Responds with the requested media file (image).
    fn handle_media_request(&mut self, receiver_id: NodeId, media_id: String, source: NodeId) {
        if !self.is_right_receiver(receiver_id) {
            return;
        }
        match self.media_map.get(&media_id) {
            Some(path) => match Self::process_image_path(path) {
                Ok(image) => {
                    //self.send_logging(format!("Image {} loaded: {}x{}", media_id, image.width, image.height));
                    self.send_response(ServerToClient::Media(self.info.host.id, media_id, image), source);
                }
                Err(e) => {
                    self.send_response(ServerToClient::ErrNotFound(self.info.host.id), source);
                    self.send_logging(LogType::ERROR,LogModule::MEDIA,format!("Error processing image: {}", e));
                }
            },
            None => self.send_response(ServerToClient::ErrNotFound(self.info.host.id), source),
        }
    }

    /// Handles a new media file upload from a client.
    pub fn handle_new_media(&mut self, receiver_id: NodeId, file_id: String, image: Image, source: NodeId) {
        if !self.is_right_receiver(receiver_id) {
            return;
        }

        let ext_opt = Path::new(&file_id)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase());

        match ext_opt.as_deref() {
            Some("png") | Some("jpg") | Some("jpeg") => {
                let path = self.private_images_dir.join(
                    Path::new(&file_id)
                        .file_name()
                        .unwrap_or_else(|| std::ffi::OsStr::new("invalid_media")),
                );
                match Self::save_image_to(&path, &image) {
                    Ok(_) => {
                        self.send_logging(LogType::INFO,LogModule::MEDIA,format!("Media saved in {}", path.display()))
                    },
                    Err(e) => self.send_logging(LogType::ERROR,LogModule::MEDIA,format!("Error writing media {}: {}", path.display(), e)),
                }
            }
            Some("txt") | Some("md") | Some("json") | Some("toml") | Some("pdf") => {
                //self.send_logging(format!( "Detected text file extension '{}', delegating to handle_new_file.", ext_opt.unwrap()));
                self.handle_new_file(receiver_id, file_id, image.data.clone(), source);
            }
            Some(ext) => {
                self.send_logging(LogType::ERROR,LogModule::FILE,format!("Unsupported file extension '{}', skipping save.", ext));
            }
            None => {
                self.send_logging(LogType::ERROR,LogModule::FILE,"No extension found in file_id, skipping save.".to_string());
            }
        }
    }

    /// Handles a new text or image file upload from a client.
    pub fn handle_new_file(&mut self, receiver_id: NodeId, file_id: String, data: Vec<u8>, source: NodeId) {
        if !self.is_right_receiver(receiver_id) {
            return;
        }

        let ext_opt = Path::new(&file_id)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase());

        match ext_opt.as_deref() {
            Some("png") | Some("jpg") | Some("jpeg") => {
                //self.send_logging(format!("Detected image extension '{}', delegating to handle_new_media.", ext_opt.unwrap()));
                match image::load_from_memory(&data) {
                    Ok(img) => {
                        let image = Image {
                            width: img.width(),
                            height: img.height(),
                            data: img.to_rgba8().to_vec(),
                        };
                        self.handle_new_media(receiver_id, file_id, image, source);
                    }
                    Err(e) => {
                        self.send_logging(LogType::ERROR,LogModule::MEDIA,format!("Error decoding image data: {}", e));
                        self.send_response(ServerToClient::ErrNotFound(self.info.host.id), source);
                    }
                }
            }
            Some("txt") | Some("md") | Some("json") | Some("toml") | Some("pdf") => {
                let path = self.private_files_dir.join(
                    Path::new(&file_id)
                        .file_name()
                        .unwrap_or_else(|| std::ffi::OsStr::new("invalid_file")),
                );
                match Self::save_text_file(&path, &data) {
                    Ok(_) => {
                        //self.send_logging(format!("File saved in {}", path.display())),
                    }
                    Err(e) => self.send_logging(LogType::ERROR,LogModule::FILE,format!("Error writing file {}: {}", path.display(), e)),
                }
            }
            Some(ext) => {
                self.send_logging(LogType::ERROR,LogModule::FILE,format!(
                    "Unsupported file extension '{}', skipping save.",
                    ext
                ));
            }
            None => {
                self.send_logging(LogType::ERROR,LogModule::FILE,"No extension found in file_id, skipping save.".to_string());
            }
        }
    }

    /// Checks whether the packet is intended for this server.
    fn is_right_receiver(&self, receiver: NodeId) -> bool {
        self.info.host.id == receiver
    }
}

impl MediaServer for ContentServer {}
impl TextServer for ContentServer {}