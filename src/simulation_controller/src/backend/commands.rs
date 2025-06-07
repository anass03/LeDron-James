use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::network::NodeId;
use wg_2024::packet::Packet;
use host::{ClientId, Extension, FileId, HostCommand, HostEvent, Image, MediaId, ServerId, ServerToClient};
use client::ClientCommand;
use client::ClientCommand::*;
use host::ClientNetCommand::*;
use image::{DynamicImage, ImageBuffer, RgbImage, Rgba};
use crate::backend::simulation_controller::SimulationControllerData;
use crate::backend::sc_models::LogType;


#[derive(Default, Clone, Copy)]
enum NodeType {
    #[default]
    Drone,
    Client,
    Server,
}

impl Display for NodeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeType::Drone => f.write_str("Drone"),
            NodeType::Client => f.write_str("Client"),
            NodeType::Server => f.write_str("Server"),
        }
    }
}

impl NodeType {
    fn from(sim: & SimulationControllerData, node_id: NodeId) -> NodeType {
        for drone in sim.drones.iter() {
            if drone.id == node_id {
                return NodeType::Drone;
            }
        }
        for client in sim.clients.iter() {
            if client.id == node_id {
                return NodeType::Client;
            }
        }
        for server in sim.servers.iter() {
            if server.id == node_id {
                return NodeType::Server;
            }
        }
        NodeType::default()
    }
}

fn dfs(
    current: NodeId,
    net_map: &HashMap<NodeId, Vec<NodeId>>,
    exclude: Option<NodeId>,
    visited: &mut HashSet<NodeId>
) {
    match exclude {
        Some(exclude) => {
            if current == exclude {
                return
            }
            visited.insert(current);
            for &neighbor in &net_map[&current] {
                if !visited.contains(&neighbor)  && neighbor != exclude {
                    dfs(neighbor, net_map, Some(exclude), visited);
                }
            }
        }
        None => {
            visited.insert(current);
            for &neighbor in &net_map[&current] {
                if !visited.contains(&neighbor) {
                    dfs(neighbor, net_map, None, visited);
                }
            }
        }
    }
}

impl SimulationControllerData {

    //builds connections map
    fn build_network_map(&mut self) -> HashMap<NodeId, Vec<NodeId>> {
        let mut network_map: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for client in self.clients.iter() {
            network_map.insert(client.id, client.connected_drone_ids.clone());
        }
        for server in self.servers.iter() {
            network_map.insert(server.id, server.connected_drone_ids.clone());
        }
        for drone in self.drones.iter() {
            network_map.insert(drone.id, drone.connected_node_ids.clone());
        }
        self.logs.push_sc_log(LogType::DEBUG, format!("{:?}", network_map));
        network_map
    }
    //returns true if node is critical
    fn check_critical_node(&mut self, drone_id: NodeId) -> Result<bool, String>{
        let net_map = self.build_network_map();
        if net_map.len() <= 1 {
            return Ok(false);
        }

        let mut visited = HashSet::new();

        let start = match net_map.keys().find(|&&id| id != drone_id) {
            Some(&id) => id,
            None => return Err(format!("Drone {drone_id} is not connected in the network"))
        };

        //applying dfs without target node to check if the network would still be connected
        dfs(start, &net_map, Some(drone_id), &mut visited);
        //If not all node could be found => target node is critical
        Ok(visited.len() + 1 != net_map.len())
    }

    fn check_critical_connection(&mut self, first_node: NodeId, second_node: NodeId) -> Result<bool, String>{
        let net_map = self.build_network_map();
        let mut modified_net_map = net_map.clone();
        //simulate removing connection
        match modified_net_map.get_mut(&first_node) {
            Some(mut first_node_neighbors) => {
                first_node_neighbors.retain(|&id| id != second_node);
            },
            None => return Err(format!("connection between {first_node} and {second_node} !does not exist"))
        }
        match modified_net_map.get_mut(&second_node) {
            Some(mut second_node_neighbors) => {
                second_node_neighbors.retain(|&id| id != first_node);
            },
            None => return Err(format!("connection between {first_node} and {second_node} !does not exist"))
        }

        let mut visited = HashSet::new();
        //applying dfs without connection
        dfs(first_node, &modified_net_map, None , &mut visited);

        Ok(!visited.contains(&second_node))
    }

