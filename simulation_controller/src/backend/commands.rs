use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::network::NodeId;
use wg_2024::packet::Packet;
use host::{ClientId, ClientNetCommand, ContentID, FileId, HostCommand, HostEvent, Image, MediaId, ServerId, ServerToClient};
use client::ClientCommand;
use client::ClientCommand::*;
use host::ClientNetCommand::*;
use image::{DynamicImage, ImageBuffer, ImageReader, RgbImage, Rgba};
use crate::backend::simulation_controller::SimulationControllerData;
use crate::backend::sc_models::{NetNode, NodeCommand, NodeType};
use crate::backend::sc_models::{LogType, ChatList, NetCacheMedia, RendererResponse, FileRendererUpdate, RendererCommand};
use host::ServerTypes::*;
use petgraph::visit::Topo;
use ratatui_image::thread::ResizeRequest;
use server::chat::Chat;
use server::MediaServerError;
use crate::frontend::app::topology_algorithm::alg_models::{DisplayedNode, TopologyAlgorithm, TopologyStatus};
use crate::frontend::app::topology_algorithm::vectors::Vector;
use log::*;

impl SimulationControllerData {
    /// - Drone Commands
    pub fn crash(&mut self, drone_id: NodeId) -> Result<(), String> {
        match self.cannot_crash(drone_id) {
            Ok(cannot_crash) => {
                if cannot_crash {
                    return Err(format!("Drone {drone_id} cannot be crashed because it's critical!"))
                }
            }
            Err(e) => return Err(e)
        }
        debug!(target: "main", "Trying to Crash Drone: {drone_id}");
        let command = DroneCommand::Crash;
        match self.drone_command_sender.get(&drone_id) {
            Some(drone_command_sender) => {
                match drone_command_sender.send(command) {
                    Ok(_) => {
                        //removing all neighboring channels
                        let mut neighbors = vec![];
                        for drone in self.drones.iter() {
                            if drone.id == drone_id{
                                for neighbor in drone.connected_node_ids.iter() {
                                    neighbors.push(*neighbor);
                                }
                            }
                        }
                        if neighbors.is_empty() { return Err(format!("Drone {drone_id} does not exist")); }

                        for neighbor in neighbors.iter() {
                            let forward_node = match NetNode::from(&self, drone_id) {
                                Some(node) => node,
                                None => return Err(format!("Node {drone_id} not found")),
                            };
                            let backwards_node = match NetNode::from(&self, *neighbor){
                                Some(node) => node,
                                None => return Err(format!("Node {neighbor} not found")),
                            };

                            self.mono_remove_sender(forward_node, backwards_node)?;
                            self.mono_remove_sender(backwards_node, forward_node)?
                        }

                        self.drones.retain(|d| d.id != drone_id);
                        self.drone_command_sender.remove(&drone_id);
                        self.packet_sender.remove(&drone_id);
                        self.stats.total_active_connections -= 1;

                        Ok(())
                    },
                    Err(e) => Err(format!("Error sending crash command - {}", e.to_string())),
                }
            },
            None => Err(format!("Drone {drone_id} command sender not found"))
        }
    }

    pub fn set_pdr(&mut self, drone_id: NodeId, new_pdr: f32) -> Result<(), String> {
        let valid_pdr;
        if new_pdr >= 0.0 && new_pdr <= 1.0 {
            valid_pdr = new_pdr;
        } else {
            return Err(format!("Invalid PDR value: {} | 0.0 < PDR < 1.0", new_pdr));
        }
        let command = DroneCommand::SetPacketDropRate(valid_pdr);
        match self.drone_command_sender.get(&drone_id) {
            Some(drone_command_sender) => {
                match drone_command_sender.send(command) {
                    Ok(()) => {
                        info!(target: "main", "Changing Drone {drone_id} PDR to: {valid_pdr}");
                        Ok(()) },
                    Err(e) => Err(e.to_string()),
                }
            },
            None => Err(format!("Drone {drone_id} command sender not found")),
        }
    }

