use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::network::NodeId;
use wg_2024::packet::Packet;
use host::{ClientId, FileId, HostCommand, HostEvent, Image, MediaId, ServerId, ServerToClient};
use client::ClientCommand;
use client::ClientCommand::*;
use host::ClientNetCommand::*;
use image::{DynamicImage, ImageBuffer, ImageReader, RgbImage, Rgba};
use crate::backend::simulation_controller::{NetNode, SimulationControllerData};
use crate::backend::sc_models::{LogType, ChatList, NetCacheMedia, RendererResponse, FileRendererUpdate, RendererCommand};
use host::ServerTypes::*;
use ratatui_image::thread::ResizeRequest;
use server::chat2::Chat;
use server::MediaServerError;

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
impl NetNode {
    pub fn id(&self) -> &NodeId {
        match self {
            NetNode::Drone(id) => id,
            NetNode::Client(id) => id,
            NetNode::Server(id) => id,
        }
    }

    fn from(sim: & SimulationControllerData, node_id: NodeId) -> Option<NetNode> {
        for drone in sim.drones.iter() {
            if drone.id == node_id {
                return Some(NetNode::Drone(node_id));
            }
        }
        for client in sim.clients.iter() {
            if client.id == node_id {
                return Some(NetNode::Client(node_id));
            }
        }
        for server in sim.servers.iter() {
            if server.id == node_id {
                return Some(NetNode::Server(node_id));
            }
        }
        None
    }

    fn node_type(&self) -> NodeType {
        match self {
            NetNode::Drone(_) => NodeType::Drone,
            NetNode::Client(_) => NodeType::Client,
            NetNode::Server(_) => NodeType::Server,
        }
    }
}
type NodeMap = HashMap<NetNode, Vec<NodeId>>;

fn dfs(
    current: NodeId,
    graph: &HashMap<NodeId, Vec<NodeId>>,
    exclude: Option<NodeId>,
    visited: &mut HashSet<NodeId>,
)
{
    if Some(current) == exclude {return}
    visited.insert(current);
    if let Some(neighbors) = graph.get(&current) {
        for &neighbor in neighbors {
            if !visited.contains(&neighbor) && Some(neighbor) != exclude {
                dfs(neighbor, graph, exclude, visited);
            }
        }
    }
}

