use std::collections::HashMap;
use std::{fs, thread};
use crossbeam_channel::{Receiver, Sender};
use wg_2024::network::NodeId;
use wg_2024::packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, Packet};
use crate::{GenServerBehavior, MediaServer, TextServer, GenServer};
use host::{ClientCommand, ServerToClient, ServerTypes, generate_unique_session_id, Message, NetCommand, Functionality, HostEvent};
use delegate::delegate;

pub struct ContentServer {
    info: GenServer,
    files: Vec<usize>,
    media: Vec<usize>,
    sessions: HashMap<usize, (Receiver<ServerTypes>, Sender<ServerToClient>)>,
    command_receiver: Receiver<ClientCommand>,
    command_sender: Sender<ClientCommand>,
}

impl ContentServer {
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
            fn servertype(&mut self, request: ClientCommand, destination: NodeId);
            fn handle_msg_fragment(&mut self, packet: Packet, fragment: Fragment) -> Option<Vec<u8>>;
        }
    }
    fn new(info: GenServer) -> Self {
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
        ContentServer {
            info,
            files: Vec::new(),
            media: Vec::new(),
            sessions: HashMap::new(),
            command_receiver: cmd_rx,
            command_sender: cmd_tx,
        }
    }

    fn start(&mut self) {
        let receiver = self.command_receiver.clone();
        thread::spawn(move || loop {
            if let Ok(cmd) = receiver.recv() {
                // gestione del comando
            }
        });
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

    /// Raccoglie gli ID dei file (nomi numerici senza estensione) nella cartella specificata.
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
    fn count_files(&mut self) {
        self.media = self.count_media("https://raw.githubusercontent.com/leooon3/LeDron-James/tree/server/src/server/images/".into());
        self.files = self.count_media("https://raw.githubusercontent.com/leooon3/LeDron-James/tree/server/src/server/files/".into());
    }

    fn handle_command(&mut self, cmd: ClientCommand, destination: NodeId) {
        match cmd {
            ClientCommand::ServerType => {
                self.send_response(ServerToClient::ServerType(ServerTypes::Content), destination);
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

impl MediaServer for ContentServer {}
impl TextServer for ContentServer {}

#[test]
fn main() {}