    /// - Net Commands
    fn mono_add_sender(&mut self, first_node: NetNode, second_node: NetNode) -> Result<(), String> {
        //checking if packet sender for forward node exists
        match self.packet_sender.get(second_node.id()) {
            Some(forward_packet_sender) => {
                //checking if packet sender for backwards node exists
                match first_node {
                    NetNode::Drone(first_id) => {
                        match self.drone_command_sender.get(first_node.id()) {
                            Some(forward_drone_command_sender) => {
                                let forward_command = DroneCommand::AddSender(*second_node.id(), forward_packet_sender.clone());
                                //trying to send command forward
                                forward_node_command(forward_drone_command_sender, forward_command, first_id, first_node.node_type(), *second_node.id(), second_node.node_type())?

                            },
                            None => return Err(format!("Drone {first_id} command sender not found")),
                        }
                        for drone in self.drones.iter_mut() {
                            if drone.id == first_id {
                                drone.connected_node_ids.push(*second_node.id());
                            }
                        }
                    },
                    NetNode::Client(first_id) => {
                        match self.client_command_sender.get(&first_id) {
                            Some(forward_client_command_sender) => {
                                let forward_command = Internal(HostCommand::AddSender(*second_node.id(), forward_packet_sender.clone()));
                                //trying to send command forward
                                forward_node_command(forward_client_command_sender, forward_command, first_id, first_node.node_type(), *second_node.id(), second_node.node_type())?

                            },
                            None => return Err(format!("Client {first_id} command sender not found")),
                        }
                        for client in self.clients.iter_mut() {
                            if client.id == first_id {
                                client.connected_drone_ids.push(*second_node.id());
                            }
                        }
                    },
                    NetNode::Server(first_id) => {
                        match self.server_command_sender.get(&first_id) {
                            Some(forward_server_command_sender) => {
                                let forward_command = HostCommand::AddSender(*second_node.id(), forward_packet_sender.clone());
                                //trying to send command forward
                                forward_node_command(forward_server_command_sender, forward_command, first_id, first_node.node_type(), *second_node.id(), second_node.node_type())?
                            },
                            None => return Err(format!("Server {first_id} command sender not found")),
                        }
                        for server in self.servers.iter_mut() {
                            if server.id == first_id {
                                server.connected_drone_ids.push(*second_node.id());
                            }
                        }
                    }
                }
            },
            None => return Err(format!("{} {} packet sender not found", second_node.node_type(), second_node.id())),
        }

        Ok(())
    }

    pub fn add_sender(&mut self, node_id: NodeId, sender_id: NodeId) -> Result<(), String> {
        let forward_node = match NetNode::from(&self, node_id) {
            Some(node) => node,
            None => return Err(format!("Node {node_id} not found")),
        };
        let backwards_node = match NetNode::from(&self, sender_id){
            Some(node) => node,
            None => return Err(format!("Node {sender_id} not found")),
        };

        match self.cannot_add(forward_node, backwards_node) {
            Ok(critical) => {
                if critical {
                    return Err(format!("Connection between {} {node_id} and {} {sender_id} is not possible!", forward_node.node_type(), backwards_node.node_type()))
                }
            }
            Err(e) => return Err(e),
        }

        self.mono_add_sender(forward_node, backwards_node)?;
        self.mono_add_sender(backwards_node, forward_node)
    }

    fn mono_remove_sender(&mut self, first_node: NetNode, second_node: NetNode) -> Result<(), String> {
        match first_node {
            NetNode::Drone(first_id) => {
                match self.drone_command_sender.get(&first_id) {
                    Some(forward_drone_command_sender) => {
                        let forward_command = DroneCommand::RemoveSender(*second_node.id());
                        //trying to send command forward
                        forward_node_command(forward_drone_command_sender, forward_command, first_id, first_node.node_type(), *second_node.id(), second_node.node_type())?

                    },
                    None => return Err(format!("Drone {first_id} command sender not found")),
                }
                for drone in self.drones.iter_mut() {
                    if drone.id == first_id {
                        drone.connected_node_ids.retain(|&id| id != *second_node.id());
                    }
                }
            }
            NetNode::Client(first_id) => {
                match self.client_command_sender.get(&first_id) {
                    Some(forward_client_command_sender) => {
                        let forward_command = Internal(HostCommand::RemoveSender(*second_node.id()));
                        //trying to send command forward
                        forward_node_command(forward_client_command_sender, forward_command, first_id, first_node.node_type(), *second_node.id(), second_node.node_type())?

                    },
                    None => return Err(format!("Client {first_id} command sender not found💥")),
                }
                for client in self.clients.iter_mut() {
                    if client.id == first_id {
                        client.connected_drone_ids.retain(|&id| id != *second_node.id());
                    }
                }
            }
            NetNode::Server(first_id) => {
                match self.server_command_sender.get(&first_id) {
                    Some(forward_server_command_sender) => {
                        let forward_command = HostCommand::RemoveSender(*second_node.id());
                        //trying to send command forward
                        forward_node_command(forward_server_command_sender, forward_command, first_id, first_node.node_type(), *second_node.id(), second_node.node_type())?

                    },
                    None => return Err(format!("Server {first_id} command sender not found")),
                }
                for server in self.servers.iter_mut() {
                    if server.id == first_id {
                        server.connected_drone_ids.retain(|&id| id != *second_node.id());
                    }
                }
            }
        }
        Ok(())
    }

