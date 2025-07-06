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
use petgraph::visit::{Topo, Walker};
use ratatui_image::thread::ResizeRequest;
use server::chat::Chat;
use server::MediaServerError;
use crate::frontend::app::topology_algorithm::alg_models::{DisplayedNode, TopologyAlgorithm, TopologyStatus};
use crate::frontend::app::topology_algorithm::vectors::Vector;
use log::*;

impl SimulationControllerData {
    /// - Drone Commands
    pub fn crash(&mut self, drone_id: NodeId) -> Result<(), String> {
        // Early validation - check if drone exists and can be crashed
        if let Err(e) = self.cannot_crash(drone_id) {
            return Err(e);
        }

        // Find the drone and collect neighbors in one pass
        let drone_neighbors = self.drones.iter()
            .find(|drone| drone.id == drone_id)
            .map(|drone| drone.connected_node_ids.clone())
            .ok_or_else(|| format!("Drone {drone_id} does not exist"))?;

        debug!(target: "main", "Trying to Crash Drone: {drone_id}");

        // Send crash command
        let command = DroneCommand::Crash;
        let drone_command_sender = self.drone_command_sender.get(&drone_id)
            .ok_or_else(|| format!("Drone {drone_id} command sender not found"))?;

        drone_command_sender.send(command)
            .map_err(|e| format!("Error sending crash command - {}", e))?;

        // Remove all neighboring connections efficiently
        let forward_node = NetNode::Drone(drone_id);
        for neighbor_id in &drone_neighbors {
            let backwards_node = NetNode::from(&self, *neighbor_id)
                .ok_or_else(|| format!("Node {neighbor_id} not found"))?;

            self.mono_remove_sender(forward_node, backwards_node)?;
            self.mono_remove_sender(backwards_node, forward_node)?;
        }

        // Clean up drone data
        self.drones.retain(|d| d.id != drone_id);
        self.drone_command_sender.remove(&drone_id);
        self.packet_sender.remove(&drone_id);
        self.stats.total_active_connections -= 1;

        Ok(())
    }

    pub fn set_pdr(&mut self, drone_id: NodeId, new_pdr: f32) -> Result<(), String> {
        // Validate PDR value
        if !(0.0..=1.0).contains(&new_pdr) {
            return Err(format!("Invalid PDR value: {} | 0.0 <= PDR <= 1.0", new_pdr));
        }

        let command = DroneCommand::SetPacketDropRate(new_pdr);
        let drone_command_sender = self.drone_command_sender.get(&drone_id)
            .ok_or_else(|| format!("Drone {drone_id} command sender not found"))?;

        drone_command_sender.send(command)
            .map_err(|e| e.to_string())?;

        // Update drone PDR in single pass
        if let Some(drone) = self.drones.iter_mut().find(|drone| drone.id == drone_id) {
            drone.pdr = new_pdr;
        }

        info!(target: "main", "Changing Drone {drone_id} PDR to: {new_pdr}");
        Ok(())
    }

    /// - Net Commands
    fn mono_add_sender(&mut self, first_node: NetNode, second_node: NetNode) -> Result<(), String> {
        // Get packet sender for second node first
        let forward_packet_sender = self.packet_sender.get(second_node.id())
            .ok_or_else(|| format!("{} {} packet sender not found", second_node.node_type(), second_node.id()))?
            .clone();

        // Handle command sending and connection update based on first node type
        match first_node {
            NetNode::Drone(first_id) => {
                let forward_drone_command_sender = self.drone_command_sender.get(&first_id)
                    .ok_or_else(|| format!("Drone {first_id} command sender not found"))?;

                let forward_command = DroneCommand::AddSender(*second_node.id(), forward_packet_sender);
                forward_node_command(forward_drone_command_sender, forward_command, first_id, first_node.node_type(), *second_node.id(), second_node.node_type())?;

                // Update drone connections
                if let Some(drone) = self.drones.iter_mut().find(|d| d.id == first_id) {
                    drone.connected_node_ids.push(*second_node.id());
                }
            },
            NetNode::Client(first_id) => {
                let forward_client_command_sender = self.client_command_sender.get(&first_id)
                    .ok_or_else(|| format!("Client {first_id} command sender not found"))?;

                let forward_command = Internal(HostCommand::AddSender(*second_node.id(), forward_packet_sender));
                forward_node_command(forward_client_command_sender, forward_command, first_id, first_node.node_type(), *second_node.id(), second_node.node_type())?;

                // Update client connections
                if let Some(client) = self.clients.iter_mut().find(|c| c.id == first_id) {
                    client.connected_drone_ids.push(*second_node.id());
                }
            },
            NetNode::Server(first_id) => {
                let forward_server_command_sender = self.server_command_sender.get(&first_id)
                    .ok_or_else(|| format!("Server {first_id} command sender not found"))?;

                let forward_command = HostCommand::AddSender(*second_node.id(), forward_packet_sender);
                forward_node_command(forward_server_command_sender, forward_command, first_id, first_node.node_type(), *second_node.id(), second_node.node_type())?;

                // Update server connections
                if let Some(server) = self.servers.iter_mut().find(|s| s.id == first_id) {
                    server.connected_drone_ids.push(*second_node.id());
                }
            }
        }

        if let Some(mut vec) = self.running_network.get_mut(&first_node) {
            vec.push(*second_node.id());
        }

        Ok(())
    }

