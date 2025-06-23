use std::collections::HashMap;
use wg_2024::config::Config;
use wg_2024::network::NodeId;
use std::collections::HashSet;

pub fn parse_and_check_config(path: &str) -> Result<Config, String> {
    // Parse config file
    let config = match std::fs::read_to_string(path) {
        Ok(file_str) => toml::from_str::<Config>(&file_str)
            .map_err(|e| format!("Error parsing string file: {}", e))?,
        Err(e) => return Err(format!("Error reading config file: {}", e)),
    };

    // Create maps
    let drone_map: HashMap<_, _> = config.drone.iter().map(|d| (d.id, d)).collect();
    let client_map: HashMap<_, _> = config.client.iter().map(|c| (c.id, c)).collect();
    let server_map: HashMap<_, _> = config.server.iter().map(|s| (s.id, s)).collect();

    let mut errors = Vec::new();

    // Check for duplicate IDs across all node types
    let mut id_set = HashSet::new();
    for &id in drone_map.keys().chain(client_map.keys()).chain(server_map.keys()) {
        if !id_set.insert(id) {
            errors.push(format!("Duplicate NodeId found: {}\n", id));
        }
    }

    // Validate drones
    for drone in &config.drone {
        // Check PDR bounds
        if !(0.0..1.0).contains(&drone.pdr) {
            let msg = if drone.pdr > 1.0 { "too high" } else { "negative" };
            errors.push(format!("Drone {} pdr is {}: {} | 0.0 < pdr < 1.0\n", drone.id, msg, drone.pdr));
        }

        // Check connections
        for &node_id in &drone.connected_node_ids {
            let back_link_exists = match (client_map.get(&node_id), server_map.get(&node_id), drone_map.get(&node_id)) {
                (Some(client), _, _) => client.connected_drone_ids.contains(&drone.id),
                (_, Some(server), _) => server.connected_drone_ids.contains(&drone.id),
                (_, _, Some(conn_drone)) => conn_drone.connected_node_ids.contains(&drone.id),
                _ => {
                    errors.push(format!("Node {} connected to Drone {} does not exist\n", node_id, drone.id));
                    continue;
                }
            };

            if !back_link_exists {
                let node_type = if client_map.contains_key(&node_id) { "Client" }
                else if server_map.contains_key(&node_id) { "Server" }
                else { "Drone" };
                errors.push(format!("{} {} does not link back to Drone {}\n", node_type, node_id, drone.id));
            }
        }
    }

    // Validate clients
    for client in &config.client {
        let neighbors = client.connected_drone_ids.len();
        match neighbors {
            0 => errors.push(format!("Client {} has no neighbors", client.id)),
            n if n > 2 => errors.push(format!("Client {} has too many neighbors: {:?}", client.id, client.connected_drone_ids)),
            _ => {}
        }

        for &drone_id in &client.connected_drone_ids {
            match drone_map.get(&drone_id) {
                Some(drone) if !drone.connected_node_ids.contains(&client.id) => {
                    errors.push(format!("Drone {} does not link back to Client {}\n", drone_id, client.id));
                }
                None => errors.push(format!("Client {} refers to unknown Drone {}\n", client.id, drone_id)),
                _ => {}
            }
        }
    }

    // Validate servers
    for server in &config.server {
        if server.connected_drone_ids.len() < 2 {
            errors.push(format!("Server {} must be connected to at least 2 drones, found {}\n",
                                server.id, server.connected_drone_ids.len()));
        }

        for &drone_id in &server.connected_drone_ids {
            if client_map.contains_key(&drone_id) || server_map.contains_key(&drone_id) {
                errors.push(format!("Server {} must only connect to drones, found link to non-drone node {}\n",
                                    server.id, drone_id));
                continue;
            }

            match drone_map.get(&drone_id) {
                Some(drone) if !drone.connected_node_ids.contains(&server.id) => {
                    errors.push(format!("Drone {} does not link back to Server {}\n", drone_id, server.id));
                }
                None => errors.push(format!("Server {} refers to unknown Drone {}\n", server.id, drone_id)),
                _ => {}
            }
        }
    }

    // Check drone network connectivity
    if let Some(first_drone) = config.drone.first() {
        let drone_graph: HashMap<_, _> = config.drone.iter()
            .map(|d| (d.id, d.connected_node_ids.iter()
                .filter(|&&id| drone_map.contains_key(&id))
                .copied()
                .collect::<Vec<_>>()))
            .collect();

        let mut visited = HashSet::new();
        let mut stack = vec![first_drone.id];

        while let Some(node) = stack.pop() {
            if visited.insert(node) {
                if let Some(neighbors) = drone_graph.get(&node) {
                    stack.extend(neighbors.iter().filter(|&&n| !visited.contains(&n)));
                }
            }
        }

        if visited.len() != config.drone.len() {
            errors.push("The drone network is not fully connected (check for subnet gaps)\n".to_string());
        }
    }

    if errors.is_empty() { Ok(config) } else { Err(errors.join(" | ")) }
}