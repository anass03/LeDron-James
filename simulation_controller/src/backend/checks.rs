use std::collections::{HashMap, HashSet};
use log::debug;
use wg_2024::network::NodeId;
use crate::backend::sc_models::{NetNode, LogType, NetworkState, NodeMap};
use crate::backend::simulation_controller::SimulationControllerData;

impl SimulationControllerData {
    fn build_network_state(&self) -> NetworkState {
        let mut network_map: HashMap<NetNode, Vec<NodeId>> = HashMap::with_capacity(
            self.clients.len() + self.servers.len() + self.drones.len()
        );
        let mut host_ids = HashSet::with_capacity(self.clients.len() + self.servers.len());

        // Build network map and collect host IDs
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

        // Build drone-only graph
        let mut drone_graph: HashMap<NodeId, Vec<NodeId>> = HashMap::with_capacity(self.drones.len());
        for drone in &self.drones {
            let drone_neighbors: Vec<NodeId> = drone.connected_node_ids
                .iter()
                .filter(|id| !host_ids.contains(id))
                .copied()
                .collect();
            drone_graph.insert(drone.id, drone_neighbors);
        }

        NetworkState {
            node_map: network_map,
            drone_graph,
            host_ids,
        }
    }

    fn build_network_map(&mut self) -> NodeMap {
        let mut network_map: HashMap<NetNode, Vec<NodeId>> = HashMap::with_capacity(
            self.clients.len() + self.servers.len() + self.drones.len()
        );

        for client in &self.clients {
            network_map.insert(NetNode::Client(client.id), client.connected_drone_ids.clone());
        }
        for server in &self.servers {
            network_map.insert(NetNode::Server(server.id), server.connected_drone_ids.clone());
        }
        for drone in &self.drones {
            network_map.insert(NetNode::Drone(drone.id), drone.connected_node_ids.clone());
        }

        debug!(target: "main", "{:?}", network_map);
        network_map
    }

    // Connectivity check using iterative DFS
    fn is_graph_connected_excluding(
        &self,
        graph: &HashMap<NodeId, Vec<NodeId>>,
        exclude_node: Option<NodeId>,
    ) -> bool
    {
        if graph.is_empty() {
            return true;
        }

        // Find first non-excluded node as starting point
        let start_node = match graph.keys().find(|&&id| Some(id) != exclude_node) {
            Some(&node) => node,
            None => return true, // No nodes to check
        };

        let expected_count = if exclude_node.is_some() && graph.contains_key(&exclude_node.unwrap()) {
            graph.len() - 1
        } else {
            graph.len()
        };

        // Early exit if only one node to visit
        if expected_count <= 1 {
            return true;
        }

        let mut visited = HashSet::with_capacity(expected_count);
        let mut stack = Vec::with_capacity(graph.len()); // Pre-allocate with reasonable capacity
        stack.push(start_node);

        while let Some(current) = stack.pop() {
            if Some(current) == exclude_node || visited.contains(&current) {
                continue;
            }

            visited.insert(current);

            // Early exit if all visited
            if visited.len() == expected_count {
                return true;
            }

            if let Some(neighbors) = graph.get(&current) {
                for &neighbor in neighbors {
                    if Some(neighbor) != exclude_node && !visited.contains(&neighbor) {
                        stack.push(neighbor);
                    }
                }
            }
        }

        visited.len() == expected_count
    }

    pub(crate) fn cannot_crash(&mut self, drone_id: NodeId) -> Result<bool, String> {
        // Existence check before building network state
        if !self.drones.iter().any(|d| d.id == drone_id) {
            return Err(format!("Drone {drone_id} is not in the net!"));
        }

        let network_state = self.build_network_state();

        if network_state.node_map.len() <= 1 {
            return Ok(false);
        }

        // Check host constraints after removing drone connections
        for (node, neighbors) in &network_state.node_map {
            match node {
                NetNode::Client(client_id) => {
                    let remaining_connections = neighbors.iter()
                        .filter(|id| **id != drone_id)
                        .count();

                    if remaining_connections == 0 || remaining_connections > 2 {
                        return Err(format!(
                            "Cannot Crash Drone {drone_id} because Client {client_id} would have {remaining_connections} Neighbors!"
                        ));
                    }
                }
                NetNode::Server(server_id) => {
                    let remaining_connections = neighbors.iter()
                        .filter(|id| **id != drone_id)
                        .count();

                    if remaining_connections <= 1 {
                        return Err(format!(
                            "Cannot Crash Drone {drone_id} because Server {server_id} would have only {remaining_connections} Neighbors!"
                        ));
                    }
                }
                _ => continue,
            }
        }

        // Check if removing drone disconnects the drone network
        let is_connected = self.is_graph_connected_excluding(&network_state.drone_graph, Some(drone_id));
        Ok(!is_connected)
    }