/*
    //builds connections map
    fn build_network_map(&mut self) -> NodeMap {
        let mut network_map: HashMap<NetNode, Vec<NodeId>> = HashMap::new();
        for client in self.clients.iter() {
            network_map.insert(NetNode::Client(client.id), client.connected_drone_ids.clone());
        }
        for server in self.servers.iter() {
            network_map.insert(NetNode::Server(server.id), server.connected_drone_ids.clone());
        }
        for drone in self.drones.iter() {
            network_map.insert(NetNode::Drone(drone.id), drone.connected_node_ids.clone());
        }
        self.logs.push_sc_log(LogType::DEBUG, format!("{:?}", network_map));
        network_map
    }
    fn cannot_crash(&mut self, drone_id: NodeId) -> Result<bool, String> {
        //net cannot be separated
        //hosts must be at the edges of the net
        //clients must have 1 or 2 neighbors
        //servers must have at least 2 neighbors

        //create a NodeMap of my network -> NodeMap: HashMap<NetNode, Vec<NodeId>>
        let net_map = self.build_network_map();
        if net_map.len() <= 1 {
            return Ok(false);
        }

        //Build a net without target drone
        let mut crashed_net = net_map.clone();
        match crashed_net.remove(&NetNode::Drone(drone_id)) {
            Some(_) => (),
            None => return Err(format!("Drone {drone_id} is not in the net!"))
        }
        //check hosts constraints
        for (node, neighbors) in crashed_net.iter() {
            match node {
                NetNode::Client(client_id) => {
                    let neighbors: Vec<NodeId> = neighbors
                        .iter()
                        .filter(|id| **id!= drone_id)
                        .cloned()
                        .collect();
                    let adj = neighbors.len();
                    if adj <= 0 || adj > 2 {
                        return Err(format!("Cannot Crash Drone {drone_id} because Client {client_id} would have {adj} Neighbors!"));
                    }
                }
                NetNode::Server(server_id) => {
                    let neighbors: Vec<NodeId> = neighbors
                        .iter()
                        .filter(|id| **id!= drone_id)
                        .cloned()
                        .collect();
                    let adj = neighbors.len();
                    if adj <= 1 {
                        return Err(format!("Cannot Crash Drone {drone_id} because Server {server_id} would have only {adj} Neighbors!"));
                    }
                }
                _ => continue
            }
        }
        let mut host_ids = Vec::new();
        for node in net_map.keys() {
            match node {
                NetNode::Client(client_id) => {host_ids.push(*client_id);},
                NetNode::Server(server_id) => {host_ids.push(*server_id);},
                _ => continue
            }
        }

        // Build a net without hosts
        let mut drone_net: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for (drone, neighbors) in &net_map {
            if let NetNode::Drone(drone_id) = drone {
                let adj: Vec<_> = neighbors
                    .iter()
                    .filter(|id| !host_ids.contains(id))
                    .cloned()
                    .collect();
                drone_net.insert(*drone_id, adj);
            }
        }

        // Check if crashing splits the network without considering hosts (they are at the edges)
        let mut visited = HashSet::new();
        let &start = drone_net.keys().find(|&&id| id != drone_id).ok_or("No other nodes to start DFS")?;
        dfs(start, &drone_net, Some(drone_id), &mut visited);
        if visited.len() + 1 != drone_net.len() {
            return Ok(true);
        }
        Ok(false)
    }

    fn cannot_remove(&mut self, first: NetNode, second: NetNode) -> Result<bool, String> {
        //net cannot be separated
        //hosts must be at the edges of the net
        //clients must have 1 or 2 neighbors
        //servers must have at least 2 neighbors
        let net_map = self.build_network_map();
        let mut modified_net_map = net_map.clone();
        //simulate removing connection
        if let Some(first_neighbors) = modified_net_map.get_mut(&first) {
            if !first_neighbors.contains(&first.id()) {
                return Err(format!("{} {} is not connected to {} {}!", first.node_type(), first.id(), second.node_type(), second.id()));
            }
            first_neighbors.retain(|&id| id != *second.id());
        } else {
            return Err(format!("{} {} is not in the map!", first.node_type(), first.id()));
        }
        if let Some(second_neighbors) = modified_net_map.get_mut(&second) {
            if !second_neighbors.contains(&first.id()) {
                return Err(format!("{} {} is not connected to {} {}!", first.node_type(), first.id(), second.node_type(), second.id()));
            }
            second_neighbors.retain(|&id| id != *first.id());
        } else {
            return Err(format!("{} {} is not in the map!", second.node_type(), second.id()));
        }

        //check hosts constraints
        for (node, neighbors) in modified_net_map.iter() {
            match node {
                NetNode::Client(client_id) => {
                    let adj = neighbors.len();
                    if adj <= 0 || adj > 2 {
                        return Err(format!("Connection cannot be removed because Client {client_id} would have {adj} Neighbors!"));
                    }
                }
                NetNode::Server(server_id) => {
                    let adj = neighbors.len();
                    if adj <= 1 {
                        return Err(format!("Connection cannot be removed because Server {server_id} would have only {adj} Neighbors!"));
                    }
                }
                _ => continue
            }
        }
        let mut host_ids = Vec::new();
        for node in net_map.keys() {
            match node {
                NetNode::Client(client_id) => {host_ids.push(*client_id);},
                NetNode::Server(server_id) => {host_ids.push(*server_id);},
                _ => continue
            }
        }

        // Build a net without hosts
        let mut drone_net: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for (node, neighbors) in &modified_net_map {
            if let NetNode::Drone(drone_id) = node {
                let adj: Vec<_> = neighbors
                    .iter()
                    .filter(|id| !host_ids.contains(id))
                    .cloned()
                    .collect();
                drone_net.insert(*drone_id, adj);
            }
        }
        self.logs.push_sc_log(LogType::DEBUG, format!("{:?}", drone_net));

        // Check if removing connection splits the network without considering hosts (they are at the edges)
        let mut visited = HashSet::new();
        dfs(*first.id(), &drone_net, None, &mut visited);
        if visited.len() != drone_net.len() {
            return Ok(true);
        }
        Ok(false)
    }

    fn cannot_add(&mut self, first: NetNode, second: NetNode) -> Result<bool, String> {
        //clients must have 1 or 2 neighbors
        //hosts cannot connect directly with other hosts
        let net_map = self.build_network_map();
        let mut modified_net_map = net_map.clone();
        //simulate adding connection
        match modified_net_map.get_mut(&first) {
            Some(mut first_node_neighbors) => {
                first_node_neighbors.push(*second.id());
            },
            None => return Err(format!("{} {} is not in the map!", first.node_type(), first.id()))
        }
        match modified_net_map.get_mut(&second) {
            Some(mut second_node_neighbors) => {
                second_node_neighbors.push(*first.id());
            },
            None => return Err(format!("{} {} is not in the map!", second.node_type(), second.id()))
        }

        //check hosts constraints
        for (node, neighbors) in modified_net_map.iter() {
            match node {
                NetNode::Client(client_id) => {
                    let adj = neighbors.len();
                    if adj <= 0 || adj > 2 {
                        return Err(format!("Connection cannot be removed because Client {client_id} would have {adj} Neighbors!"));
                    }
                }
                _ => continue
            }
        }
        Ok(false)
    }

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
        self.logs.push_sc_log(LogType::DEBUG, format!("Trying to Crash Drone: {drone_id}"));
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

                    // ———> qui rimuovo COMPLETAMENTE il drone dallo stato
                        self.drones.retain(|d| d.id != drone_id);
                        self.drone_command_sender.remove(&drone_id);
                        self.packet_sender.remove(&drone_id);
                        // se tieni altri mappi (es. self.running_network, self.stats, ecc.)
                        // rimuovi anche lì drone_id

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
                //let _sender_id = packet.routing_header.hops[packet.routing_header.hop_index - 1 ];
                //handle_packet(&packet);
                let _session_id = packet.session_id;
                self.stats.received_packet();
                Ok(())
                //self.logs.push_sc_log(LogType::INFO, format!("Drone ID {:?} - Sent packet [ Origin: {sender_id}, Session ID: {session_id} ]", packet.routing_header.hops[packet.routing_header.hop_index - 1]).as_str());
            },
            DroneEvent::PacketDropped(packet) => {
                //let _sender_id = packet.routing_header.hops[packet.routing_header.hop_index - 1 ];
                //handle_packet(&packet);
                let _session_id = packet.session_id;
                Ok(())
                //self.logs.push_sc_log(LogType::INFO, format!("Drone ID {:?} - Dropped packet [ Origin: {sender_id}, Session ID: {session_id}", packet.routing_header.hops[packet.routing_header.hop_index - 1]).as_str());
            },
            DroneEvent::ControllerShortcut(packet) => {
                //let _sender_id = packet.routing_header.hops[packet.routing_header.hop_index - 1 ];
                let receiver_id = packet.routing_header.hops.get(packet.routing_header.hops.len() - 1).unwrap();
                match self.controller_shortcut(*receiver_id, packet) {
                    Ok(()) => Ok(()),
                    Err(e) => Err((LogType::ERROR, e)),
                }

            },
        }
    }

    /// - Nodes Commands
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
                                match forward_drone_command_sender.send(forward_command) {
                                    Ok(()) => {
                                        self.logs.push_sc_log(LogType::DEBUG, format!("Asking to add {} {} Sender to {} {first_id}", second_node.node_type(), second_node.id(), first_node.node_type()));
                                    },
                                    Err(e) => return Err(e.to_string()),
                                }
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
                                match forward_client_command_sender.send(forward_command) {
                                    Ok(()) => {
                                        self.logs.push_sc_log(LogType::DEBUG, format!("Asking to add {} {} Sender to {} {first_id}", second_node.node_type(), second_node.id(), first_node.node_type()));
                                    },
                                    Err(e) => return Err(e.to_string()),
                                }
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
                                match forward_server_command_sender.send(forward_command) {
                                    Ok(()) => {
                                        self.logs.push_sc_log(LogType::DEBUG, format!("Asking to add {} {} Sender to {} {first_id}", second_node.node_type(), second_node.id(), first_node.node_type()));
                                    },
                                    Err(e) => return Err(e.to_string()),
                                }
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
                        match forward_drone_command_sender.send(forward_command) {
                            Ok(()) => {
                                self.logs.push_sc_log(LogType::DEBUG, format!("Asking to remove {} {} Sender from {} {first_id}", second_node.node_type(), second_node.id(), first_node.node_type()));
                            },
                            Err(e) => return Err(e.to_string()),
                        }
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
                        match forward_client_command_sender.send(forward_command) {
                            Ok(()) => {
                                self.logs.push_sc_log(LogType::DEBUG, format!("Asking to remove {} {} Sender from {} {first_id}", second_node.node_type(), second_node.id(), first_node.node_type()));
                            },
                            Err(e) => return Err(e.to_string()),
                        }
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
                        match forward_server_command_sender.send(forward_command) {
                            Ok(()) => {
                                self.logs.push_sc_log(LogType::DEBUG, format!("Asking to remove {} {} Sender from {} {first_id}", second_node.node_type(), second_node.id(), first_node.node_type()));
                            },
                            Err(e) => return Err(e.to_string()),
                        }
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
    }*/