    pub fn add_sender(&mut self, node_id: NodeId, sender_id: NodeId) -> Result<(), String> {
        let forward_node = NetNode::from(&self, node_id)
            .ok_or_else(|| format!("Node {node_id} not found"))?;
        let backwards_node = NetNode::from(&self, sender_id)
            .ok_or_else(|| format!("Node {sender_id} not found"))?;

        // Check if connection can be added
        if let Err(e) = self.cannot_add(forward_node, backwards_node) {
            return Err(e);
        }

        // Add connection in both directions
        self.mono_add_sender(forward_node, backwards_node)?;
        self.mono_add_sender(backwards_node, forward_node)
    }

    fn mono_remove_sender(&mut self, first_node: NetNode, second_node: NetNode) -> Result<(), String> {
        // Handle command sending and connection update based on first node type
        match first_node {
            NetNode::Drone(first_id) => {
                let forward_drone_command_sender = self.drone_command_sender.get(&first_id)
                    .ok_or_else(|| format!("Drone {first_id} command sender not found"))?;

                let forward_command = DroneCommand::RemoveSender(*second_node.id());
                forward_node_command(forward_drone_command_sender, forward_command, first_id, first_node.node_type(), *second_node.id(), second_node.node_type())?;

                // Update drone connections
                if let Some(drone) = self.drones.iter_mut().find(|d| d.id == first_id) {
                    drone.connected_node_ids.retain(|&id| id != *second_node.id());
                }
            }
            NetNode::Client(first_id) => {
                let forward_client_command_sender = self.client_command_sender.get(&first_id)
                    .ok_or_else(|| format!("Client {first_id} command sender not found"))?;

                let forward_command = Internal(HostCommand::RemoveSender(*second_node.id()));
                forward_node_command(forward_client_command_sender, forward_command, first_id, first_node.node_type(), *second_node.id(), second_node.node_type())?;

                // Update client connections
                if let Some(client) = self.clients.iter_mut().find(|c| c.id == first_id) {
                    client.connected_drone_ids.retain(|&id| id != *second_node.id());
                }
            }
            NetNode::Server(first_id) => {
                let forward_server_command_sender = self.server_command_sender.get(&first_id)
                    .ok_or_else(|| format!("Server {first_id} command sender not found"))?;

                let forward_command = HostCommand::RemoveSender(*second_node.id());
                forward_node_command(forward_server_command_sender, forward_command, first_id, first_node.node_type(), *second_node.id(), second_node.node_type())?;

                // Update server connections
                if let Some(server) = self.servers.iter_mut().find(|s| s.id == first_id) {
                    server.connected_drone_ids.retain(|&id| id != *second_node.id());
                }
            }
        }

        if let Some(mut vec) = self.running_network.get_mut(&first_node) {
            vec.retain(|id| id != second_node.id());
        }

        Ok(())
    }

    pub fn remove_sender(&mut self, node_id: NodeId, sender_id: NodeId) -> Result<(), String> {
        let forward_node = NetNode::from(&self, node_id)
            .ok_or_else(|| format!("Node {node_id} not found"))?;
        let backwards_node = NetNode::from(&self, sender_id)
            .ok_or_else(|| format!("Node {sender_id} not found"))?;

        // Check if connection can be removed
        match self.cannot_remove(forward_node, backwards_node) {
            Ok(true) => {
                return Err(format!("Connection between {} {node_id} and {} {sender_id} is Critical and cannot be removed!", forward_node.node_type(), backwards_node.node_type()));
            }
            Err(e) => return Err(e),
            Ok(false) => {} // Connection can be removed
        }

        // Remove connection in both directions
        self.mono_remove_sender(forward_node, backwards_node)?;
        self.mono_remove_sender(backwards_node, forward_node)
    }

    /// - Client Commands