    pub fn remove_sender(&mut self, node_id: NodeId, sender_id: NodeId) -> Result<(), String> {
        let forward_node = match NetNode::from(&self, node_id) {
            Some(node) => node,
            None => return Err(format!("Node {node_id} not found")),
        };
        let backwards_node = match NetNode::from(&self, sender_id){
            Some(node) => node,
            None => return Err(format!("Node {sender_id} not found")),
        };

        match self.cannot_remove(forward_node, backwards_node) {
            Ok(critical) => {
                if critical {
                    return Err(format!("Connection between {} {node_id} and {} {sender_id} is Critical and cannot be removed!", forward_node.node_type(), backwards_node.node_type()))
                }
            }
            Err(e) => return Err(e),
        }

        self.mono_remove_sender(forward_node, backwards_node)?;
        self.mono_remove_sender(backwards_node, forward_node)
    }

    /// - Client Commands

    fn send_client_command(&self, client_id: ClientId, command: ClientCommand) -> Result<(), String> {
        match self.client_command_sender.get(&client_id) {
            Some(client_command_sender) => {
                match client_command_sender.send(command) {
                    Ok(()) => Ok(()),
                    Err(e) => Err(e.to_string()),
                }
            },
            None => Err(format!("Client {client_id} command sender not found")),
        }
    }

    pub fn request_server_type(&mut self, client_id: ClientId) -> Result<(), String> {
        let command = Net(ServerType);
        debug!(target: "main", "Sending Server Type Request to Client {client_id}");
        self.send_client_command(client_id, command)
    }

    pub fn request_files_list(&mut self, client_id: ClientId, server_id: ServerId) -> Result<(), String> {
        debug!(target: "main", "Sending File List Request to Client {client_id} for Server {server_id}");
        let command = Net(FilesList(server_id));
        self.send_client_command(client_id, command)
    }

    pub fn request_file(&mut self, client_id: ClientId, server_id: ServerId, file_id: FileId) -> Result<(), String> {
        debug!(target: "main", "Sending File {file_id} Request to Client {client_id} for Server {server_id}");
        let command = Net(File(server_id, file_id));
        self.send_client_command(client_id, command)
    }

    pub fn request_media_list(&mut self, client_id: ClientId, server_id: ServerId) -> Result<(), String> {
        debug!(target: "main", "Sending Media List Request to Client {client_id} for Server {server_id}");
        let command = Net(MediaList(server_id));
        self.send_client_command(client_id, command)
    }

    pub fn request_media(&mut self, client_id: ClientId, server_id: ServerId, media_id: MediaId) -> Result<(), String> {
        debug!(target: "main", "Sending Media {media_id} Request to Client {client_id} for Server {server_id}");
        let command = Net(Media(server_id, media_id));
        self.send_client_command(client_id, command)
    }

    pub fn request_registration_to_chat(&mut self, client_id: ClientId, server_id: ServerId) -> Result<(), String> {
        debug!(target: "main", "Sending Registration to Chat Request to Client {client_id} for Server {server_id}");
        let command = Net(RegistrationToChat(server_id));
        self.send_client_command(client_id, command)
    }

    pub fn request_client_list(&mut self, client_id: ClientId, server_id: ServerId) -> Result<(), String> {
        //if client is not registered to chat it automatically sends a request to register
        let should_register = match self.chat_cache.known_contacts.get(&client_id) {
            Some(contact_map) => match contact_map.get(&server_id) {
                Some(contacts) => contacts.is_empty(),
                None => true,
            },
            None => true,
        };
        if should_register {
            if let Err(e) = self.request_registration_to_chat(client_id, server_id) {
                debug!(target: "main", "Failed to register client: {}", e);
            }
        }

        debug!(target: "main", "Sending Client List Request to Client {client_id} for Server {server_id}");
        let command = Net(ClientList(server_id));
        self.send_client_command(client_id, command)
    }

    pub fn message_for(&mut self, sender_client_id: ClientId, receiver_client_id: ClientId, server_id: ServerId, message: String) -> Result<(), String> {
        debug!(target: "main", "Sending Message For Request to Client {sender_client_id} for Server {server_id} to Client {receiver_client_id}");
        let command = Net(MessageFor(receiver_client_id, server_id, message));
        self.send_client_command(sender_client_id, command)
    }