#[derive(Debug, Clone)]
enum NetworkOperation {
    CrashDrone(NodeId),
    RemoveConnection(NetNode, NetNode),
    AddConnection(NetNode, NetNode),
}
// Struct to cache network state and avoid rebuilding maps
struct NetworkState {
    node_map: NodeMap,
    drone_graph: HashMap<NodeId, Vec<NodeId>>,
    host_ids: HashSet<NodeId>,
}

impl SimulationControllerData {
    // Build network state once and reuse
    fn build_network_state(&self) -> NetworkState {
        let mut network_map: HashMap<NetNode, Vec<NodeId>> = HashMap::new();
        let mut host_ids = HashSet::new();

        // Build network map and collect host IDs in one pass
        for client in &self.clients {
            network_map.insert(NetNode::Client(client.id), client.connected_drone_ids.clone());
            host_ids.insert(client.id);
        }
        for server in &self.servers {
            network_map.insert(NetNode::Server(server.id), server.connected_drone_ids.clone());
            host_ids.insert(server.id);
        }
        for drone in &self.drones {
            network_map.insert(NetNode::Drone(drone.id), drone.connected_node_ids.clone());
        }

        // Build drone-only graph (excluding hosts)
        let mut drone_graph: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for (node, neighbors) in &network_map {
            if let NetNode::Drone(drone_id) = node {
                let drone_neighbors: Vec<NodeId> = neighbors
                    .iter()
                    .filter(|id| !host_ids.contains(id))
                    .cloned()
                    .collect();
                drone_graph.insert(*drone_id, drone_neighbors);
            }
        }

        NetworkState {
            node_map: network_map,
            drone_graph,
            host_ids,
        }
    }
    fn build_network_map(&mut self) -> NodeMap {
        let mut network_map: HashMap<NetNode, Vec<NodeId>> = HashMap::new();
        for client in self.clients.iter() {
            network_map.insert(NetNode::Client(client.id), client.connected_drone_ids.clone());
        }
        for server in self.servers.iter() {
            network_map.insert(NetNode::Server(server.id), server.connected_drone_ids.clone());
        }
        for drone in self.drones.iter() {
            network_map.insert(NetNode::Drone(drone.id), drone.connected_node_ids.clone());
        }
        self.logs.push_sc_log(LogType::DEBUG, format!("{:?}", network_map));
        network_map
    }
    fn is_graph_connected_excluding(
        &self,
        graph: &HashMap<NodeId, Vec<NodeId>>,
        exclude_node: Option<NodeId>,
    ) -> bool
    {
        if graph.is_empty() {
            return true;
        }

        // Find first non-excluded node as starting point - match original logic
        let start_node = match graph.keys().find(|&&id| Some(id) != exclude_node) {
            Some(&node) => node,
            None => return true, // No nodes to check
        };

        let mut visited = HashSet::new();
        let mut stack = vec![start_node];

        while let Some(current) = stack.pop() {
            if Some(current) == exclude_node || visited.contains(&current) {
                continue;
            }

            visited.insert(current);

            if let Some(neighbors) = graph.get(&current) {
                for &neighbor in neighbors {
                    if Some(neighbor) != exclude_node && !visited.contains(&neighbor) {
                        stack.push(neighbor);
                    }
                }
            }
        }

        // Check connectivity exactly like original: visited.len() + 1 != graph.len()
        // But adjust for exclude_node presence
        let expected_count = if exclude_node.is_some() && graph.contains_key(&exclude_node.unwrap()) {
            graph.len() - 1
        } else {
            graph.len()
        };

        visited.len() == expected_count
    }
    fn cannot_crash(&mut self, drone_id: NodeId) -> Result<bool, String> {
        let network_state = self.build_network_state();

        if network_state.node_map.len() <= 1 {
            return Ok(false);
        }

        // Check if drone exists
        if !network_state.node_map.contains_key(&NetNode::Drone(drone_id)) {
            return Err(format!("Drone {drone_id} is not in the net!"));
        }

        // Get drone's neighbors to simulate removal
        //let drone_neighbors = network_state.node_map.get(&NetNode::Drone(drone_id)).unwrap();

        // Check host constraints after removing drone connections
        for (node, neighbors) in &network_state.node_map {
            match node {
                NetNode::Client(client_id) => {
                    let filtered_neighbors: Vec<NodeId> = neighbors
                        .iter()
                        .filter(|id| **id != drone_id)
                        .cloned()
                        .collect();
                    let adj = filtered_neighbors.len();
                    if adj <= 0 || adj > 2 {
                        return Err(format!(
                            "Cannot Crash Drone {drone_id} because Client {client_id} would have {adj} Neighbors!"
                        ));
                    }
                }
                NetNode::Server(server_id) => {
                    let filtered_neighbors: Vec<NodeId> = neighbors
                        .iter()
                        .filter(|id| **id != drone_id)
                        .cloned()
                        .collect();
                    let adj = filtered_neighbors.len();
                    if adj <= 1 {
                        return Err(format!(
                            "Cannot Crash Drone {drone_id} because Server {server_id} would have only {adj} Neighbors!"
                        ));
                    }
                }
                _ => continue,
            }
        }

        // Check if removing drone disconnects the drone network
        let is_connected = self.is_graph_connected_excluding(&network_state.drone_graph, Some(drone_id));

        // Return true if crashing would disconnect the network
        Ok(!is_connected)
    }