    /// - Drone Commands
    pub fn crash(&mut self, drone_id: NodeId) -> Result<(), String> {
        match self.check_critical_node(drone_id) {
            Ok(critical) => {
                if critical {
                    return Err(format!("Drone {drone_id} cannot be crashed because it's critical!"))

                }
            },
            Err(e) => return Err(e)
        }
        self.logs.push_sc_log(LogType::DEBUG, format!("Trying to Crash Drone: {drone_id}"));
        let command = DroneCommand::Crash;
        match self.drone_command_sender.get(&drone_id) {
            Some(drone_command_sender) => {
                match drone_command_sender.send(command) {
                    Ok(_) => {
                        //removing all neighboring channels
                        let mut neighbors = vec![];
                        match self.drones.get((drone_id as usize) - 1) {
                            Some(drone) => {
                                for neighbor in drone.connected_node_ids.iter() {
                                    neighbors.push(*neighbor);
                                }
                            },
                            None => { return Err(format!("Drone {drone_id} does not exist")); }
                        }
                        for neighbor in neighbors.iter() {
                            let forward_node_type = NodeType::from(&self, drone_id);
                            let backwards_node_type = NodeType::from(&self, *neighbor);

                            self.mono_remove_sender(forward_node_type, drone_id, backwards_node_type, *neighbor)?;
                            self.mono_remove_sender(backwards_node_type, *neighbor, forward_node_type, drone_id)?
                        }
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
                        self.logs.push_sc_log(LogType::INFO, format!("Changing Drone {drone_id} PDR to: {valid_pdr}"));
                        Ok(()) },
                    Err(e) => Err(e.to_string()),
                }
            },
            None => Err(format!("Drone {drone_id} command sender not found")),
        }
    }

    /// - Drone Events

    pub fn handle_drone_event(&mut self, event: DroneEvent) -> Result<(), (LogType, String)> {
        match event {
            DroneEvent::PacketSent(packet) => {
                let _sender_id = packet.routing_header.hops[packet.routing_header.hop_index - 1 ];
                //handle_packet(&packet);
                let _session_id = packet.session_id;
                self.stats.received_packet();
                Ok(())
                //self.logs.push_sc_log(LogType::INFO, format!("Drone ID {:?} - Sent packet [ Origin: {sender_id}, Session ID: {session_id} ]", packet.routing_header.hops[packet.routing_header.hop_index - 1]).as_str());
            },
            DroneEvent::PacketDropped(packet) => {
                let _sender_id = packet.routing_header.hops[packet.routing_header.hop_index - 1 ];
                //handle_packet(&packet);
                let _session_id = packet.session_id;
                Ok(())
                //self.logs.push_sc_log(LogType::INFO, format!("Drone ID {:?} - Dropped packet [ Origin: {sender_id}, Session ID: {session_id}", packet.routing_header.hops[packet.routing_header.hop_index - 1]).as_str());
            },
            DroneEvent::ControllerShortcut(packet) => {
                let _sender_id = packet.routing_header.hops[packet.routing_header.hop_index - 1 ];
                let receiver_id = packet.routing_header.hops.get(packet.routing_header.hops.len() - 1).unwrap();
                match self.controller_shortcut(*receiver_id, packet) {
                    Ok(()) => Ok(()),
                    Err(e) => Err((LogType::ERROR, e)),
                }

            },
        }
    }