    #[inline]
    fn send_client_command(&self, client_id: ClientId, command: ClientCommand) -> Result<(), String> {
        self.client_command_sender.get(&client_id)
            .ok_or_else(|| format!("Client {client_id} command sender not found"))?
            .send(command)
            .map_err(|e| e.to_string())
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
        // Check if client is registered to chat
        let should_register = self.chat_cache.known_contacts
            .get(&client_id)
            .and_then(|contact_map| contact_map.get(&server_id))
            .map_or(true, |contacts| contacts.is_empty());

        if should_register {
            if let Err(e) = self.request_registration_to_chat(client_id, server_id) {
                debug!(target: "main", "Failed to register client: {}", e);
            }
        }

        debug!(target: "main", "Sending Client List Request to Client {client_id} for Server {server_id}");
        for client in &self.clients {
            let command = Net(ClientList(server_id));
            self.send_client_command(client.id, command)?
        }
        Ok(())
    }

    pub fn message_for(&mut self, sender_client_id: ClientId, receiver_client_id: ClientId, server_id: ServerId, message: String) -> Result<(), String> {
        debug!(target: "main", "Sending Message For Request to Client {sender_client_id} for Server {server_id} to Client {receiver_client_id}");
        let command = Net(MessageFor(receiver_client_id, server_id, message));
        self.send_client_command(sender_client_id, command)
    }

    pub fn media_for<P: AsRef<Path>>(&mut self, sender_client_id: ClientId, receiver_client_id: ClientId, server_id: ServerId, path: P) -> Result<(), String> {
        let media_vec = fs::read(&path)
            .map_err(|e| format!("Failed to read media {}: {}", path.as_ref().display(), e))?;

        let media = ImageReader::new(Cursor::new(&media_vec))
            .with_guessed_format()
            .map_err(|e| e.to_string())?
            .decode()
            .map_err(|e| e.to_string())?;

        let image = Image {
            width: media.width(),
            height: media.height(),
            data: media.to_rgba8().to_vec(),
        };

        let media_id = get_filename(path)
            .ok_or_else(|| "Failed to get Media ID".to_string())?;

        debug!(target: "main", "Sending Media For Request to Client {sender_client_id} for Server {server_id} to Client {receiver_client_id}");
        let command = Net(MediaFor(receiver_client_id, server_id, media_id, image));
        self.send_client_command(sender_client_id, command)
    }

    pub fn file_for<P: AsRef<Path>>(&mut self, sender_client_id: ClientId, receiver_client_id: ClientId, server_id: ServerId, path: P) -> Result<(), String> {
        let file = fs::read(&path)
            .map_err(|e| format!("Failed to read file {}: {}", path.as_ref().display(), e))?;

        let file_id = get_filename(path)
            .ok_or_else(|| "Failed to get File ID".to_string())?;

        debug!(target: "main", "Sending File For Request to Client {sender_client_id} for Server {server_id} to Client {receiver_client_id}");
        let command = Net(FileFor(receiver_client_id, server_id, file_id, file));
        self.send_client_command(sender_client_id, command)
    }

    pub fn new_media<P: AsRef<Path>>(&mut self, sender_client_id: ClientId, server_id: ServerId, path: P) -> Result<(), String> {
        let media_vec = fs::read(&path)
            .map_err(|e| format!("Failed to read media {}: {}", path.as_ref().display(), e))?;

        let media = ImageReader::new(Cursor::new(&media_vec))
            .with_guessed_format()
            .map_err(|e| e.to_string())?
            .decode()
            .map_err(|e| e.to_string())?;

        let image = Image {
            width: media.width(),
            height: media.height(),
            data: media.to_rgba8().to_vec(),
        };

        let media_id = get_filename(path)
            .ok_or_else(|| "Failed to get Media ID".to_string())?;

        debug!(target: "main", "Sending New Media Request from Client {sender_client_id} to Server {server_id}");
        let command = Net(NewMedia(server_id, media_id, image));
        self.send_client_command(sender_client_id, command)
    }

    pub fn new_file<P: AsRef<Path>>(&mut self, sender_client_id: ClientId, server_id: ServerId, path: P) -> Result<(), String> {
        let file = fs::read(&path)
            .map_err(|e| format!("Failed to read file {}: {}", path.as_ref().display(), e))?;

        let file_id = get_filename(path)
            .ok_or_else(|| "Failed to get File ID".to_string())?;

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

#[inline]
fn get_filename<P: AsRef<Path>>(path: P) -> Option<String> {
    path.as_ref()
        .file_name()
        .and_then(|name| name.to_str())
        .map(|s| s.to_string())
}

#[inline]
fn forward_node_command<T: NodeCommand>(sender: &Sender<T>, command: T, first_id: NodeId, first_type: NodeType, second_id: NodeId, second_type: NodeType) -> Result<(), String> {
    sender.send(command)
        .map_err(|e| e.to_string())
        .map(|_| debug!(target: "main", "Asking to add {second_type} {second_id} Sender to {first_type} {first_id}"))
}