    fn cannot_remove(&mut self, first: NetNode, second: NetNode) -> Result<bool, String> {
        let network_state = self.build_network_state();

        // Validate both nodes exist and are connected - match original error messages
        let first_neighbors = network_state.node_map.get(&first)
            .ok_or_else(|| format!("{} {} is not in the map!", first.node_type(), first.id()))?;

        if !first_neighbors.contains(second.id()) {
            return Err(format!(
                "{} {} is not connected to {} {}!",
                first.node_type(), first.id(),
                second.node_type(), second.id()
            ));
        }

        let second_neighbors = network_state.node_map.get(&second)
            .ok_or_else(|| format!("{} {} is not in the map!", second.node_type(), second.id()))?;

        if !second_neighbors.contains(first.id()) {
            return Err(format!(
                "{} {} is not connected to {} {}!",
                second.node_type(), second.id(),
                first.node_type(), first.id()
            ));
        }

        // Check host constraints after removing connection - match original logic
        for (node, neighbors) in &network_state.node_map {
            let mut adj = neighbors.len();

            // Simulate removing the connection
            if *node == first && neighbors.contains(second.id()) {
                adj -= 1;
            } else if *node == second && neighbors.contains(first.id()) {
                adj -= 1;
            }

            match node {
                NetNode::Client(client_id) => {
                    if adj <= 0 || adj > 2 {
                        return Err(format!(
                            "Connection cannot be removed because Client {client_id} would have {adj} Neighbors!"
                        ));
                    }
                }
                NetNode::Server(server_id) => {
                    if adj <= 1 {
                        return Err(format!(
                            "Connection cannot be removed because Server {server_id} would have only {adj} Neighbors!"
                        ));
                    }
                }
                _ => continue,
            }
        }

        // Create modified drone graph without this edge and check connectivity
        let mut modified_drone_graph = network_state.drone_graph.clone();

        if let NetNode::Drone(first_id) = first {
            if let Some(neighbors) = modified_drone_graph.get_mut(&first_id) {
                neighbors.retain(|&id| id != *second.id());
            }
        }
        if let NetNode::Drone(second_id) = second {
            if let Some(neighbors) = modified_drone_graph.get_mut(&second_id) {
                neighbors.retain(|&id| id != *first.id());
            }
        }

        // Add debug log like original
        self.logs.push_sc_log(LogType::DEBUG, format!("{:?}", modified_drone_graph));

        let is_connected = self.is_graph_connected_excluding(&modified_drone_graph, None);
        Ok(!is_connected) // Return true if removal would disconnect the network
    }