    /// - Nodes Commands
    fn mono_add_sender(&mut self, first_type: NodeType, first_id: NodeId, second_type: NodeType, second_id: NodeId) -> Result<(), String> {
        //checking if packet sender for forward node exists
        match self.packet_sender.get(&second_id) {
            Some(forward_packet_sender) => {
                //checking if packet sender for backwards node exists
                match first_type {
                    NodeType::Drone => {
                        match self.drone_command_sender.get(&first_id) {
                            Some(forward_drone_command_sender) => {
                                let forward_command = DroneCommand::AddSender(second_id, forward_packet_sender.clone());
                                //trying to send command forward
                                match forward_drone_command_sender.send(forward_command) {
                                    Ok(()) => {
                                        self.logs.push_sc_log(LogType::DEBUG, format!("Asking to add {second_type} {second_id} Sender to {first_type} {first_id}"));
                                    },
                                    Err(e) => return Err(e.to_string()),
                                }
                            },
                            None => return Err(format!("Drone {first_id} command sender not found")),
                        }
                        for drone in self.drones.iter_mut() {
                            if drone.id == first_id {
                                drone.connected_node_ids.push(second_id);
                            }
                        }
                    },
                    NodeType::Client => {
                        match self.client_command_sender.get(&first_id) {
                            Some(forward_client_command_sender) => {
                                let forward_command = Internal(HostCommand::AddSender(second_id, forward_packet_sender.clone()));
                                //trying to send command forward
                                match forward_client_command_sender.send(forward_command) {
                                    Ok(()) => {
                                        self.logs.push_sc_log(LogType::DEBUG, format!("Asking to add {second_type} {second_id} Sender to {first_type} {first_id}"));
                                    },
                                    Err(e) => return Err(e.to_string()),
                                }
                            },
                            None => return Err(format!("Client {first_id} command sender not found")),
                        }
                        for client in self.clients.iter_mut() {
                            if client.id == first_id {
                                client.connected_drone_ids.push(second_id);
                            }
                        }
                    },
                    NodeType::Server => {
                        match self.server_command_sender.get(&first_id) {
                            Some(forward_server_command_sender) => {
                                let forward_command = HostCommand::AddSender(second_id, forward_packet_sender.clone());
                                //trying to send command forward
                                match forward_server_command_sender.send(forward_command) {
                                    Ok(()) => {
                                        self.logs.push_sc_log(LogType::DEBUG, format!("Asking to add {second_type} {second_id} Sender to {first_type} {first_id}"));
                                    },
                                    Err(e) => return Err(e.to_string()),
                                }
                            },
                            None => return Err(format!("Server {first_id} command sender not found")),
                        }
                        for server in self.servers.iter_mut() {
                            if server.id == first_id {
                                server.connected_drone_ids.push(second_id);
                            }
                        }
                    }
                }
            },
            None => return Err(format!("{second_type} {second_id} packet sender not found")),
        }

        Ok(())
    }

    pub fn add_sender(&mut self, node_id: NodeId, sender_id: NodeId) -> Result<(), String> {
        let forward_node_type = NodeType::from(&self, node_id);
        let backwards_node_type = NodeType::from(&self, sender_id);

        self.mono_add_sender(forward_node_type, node_id, backwards_node_type, sender_id)?;
        self.mono_add_sender(backwards_node_type, sender_id, forward_node_type, node_id)
    }

