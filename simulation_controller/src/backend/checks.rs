use std::collections::{HashMap, HashSet};
use log::debug;
use wg_2024::network::NodeId;
use crate::backend::sc_models::{LogType, NetworkState, NodeMap};
use crate::backend::simulation_controller::{NetNode, SimulationControllerData};

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

impl SimulationControllerData {
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
        debug!(target: "main", "{:?}", network_map);
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
    pub(crate) fn cannot_crash(&mut self, drone_id: NodeId) -> Result<bool, String> {
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

    pub(crate) fn cannot_remove(&mut self, first: NetNode, second: NetNode) -> Result<bool, String> {
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

    pub(crate) fn cannot_add(&mut self, first: NetNode, second: NetNode) -> Result<bool, String> {
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
}