    fn cannot_add(&mut self, first: NetNode, second: NetNode) -> Result<bool, String> {
        let network_map = self.build_network_map(); // Use original function to maintain exact behavior

        // Check if nodes exist in the map - match original error messages
        if !network_map.contains_key(&first) {
            return Err(format!("{} {} is not in the map!", first.node_type(), first.id()));
        }
        if !network_map.contains_key(&second) {
            return Err(format!("{} {} is not in the map!", second.node_type(), second.id()));
        }

        // Simulate adding the connection
        let mut modified_net_map = network_map.clone();

        if let Some(first_neighbors) = modified_net_map.get_mut(&first) {
            first_neighbors.push(*second.id());
        }
        if let Some(second_neighbors) = modified_net_map.get_mut(&second) {
            second_neighbors.push(*first.id());
        }

        // Check host constraints - match original logic exactly
        for (node, neighbors) in modified_net_map.iter() {
            match node {
                NetNode::Client(client_id) => {
                    let adj = neighbors.len();
                    if adj <= 0 || adj > 2 {
                        return Err(format!(
                            "Connection cannot be removed because Client {client_id} would have {adj} Neighbors!"
                        ));
                    }
                }
                _ => continue,
            }
        }

        Ok(false)
    }

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
        self.logs.push_sc_log(LogType::DEBUG, format!("Trying to Crash Drone: {drone_id}"));
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