    fn mono_remove_sender(&mut self, first_type: NodeType, first_id: NodeId, second_type: NodeType, second_id: NodeId) -> Result<(), String> {
        match first_type {
            NodeType::Drone => {
                match self.drone_command_sender.get(&first_id) {
                    Some(forward_drone_command_sender) => {
                        let forward_command = DroneCommand::RemoveSender(second_id);
                        //trying to send command forward
                        match forward_drone_command_sender.send(forward_command) {
                            Ok(()) => {
                                self.logs.push_sc_log(LogType::DEBUG, format!("Asking to remove {second_type} {second_id} Sender from {first_type} {first_id}"));
                            },
                            Err(e) => return Err(e.to_string()),
                        }
                    },
                    None => return Err(format!("Drone {first_id} command sender not found")),
                }
                for drone in self.drones.iter_mut() {
                    if drone.id == first_id {
                        drone.connected_node_ids.retain(|&id| id != second_id);
                    }
                }
            },
            NodeType::Client => {
                match self.client_command_sender.get(&first_id) {
                    Some(forward_client_command_sender) => {
                        let forward_command = Internal(HostCommand::RemoveSender(second_id));
                        //trying to send command forward
                        match forward_client_command_sender.send(forward_command) {
                            Ok(()) => {
                                self.logs.push_sc_log(LogType::DEBUG, format!("Asking to remove {second_type} {second_id} Sender from {first_type} {first_id}"));
                            },
                            Err(e) => return Err(e.to_string()),
                        }
                    },
                    None => return Err(format!("Client {first_id} command sender not found💥")),
                }
                for client in self.clients.iter_mut() {
                    if client.id == first_id {
                        client.connected_drone_ids.retain(|&id| id != second_id);
                    }
                }
            },
            NodeType::Server => {
                match self.server_command_sender.get(&first_id) {
                    Some(forward_server_command_sender) => {
                        let forward_command = HostCommand::RemoveSender(second_id);
                        //trying to send command forward
                        match forward_server_command_sender.send(forward_command) {
                            Ok(()) => {
                                self.logs.push_sc_log(LogType::DEBUG, format!("Asking to remove {second_type} {second_id} Sender from {first_type} {first_id}"));
                            },
                            Err(e) => return Err(e.to_string()),
                        }
                    },
                    None => return Err(format!("Server {first_id} command sender not found")),
                }
                for server in self.servers.iter_mut() {
                    if server.id == first_id {
                        server.connected_drone_ids.retain(|&id| id != second_id);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn remove_sender(&mut self, node_id: NodeId, sender_id: NodeId) -> Result<(), String> {
        let forward_node_type = NodeType::from(&self, node_id);
        let backwards_node_type = NodeType::from(&self, sender_id);

        match self.check_critical_connection(node_id, sender_id) {
            Ok(critical) => {
                if critical {
                    return Err(format!("Connection between {forward_node_type} {node_id} and {backwards_node_type} {sender_id} is Critical and cannot be removed!"))
                }
            }
            Err(e) => return Err(e),
        }

        self.mono_remove_sender(forward_node_type, node_id, backwards_node_type, sender_id)?;
        self.mono_remove_sender(backwards_node_type, sender_id, forward_node_type, node_id)
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
        self.logs.push_sc_log(LogType::DEBUG, format!("Sending Server Type Request to Client {client_id}"));
        self.send_client_command(client_id, command)
    }
 
    pub fn request_files_list(&mut self, client_id: ClientId, server_id: ServerId) -> Result<(), String> {
        self.logs.push_sc_log(LogType::DEBUG, format!("Sending File List Request to Client {client_id} for Server {server_id}"));
        let command = Net(FilesList(server_id));
        self.send_client_command(client_id, command)
    }

    pub fn request_file(&mut self, client_id: ClientId, server_id: ServerId, file_id: FileId) -> Result<(), String> {
        self.logs.push_sc_log(LogType::DEBUG, format!("Sending File {file_id} Request to Client {client_id} for Server {server_id}"));
        let command = Net(File(server_id, file_id));
        self.send_client_command(client_id, command)
    }

    pub fn request_media_list(&mut self, client_id: ClientId, server_id: ServerId) -> Result<(), String> {
        self.logs.push_sc_log(LogType::DEBUG, format!("Sending Media List Request to Client {client_id} for Server {server_id}"));
        let command = Net(MediaList(server_id));
        self.send_client_command(client_id, command)
    }

    pub fn request_media(&mut self, client_id: ClientId, server_id: ServerId, media_id: MediaId) -> Result<(), String> {
        self.logs.push_sc_log(LogType::DEBUG, format!("Sending Media {media_id} Request to Client {client_id} for Server {server_id}"));
        let command = Net(Media(server_id, media_id));
        self.send_client_command(client_id, command)
    }

    pub fn request_registration_to_chat(&mut self, client_id: ClientId, server_id: ServerId) -> Result<(), String> {
        self.logs.push_sc_log(LogType::DEBUG, format!("Sending Registration to Chat Request to Client {client_id} for Server {server_id}"));
        let command = Net(RegistrationToChat(server_id));
        self.send_client_command(client_id, command)
    }

    pub fn request_client_list(&mut self, client_id: ClientId, server_id: ServerId) -> Result<(), String> {
        self.logs.push_sc_log(LogType::DEBUG, format!("Sending Client List Request to Client {client_id} for Server {server_id}"));
        let command = Net(ClientList(server_id));
        self.send_client_command(client_id, command)
    }

    pub fn message_for(&mut self, sender_client_id: ClientId, receiver_client_id: ClientId, server_id: ServerId, message: String) -> Result<(), String> {
        self.logs.push_sc_log(LogType::DEBUG, format!("Sending Message For Request to Client {sender_client_id} for Server {server_id} to Client {receiver_client_id}"));
        let command = Net(MessageFor(receiver_client_id, server_id, message));
        self.send_client_command(sender_client_id, command)
    }

    pub fn media_for(&mut self, sender_client_id: ClientId, receiver_client_id: ClientId, server_id: ServerId, media_id: MediaId, extension: Extension, image: Image) -> Result<(), String> {
        self.logs.push_sc_log(LogType::DEBUG, format!("Sending Media For Request to Client {sender_client_id} for Server {server_id} to Client {receiver_client_id}"));
        let command = Net(MediaFor(receiver_client_id, server_id, media_id, extension, image));
        self.send_client_command(sender_client_id, command)

    }

    pub fn controller_shortcut(&mut self, client_id: ClientId, packet: Packet) -> Result<(), String> {
        self.logs.push_sc_log(LogType::DEBUG, format!("Sending Controller Shortcut to Client {client_id}"));
        let command = ControllerShortcut(packet);
        self.send_client_command(client_id, command)
    }

    /// - Client Events

    pub fn handle_channels(&mut self) {
        while let Ok(event) = self.host_event_receiver.try_recv() {
            match self.handle_host_event(event) {
                Ok(()) => {},
                Err(e) => {self.logs.push_sc_log(e.0, e.1);},
            }
        }
        while let Ok(log) = self.log_receiver.try_recv() {
            self.handle_log_event(log);
        }
        while let Ok(event) = self.drone_event_receiver.try_recv() {
            match self.handle_drone_event(event) {
                Ok(()) => {},
                Err(e) => {self.logs.push_sc_log(e.0, e.1)}
            }
        }
    }

    pub fn handle_host_event(&mut self, event: HostEvent) -> Result<(), (LogType, String)> {
        match event {
            HostEvent::PacketReconstructed(server_to_client) => {
                match server_to_client {
                    ServerToClient::ServerType(server_type)
                    => {
                        self.logs.push_sc_log(LogType::INFO, format!("Server type: {:?}", server_type)) },
                    ServerToClient::FilesList(server_id, file_list) 
                    => self.logs.push_sc_log(LogType::INFO, format!("Files list from Server {server_id} received: {file_list:?}")),
                    ServerToClient::File(server_id, file_id, extension, file) 
                    => {
                        // Construct filename based on file ID and inferred extension
                        let filename = format!("file_{}.{}", file_id, extension);
                        let path = PathBuf::from("assets").join(&filename);

                        fs::write(&path, &file).expect("file not downloaded");

                        // Open with system default application
                        #[cfg(target_os = "windows")]
                        {
                            let path_str = path.to_str().unwrap();
                            //let quoted_path = format!("\"{}\"", path_str);
                            Command::new("cmd")
                                .args(&["/C", "start", "", &path_str])
                                .spawn()
                                .expect("Unable to open file on Windows");
                        }

                        #[cfg(target_os = "macos")]
                        {
                            Command::new("open")
                                .arg(&path)
                                .spawn()
                                .expect("Unable to open file on macOS");
                        }

                        #[cfg(target_os = "linux")]
                        {
                            Command::new("xdg-open")
                                .arg(&path)
                                .spawn()
                                .expect("Unable to open file on Linux");
                        }

                        self.logs.push_sc_log(LogType::INFO, format!("File {file_id} received from Server {server_id}"));

                    },
                    ServerToClient::MediaList(server_id, media_list) 
                    => self.logs.push_sc_log(LogType::INFO, format!("Media list from Server {server_id} received: {:?}", media_list)),
                    ServerToClient::Media(server_id, media_id, extension, image) 
                    => {
                        let path = format!("assets/image_{}.{extension}", media_id);
                        let path_ref = Path::new(&path);

                        if let Some(rgba_buffer) = ImageBuffer::<Rgba<u8>, _>::from_raw(image.width, image.height, image.data.clone()) {
                            // Convert to DynamicImage
                            let dynamic_image = DynamicImage::ImageRgba8(rgba_buffer);

                            // If extension is jpg/jpeg, convert to RGB to drop alpha
                            let save_result = match extension.to_lowercase().as_str() {
                                "jpg" | "jpeg" => {
                                    let rgb_image: RgbImage = dynamic_image.to_rgb8();
                                    rgb_image.save(path_ref)
                                },
                                _ => dynamic_image.save(path_ref), // png, etc.
                            };

                            match save_result {
                                Ok(_) => {
                                    self.logs.push_sc_log(LogType::INFO, format!("Media {} from Server {server_id} saved as {}", media_id, path));
                                    open_image(&path)
                                },
                                Err(e) => self.logs.push_sc_log(LogType::ERROR, format!("Error saving media {}: {}", media_id, e)),
                            }
                        } else {
                            self.logs.push_sc_log(LogType::ERROR, format!("Impossible to  generate image from media {}", media_id));
                        }
                    },
                    ServerToClient::ClientList(server_id, client_list) 
                    => self.logs.push_sc_log(LogType::INFO, format!("Client list from CatServer {server_id} received: {:?}", client_list)),
                    ServerToClient::MessageFrom(server_id, client_id, message) 
                    => self.logs.push_sc_log(LogType::INFO, format!("Message from Client {client_id} received through Server {server_id}: {message}")),
                    ServerToClient::FileFrom(server_id, client_id,..)
                    => {
                        self.logs.push_sc_log(LogType::INFO, format!("File from Client {client_id} received through Server {server_id}"));
                    }
                    ServerToClient::MediaFrom(server_id, client_id, ..)
                    => {
                        self.logs.push_sc_log(LogType::INFO, format!("Media from Client {client_id} received through Server {server_id}"));
                    }
                    ServerToClient::WrongServer(server_id)
                    => {
                        self.logs.push_sc_log(LogType::INFO, format!("Wrong Server ID: {server_id}"));
                    }
                    ServerToClient::ErrNotFound(server_id) 
                    => return Err((LogType::ERROR, format!("Err Not Found by Server {server_id}"))),
                    ServerToClient::ErrClientId(server_id) 
                    => return Err((LogType::ERROR, format!("Err Client Id Not Found from Server {server_id}"))),
                }
            },
            HostEvent::PacketSent(_packet) => {
                self.stats.received_packet();
            },
            HostEvent::ServersDiscovered(servers) => {
                self.logs.push_sc_log(LogType::INFO, format!("Servers discovered: {servers:?}"));
            }
        }
        Ok(())
    }

    pub fn handle_log_event(&mut self, log: String) {
        self.logs.push_log(LogType::INFO, log.as_str())
    }
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

/*fn handle_packet (_packet: &Packet) {
    match packet.clone().pack_type {
        PacketType::MsgFragment(fragment) => {}
        PacketType::Ack(ack) => {}
        PacketType::Nack(nack) => {}
        PacketType::FloodRequest(flood_request) => {}
        PacketType::FloodResponse(flood_response) => {}
    }
}*/