    pub(crate) fn cannot_remove(&mut self, first: NetNode, second: NetNode) -> Result<bool, String> {
        let network_state = self.build_network_state();

        // Validate both nodes exist and are connected
        let first_neighbors = network_state.node_map.get(&first)
            .ok_or_else(|| format!("{} {} is not in the map!", first.node_type(), first.id()))?;

        check_connection(first_neighbors, second, first)?;

        let second_neighbors = network_state.node_map.get(&second)
            .ok_or_else(|| format!("{} {} is not in the map!", second.node_type(), second.id()))?;

        check_connection(second_neighbors, first, second)?;

        // Check host constraints after removing connection
        for (node, neighbors) in &network_state.node_map {
            let mut remaining_connections = neighbors.len();

            // Simulate removing the connection
            if (*node == first && neighbors.contains(second.id())) ||
                (*node == second && neighbors.contains(first.id())) {
                remaining_connections -= 1;
            }

            match node {
                NetNode::Client(client_id) => {
                    if remaining_connections == 0 || remaining_connections > 2 {
                        return Err(format!(
                            "Connection cannot be removed because Client {client_id} would have {remaining_connections} Neighbors!"
                        ));
                    }
                }
                NetNode::Server(server_id) => {
                    if remaining_connections <= 1 {
                        return Err(format!(
                            "Connection cannot be removed because Server {server_id} would have only {remaining_connections} Neighbors!"
                        ));
                    }
                }
                _ => continue,
            }
        }

        // Create modified drone graph and check connectivity only if both nodes are drones
        if let (NetNode::Drone(first_id), NetNode::Drone(second_id)) = (first, second) {
            let mut modified_drone_graph = network_state.drone_graph.clone();

            // Remove the edge from both directions
            if let Some(neighbors) = modified_drone_graph.get_mut(&first_id) {
                neighbors.retain(|&id| id != second_id);
            }
            if let Some(neighbors) = modified_drone_graph.get_mut(&second_id) {
                neighbors.retain(|&id| id != first_id);
            }

            self.logs.push_sc_log(LogType::DEBUG, format!("{:?}", modified_drone_graph));

            let is_connected = self.is_graph_connected_excluding(&modified_drone_graph, None);
            return Ok(!is_connected);
        }

        // If not both drones, the removal doesn't affect drone connectivity
        Ok(false)
    }

    pub(crate) fn cannot_add(&mut self, first: NetNode, second: NetNode) -> Result<bool, String> {
        let network_map = self.build_network_map();

        // Check if nodes exist in the map
        if !network_map.contains_key(&first) {
            return Err(format!("{} {} is not in the map!", first.node_type(), first.id()));
        }
        if !network_map.contains_key(&second) {
            return Err(format!("{} {} is not in the map!", second.node_type(), second.id()));
        }

        // Only check client constraints (servers have no upper limit in add operations)
        match (&first, &second) {
            (NetNode::Client(client_id), _) => {
                let current_connections = network_map.get(&first).unwrap().len();
                if current_connections + 1 > 2 {
                    return Err(format!(
                        "Connection cannot be added because Client {client_id} would have {} Neighbors!",
                        current_connections + 1
                    ));
                }
            }
            (_, NetNode::Client(client_id)) => {
                let current_connections = network_map.get(&second).unwrap().len();
                if current_connections + 1 > 2 {
                    return Err(format!(
                        "Connection cannot be added because Client {client_id} would have {} Neighbors!",
                        current_connections + 1
                    ));
                }
            }
            _ => {} // No constraints for drone-drone or server connections in add operations
        }

        Ok(false)
    }
}

fn check_connection(neighbors: &Vec<NodeId>, first: NetNode, second: NetNode) -> Result<(), String> {
    if !neighbors.contains(first.id()) {
        Err(format!(
            "{} {} is not connected to {} {}!",
            second.node_type(), second.id(),
            first.node_type(), first.id()
        ))
    } else { Ok(()) }
}