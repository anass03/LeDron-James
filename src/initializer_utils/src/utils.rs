use std::collections::HashMap;
use wg_2024::config::Config;
use wg_2024::network::NodeId;
use std::collections::HashSet;
//add check client server connections as protocol
//check if the net is entirely connected
pub fn parse_and_check_config(path: &String) -> Result<Config, String> {
    let config;
    match std::fs::read_to_string(path) {
        Ok(file_str) => {
            match toml::from_str::<Config>(file_str.as_str()) {
                Ok(temp) => {
                    config = temp;
                },
                Err(error) => {
                    return Err(format!("Error parsing string file: {}", error.to_string()));
                }
            }
        },
        Err(e) => return Err(format!("Error reading config file: {}", e)),
    }
    // Passed all parsing

    let drone_map: HashMap<_, _> = config.drone.iter().map(|d| (d.id, d)).collect();
    let client_map: HashMap<_, _> = config.client.iter().map(|c| (c.id, c)).collect();
    let server_map: HashMap<_, _> = config.server.iter().map(|s| (s.id, s)).collect();

    let mut errors = Vec::new();

    // Unique ID check across all node types
    let mut id_set = HashSet::new();
    for drone in &config.drone {
        if !id_set.insert(drone.id) {
            errors.push(format!("Duplicate NodeId found: {}\n", drone.id));
        }
    }
    for client in &config.client {
        if !id_set.insert(client.id) {
            errors.push(format!("Duplicate NodeId found: {}\n", client.id));
        }
    }
    for server in &config.server {
        if !id_set.insert(server.id) {
            errors.push(format!("Duplicate NodeId found: {}\n", server.id));
        }
    }

    // Existing check: drone -> node
    for drone in &config.drone {
        if drone.pdr > 1.0 {
            errors.push(format!("Drone {} pdr is too high: {} | 0.0 < pdr < 1.0\n", drone.id,  drone.pdr));
        } else if drone.pdr < 0.0 {
            errors.push(format!("Drone {} pdr is negative: {} | 0.0 < pdr < 1.0\n", drone.id,  drone.pdr));
        }
        for node_id in &drone.connected_node_ids {
            if let Some(client) = client_map.get(node_id) {
                if !client.connected_drone_ids.contains(&drone.id) {
                    errors.push(format!("Client {} does not link back to Drone {}\n", client.id, drone.id));
                }
            } else if let Some(server) = server_map.get(node_id) {
                if !server.connected_drone_ids.contains(&drone.id) {
                    errors.push(format!("Server {} does not link back to Drone {}\n", server.id, drone.id));
                }
            } else if let Some(conn_drone) = drone_map.get(node_id) {
                if !conn_drone.connected_node_ids.contains(&drone.id) {
                    errors.push(format!("Drone {} does not link back to Drone {}\n", conn_drone.id, drone.id));
                }
            } else {
                errors.push(format!("Node {} connected to Drone {} does not exist\n", node_id, drone.id));
            }
        }
    }

    // Existing check: client -> drone
    for client in &config.client {
        for drone_id in &client.connected_drone_ids {
            if let Some(drone) = drone_map.get(drone_id) {
                if !drone.connected_node_ids.contains(&client.id) {
                    errors.push(format!("Drone {} does not link back to Client {}\n", drone.id, client.id));
                }
            } else {
                errors.push(format!("Client {} refers to unknown Drone {}\n", client.id, drone_id));
            }
        }
    }

    // Existing check: server -> drone
    for server in &config.server {
        if server.connected_drone_ids.len() < 2 {
            errors.push(format!(
                "Server {} must be connected to at least 2 drones, found {}\n",
                server.id,
                server.connected_drone_ids.len()
            ));
        }

        for drone_id in &server.connected_drone_ids {
            if client_map.contains_key(drone_id) || server_map.contains_key(drone_id) {
                errors.push(format!(
                    "Server {} must only connect to drones, found link to non-drone node {}\n",
                    server.id, drone_id
                ));
                continue;
            }

            if let Some(drone) = drone_map.get(drone_id) {
                if !drone.connected_node_ids.contains(&server.id) {
                    errors.push(format!(
                        "Drone {} does not link back to Server {}\n",
                        drone.id, server.id
                    ));
                }
            } else {
                errors.push(format!(
                    "Server {} refers to unknown Drone {}\n",
                    server.id, drone_id
                ));
            }
        }
    }

    // Check if drone graph is fully connected (servers and clients removed)
    fn dfs(start: NodeId, graph: &HashMap<NodeId, Vec<NodeId>>, visited: &mut HashSet<NodeId>) {
        visited.insert(start);
        if let Some(neighbors) = graph.get(&start) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    dfs(*neighbor, graph, visited);
                }
            }
        }
    }

    let mut drone_graph = HashMap::new();
    for drone in &config.drone {
        let neighbors: Vec<_> = drone.connected_node_ids
            .iter()
            .cloned()
            .filter(|id| drone_map.contains_key(id))
            .collect();
        drone_graph.insert(drone.id, neighbors);
    }

    if let Some(first) = config.drone.first() {
        let mut visited = HashSet::new();
        dfs(first.id, &drone_graph, &mut visited);
        let all_connected = config.drone.iter().all(|d| visited.contains(&d.id));
        if !all_connected {
            errors.push("The drone network is not fully connected (check for subnet gaps)\n".to_string());
        }
    }

    if errors.is_empty() {
        Ok(config)
    } else {
        Err(errors.join("\n"))
    }
}