    pub fn media_for<P: AsRef<Path>>(&mut self, sender_client_id: ClientId, receiver_client_id: ClientId, server_id: ServerId, path: P) -> Result<(), String> {
        let media_vec = match fs::read(&path) {
                Ok(vec) => vec,
                Err(e) => return Err(format!("Failed to read media {}: {}", path.as_ref().display(), e)),
            };
        let media = match ImageReader::new(Cursor::new(&media_vec))
                .with_guessed_format().map_err(|e| e.to_string())?
                .decode() {
                    Ok(media) => media,
                    Err(e) => return Err(e.to_string()),
                };

        let image = Image {
            width: media.width(),
            height: media.height(),
            data: media.to_rgba8().to_vec(),
        };
        let media_id = match get_filename(path) {
            Some(media_id) => media_id,
            None => return Err(String::from("Failed to get Media ID")),
        };
        debug!(target: "main", "Sending Media For Request to Client {sender_client_id} for Server {server_id} to Client {receiver_client_id}");
        let command = Net(MediaFor(receiver_client_id, server_id, media_id, image));
        self.send_client_command(sender_client_id, command)

    }

    pub fn file_for<P: AsRef<Path>>(&mut self, sender_client_id: ClientId, receiver_client_id: ClientId, server_id: ServerId, path: P) -> Result<(), String> {
        let file = match fs::read(&path) {
            Ok(vec) => vec,
            Err(e) => return Err(format!("Failed to read file {}: {}", path.as_ref().display(), e)),
        };
        let file_id = match get_filename(path) {
            Some(file_id) => file_id,
           None => return Err(String::from("Failed to get File ID")),
        };
        debug!(target: "main", "Sending File For Request to Client {sender_client_id} for Server {server_id} to Client {receiver_client_id}");
        let command = Net(FileFor(receiver_client_id, server_id, file_id, file));
        self.send_client_command(sender_client_id, command)

    }

    pub fn new_media<P: AsRef<Path>>(&mut self,sender_client_id: ClientId, server_id: ServerId, path: P) -> Result<(), String> {
        let media_vec = match fs::read(&path) {
            Ok(vec) => vec,
            Err(e) => return Err(format!("Failed to read media {}: {}", path.as_ref().display(), e)),
        };
        let media = match ImageReader::new(Cursor::new(&media_vec))
            .with_guessed_format().map_err(|e| e.to_string())?
            .decode() {
            Ok(media) => media,
            Err(e) => return Err(e.to_string()),
        };

        let image = Image {
            width: media.width(),
            height: media.height(),
            data: media.to_rgba8().to_vec(),
        };
        let media_id = match get_filename(path) {
            Some(media_id) => media_id,
            None => return Err(String::from("Failed to get Media ID")),
        };
        debug!(target: "main", "Sending New Media Request from Client {sender_client_id} to Server {server_id}");
        let command = Net(NewMedia(server_id, media_id, image));
        self.send_client_command(sender_client_id, command)
    }

    pub fn new_file<P: AsRef<Path>>(&mut self, sender_client_id: ClientId, server_id: ServerId, path: P) -> Result<(), String> {
        let file = match fs::read(&path) {
            Ok(vec) => vec,
            Err(e) => return Err(format!("Failed to read file {}: {}", path.as_ref().display(), e)),
        };
        let file_id = match get_filename(path) {
            Some(file_id) => file_id,
            None => return Err(String::from("Failed to get File ID")),
        };
        debug!(target: "main", "Sending NewFile Request from Client {sender_client_id} to Server {server_id}");
        let command = Net(NewFile(server_id, file_id, file));
        self.send_client_command(sender_client_id, command)

    }

    pub fn controller_shortcut(&mut self, client_id: ClientId, packet: Packet) -> Result<(), String> {
        debug!(target: "main", "[LEDRON-CLI] Sending Controller Shortcut to Client {client_id}");
        let command = ControllerShortcut(packet);
        self.send_client_command(client_id, command)
    }
}

fn get_filename<P: AsRef<Path>>(path: P) -> Option<String> {
    path.as_ref()
        .file_name()
        .and_then(|name| name.to_str())
        .map(|s| s.to_string())
}

fn forward_node_command<T: NodeCommand>(sender: &Sender<T>, command: T, first_id: NodeId, first_type: NodeType, second_id: NodeId, second_type: NodeType) -> Result<(), String> {
    match sender.send(command) {
        Ok(()) => {
            debug!(target: "main", "Asking to add {second_type} {second_id} Sender to {first_type} {first_id}");
            Ok(())
        },
        Err(e) => Err(e.to_string()),
    }
}
fn get_extension(media_id: &MediaId) -> Option<&str> {
    media_id.rsplitn(2, '.').next().filter(|&ext| ext != media_id)
}

fn open_image(path: &str) {
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(path).spawn().ok();
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd").args(["/C", "start", path]).spawn().ok();
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(path).spawn().ok();
    }
}