                        // ———> qui rimuovo COMPLETAMENTE il drone dallo stato
                        self.drones.retain(|d| d.id != drone_id);
                        self.drone_command_sender.remove(&drone_id);
                        self.packet_sender.remove(&drone_id);
                        // se tieni altri mappi (es. self.running_network, self.stats, ecc.)
                        // rimuovi anche lì drone_id

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
                //let _sender_id = packet.routing_header.hops[packet.routing_header.hop_index - 1 ];
                //handle_packet(&packet);
                let _session_id = packet.session_id;
                self.stats.received_packet();
                Ok(())
                //self.logs.push_sc_log(LogType::INFO, format!("Drone ID {:?} - Sent packet [ Origin: {sender_id}, Session ID: {session_id} ]", packet.routing_header.hops[packet.routing_header.hop_index - 1]).as_str());
            },
            DroneEvent::PacketDropped(packet) => {
                //let _sender_id = packet.routing_header.hops[packet.routing_header.hop_index - 1 ];
                //handle_packet(&packet);
                let _session_id = packet.session_id;
                Ok(())
                //self.logs.push_sc_log(LogType::INFO, format!("Drone ID {:?} - Dropped packet [ Origin: {sender_id}, Session ID: {session_id}", packet.routing_header.hops[packet.routing_header.hop_index - 1]).as_str());
            },
            DroneEvent::ControllerShortcut(packet) => {
                //let _sender_id = packet.routing_header.hops[packet.routing_header.hop_index - 1 ];
                let receiver_id = packet.routing_header.hops.get(packet.routing_header.hops.len() - 1).unwrap();
                match self.controller_shortcut(*receiver_id, packet) {
                    Ok(()) => Ok(()),
                    Err(e) => Err((LogType::ERROR, e)),
                }

            },
        }
    }

    /// - Nodes Commands
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
                                match forward_drone_command_sender.send(forward_command) {
                                    Ok(()) => {
                                        self.logs.push_sc_log(LogType::DEBUG, format!("Asking to add {} {} Sender to {} {first_id}", second_node.node_type(), second_node.id(), first_node.node_type()));
                                    },
                                    Err(e) => return Err(e.to_string()),
                                }
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
                                match forward_client_command_sender.send(forward_command) {
                                    Ok(()) => {
                                        self.logs.push_sc_log(LogType::DEBUG, format!("Asking to add {} {} Sender to {} {first_id}", second_node.node_type(), second_node.id(), first_node.node_type()));
                                    },
                                    Err(e) => return Err(e.to_string()),
                                }
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
                                match forward_server_command_sender.send(forward_command) {
                                    Ok(()) => {
                                        self.logs.push_sc_log(LogType::DEBUG, format!("Asking to add {} {} Sender to {} {first_id}", second_node.node_type(), second_node.id(), first_node.node_type()));
                                    },
                                    Err(e) => return Err(e.to_string()),
                                }
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
                        match forward_drone_command_sender.send(forward_command) {
                            Ok(()) => {
                                self.logs.push_sc_log(LogType::DEBUG, format!("Asking to remove {} {} Sender from {} {first_id}", second_node.node_type(), second_node.id(), first_node.node_type()));
                            },
                            Err(e) => return Err(e.to_string()),
                        }
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
                        match forward_client_command_sender.send(forward_command) {
                            Ok(()) => {
                                self.logs.push_sc_log(LogType::DEBUG, format!("Asking to remove {} {} Sender from {} {first_id}", second_node.node_type(), second_node.id(), first_node.node_type()));
                            },
                            Err(e) => return Err(e.to_string()),
                        }
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
                        match forward_server_command_sender.send(forward_command) {
                            Ok(()) => {
                                self.logs.push_sc_log(LogType::DEBUG, format!("Asking to remove {} {} Sender from {} {first_id}", second_node.node_type(), second_node.id(), first_node.node_type()));
                            },
                            Err(e) => return Err(e.to_string()),
                        }
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
                self.logs.push_sc_log(LogType::DEBUG, format!("Failed to register client: {}", e));
            }
        }

        self.logs.push_sc_log(LogType::DEBUG, format!("Sending Client List Request to Client {client_id} for Server {server_id}"));
        let command = Net(ClientList(server_id));
        self.send_client_command(client_id, command)
    }

    pub fn message_for(&mut self, sender_client_id: ClientId, receiver_client_id: ClientId, server_id: ServerId, message: String) -> Result<(), String> {
        self.logs.push_sc_log(LogType::DEBUG, format!("Sending Message For Request to Client {sender_client_id} for Server {server_id} to Client {receiver_client_id}"));
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
        self.logs.push_sc_log(LogType::DEBUG, format!("Sending Media For Request to Client {sender_client_id} for Server {server_id} to Client {receiver_client_id}"));
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
        self.logs.push_sc_log(LogType::DEBUG, format!("Sending File For Request to Client {sender_client_id} for Server {server_id} to Client {receiver_client_id}"));
        let command = Net(FileFor(receiver_client_id, server_id, file_id, file));
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
                self.handle_packet_reconstructed(server_to_client)?
            },
            HostEvent::PacketSent(_packet) => {
                self.stats.received_packet();
            },
            HostEvent::ServersDiscovered(client_id, servers) => {
                if let Some(chat_servers) = servers.get(&Chat) {
                    self.chat_cache
                        .known_chat_servers
                        .insert(client_id, chat_servers.clone());
                }
                if let Some(content_servers) = servers.get(&Content) {
                    self.net_cache
                        .known_content_servers
                        .insert(client_id, content_servers.clone());
                }
                self.logs.push_sc_log(LogType::INFO, format!("Servers discovered: {servers:?}"));
            },
            HostEvent::ChatUpdate(server_id, string) => {
                let chat: Chat = serde_json::from_str(&string)
                    .map_err(|e| (LogType::ERROR, e.to_string()))?;

                let key = (chat.users.user1, chat.users.user2);
                let mut chats_stored = HashMap::new();
                chats_stored.insert(key, chat);

                let chat_list = ChatList { chats_stored };
                self.chat_cache.local_chats.insert(server_id, chat_list);
            }
            HostEvent::DownloadingInfo(client_id, percentage) => {
                /*let server_id = 0;
                let media_id: MediaId = String::new();

                let server_map = match self.net_cache.downloaded_medias.get_mut(&server_id) {
                    Some(map) => map,
                    None => {
                        return Err((LogType::ERROR, format!("Server ID {server_id} not found in downloaded_medias")));
                    }
                };

                match server_map.get_mut(&media_id) {
                    Some((channels, net_cache_media)) => {
                        let (resize_request_sender, resize_request_receiver) = mpsc::channel();
                        let (render_update_sender, render_update_receiver) = mpsc::channel();
                        let media_channels = (Some(resize_request_sender), Some(render_update_receiver));
                        let renderer_channels = (render_update_sender, resize_request_receiver);

                        match self.tx_to_renderer.as_ref() {
                            Some(sender) => {
                                if let Err(e) = sender.send(RendererCommand::AddChannel(renderer_channels)) {
                                    return Err((LogType::ERROR, e.to_string()));
                                }
                            },
                            None => {
                                self.logs.push_sc_log(LogType::ERROR, "No tx_to_render sender found".to_string());
                            }
                        }

                        *channels = media_channels;
                        *net_cache_media = NetCacheMedia::Downloading(percentage as f32);
                    }
                    None => {
                        return Err((LogType::ERROR, format!("Media ID {media_id} entry for NetCacheMedia not found")));
                    }
                }*/
            },
            HostEvent::UploadingInfo(client_id, percentage) => {},
            HostEvent::ExpectedPDR(expected_pdr) => {},
            HostEvent::NetSpeed(upload, download) => {},
            HostEvent::UsageCPU(usage) => {},
        }
        Ok(())
    }

    fn handle_packet_reconstructed(&mut self, server_to_client: ServerToClient) -> Result<(), (LogType, String)> {
        match server_to_client {
            ServerToClient::ServerType(server_type)
            => {
                self.logs.push_sc_log(LogType::INFO, format!("Server type: {:?}", server_type)) },
            ServerToClient::FilesList(client_id, server_id, file_list)
            => {
                self.net_cache
                    .known_file_list
                    .entry(client_id)
                    .or_default()
                    .insert(server_id, file_list.clone());
                self.logs.push_sc_log(LogType::INFO, format!("Files list from Server {server_id} received: {file_list:?}")) },
            ServerToClient::File(server_id, file_id, file)
            => {

                /*let (resize_request_sender, resize_request_receiver) = mpsc::channel();
                let (render_update_sender, render_update_receiver) = mpsc::channel();
                let media_channels = (Some(resize_request_sender), Some(render_update_receiver));
                let renderer_channels = (render_update_sender, resize_request_receiver);
                let net_cache_media = NetCacheMedia::Downloaded(file);
                let mut media_map = HashMap::new();
                media_map.insert(file_id, (media_channels, net_cache_media));
                match self.tx_to_renderer.as_ref() {
                    Some(sender) => {
                        match sender.send(RendererCommand::AddChannel(renderer_channels)) {
                            Ok(_) => {},
                            Err(e) => {return Err((LogType::ERROR, e.to_string()))}
                        }
                    },
                    None => {
                        self.logs.push_sc_log(LogType::ERROR, "No tx_to_render sender found".to_string());
                    }
                }
                self.net_cache.downloaded_medias.insert(server_id, media_map);*/

                // Construct filename based on file ID and inferred extension
                /*let path = PathBuf::from("assets").join(&file_id);

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
*/
            },
            ServerToClient::MediaList(client_id, server_id, media_list)
            => {
                self.net_cache
                    .known_media_list
                    .entry(client_id)
                    .or_default()
                    .insert(server_id, media_list.clone());
                self.logs.push_sc_log(LogType::INFO, format!("Media list from Server {server_id} received: {:?}", media_list)) },
            ServerToClient::Media(server_id, media_id, image)
            => {
                let server_map = match self.net_cache.downloaded_medias.get_mut(&server_id) {
                    Some(map) => map,
                    None => {
                        return Err((LogType::ERROR, format!("Server ID {server_id} not found in downloaded_medias")));
                    }
                };

                match server_map.get_mut(&media_id) {
                    Some((channels, net_cache_media)) => {
                        let (resize_request_sender, resize_request_receiver) = mpsc::channel();
                        let (render_update_sender, render_update_receiver) = mpsc::channel();
                        let media_channels = (Some(resize_request_sender), Some(render_update_receiver));
                        let renderer_channels = (render_update_sender, resize_request_receiver);

                        match self.tx_to_renderer.as_ref() {
                            Some(sender) => {
                                if let Err(e) = sender.send(RendererCommand::AddChannel(renderer_channels)) {
                                    return Err((LogType::ERROR, e.to_string()));
                                }
                            },
                            None => {
                                self.logs.push_sc_log(LogType::ERROR, "No tx_to_render sender found".to_string());
                            }
                        }

                        *channels = media_channels;
                        *net_cache_media = NetCacheMedia::Downloaded(image);
                    }
                    None => {
                        return Err((LogType::ERROR, format!("Media ID {media_id} entry for NetCacheMedia not found")));
                    }
                }

                //----------------------------------------------------------------------
                /*let extension = match get_extension(&media_id) {
                    Some(extension) => extension,
                    None => return Err((LogType::INFO, format!("Unknown extension for file {media_id}"))),
                };
                let path = format!("assets/{}", media_id);
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
                }*/
            },
            ServerToClient::ClientList(client_id, server_id, client_list)
            => {
                let contacts: Vec<NodeId> = client_list.iter().filter(|id| **id != client_id).cloned().collect();
                self.chat_cache
                    .known_contacts
                    .entry(client_id)
                    .or_default()
                    .insert(server_id, contacts);
                self.logs.push_sc_log(LogType::INFO, format!("Client list from CatServer {server_id} received: {:?}", client_list)) },
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
            ServerToClient::Sending(content_id, d) => {}
            ServerToClient::WrongServer(server_id)
            => {
                self.logs.push_sc_log(LogType::INFO, format!("Wrong Server ID: {server_id}"));
            }
            ServerToClient::ErrNotFound(server_id)
            => return Err((LogType::ERROR, format!("Err Not Found by Server {server_id}"))),
            ServerToClient::ErrClientId(server_id)
            => return Err((LogType::ERROR, format!("Err Client Id Not Found from Server {server_id}"))),
        }
        Ok(())
    }

    pub fn handle_log_event(&mut self, log: String) {
        self.logs.push_log(LogType::INFO, log.as_str())
    }

    pub fn handle_rx_renderer(&mut self){
        match &self.rx_from_renderer{
            None => {}
            Some(channel) => {
                while let Ok(event) = channel.try_recv() {
                    match event{
                        RendererResponse::Outcome((logtype,log)) => {
                            self.logs.push_log(logtype,log.as_str());
                        }
                        RendererResponse::Encoded((enc_server, enc_media, enc_proto)) => {
                            self.logs.push_sc_log(LogType::ERROR,"Received encoded Response".to_string());
                            match self.net_cache.downloaded_medias.get_mut(&enc_server) {
                                None => {

                                }
                                Some(hash) => {
                                    match hash.remove(&enc_media){
                                        None => {}
                                        Some(((ch1,ch2),net)) => {
                                            hash.insert(enc_media, ((ch1,ch2), NetCacheMedia::Rendered(enc_proto)));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Populate the TopologyAlgorithm.map_links field from the running_network,
    /// inserting each undirected edge exactly once.
    pub fn populate_topology_links(&mut self)-> Vec<(NodeId, NodeId)>{
        use std::collections::HashSet;
        // Collect unique, undirected links
        let mut links = HashSet::new();
        for (net_node, neighbors) in &self.running_network {
            // Extract the NodeId from NetNode
            let id = match net_node {
                NetNode::Drone(id) | NetNode::Client(id) | NetNode::Server(id) => *id,
            };
            for &neighbor in neighbors {
                // Order the pair to avoid duplicates (min, max)
                let (a, b) = if id <= neighbor { (id, neighbor) } else { (neighbor, id) };
                links.insert((a, b));
            }
        }
        links.into_iter().collect()
    }
}


fn get_extension(media_id: &MediaId) -> Option<&str> {
    media_id.rsplitn(2, '.').next().filter(|&ext| ext != media_id)
}

fn get_filename<P: AsRef<Path>>(path: P) -> Option<String> {
    path.as_ref()
        .file_name()
        .and_then(|name| name.to_str())
        .map(|s| s.to_string())
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