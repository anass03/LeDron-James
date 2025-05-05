use crossbeam_channel::{unbounded, Sender};
use std::collections::HashMap;
use std::{fs, thread};
use std::thread::JoinHandle;
use wg_2024::config::Config;
use wg_2024::drone::Drone;
use wg_2024::network::NodeId;
use wg_2024::packet::{NodeType, Packet};
use drone::Drone as LeDron;
use client::Client as LeClient;
use server::ContentServer as LeContentServer;
use server::chatserver::ChatServer as LeChatServer;
use simulation_controller::backend::simulation_controller::{log, SimulationController};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerType {
    Chat,
    Content,
}

pub fn initialize() -> (SimulationController, HashMap<NodeId, JoinHandle<()>>) {
    /// - Parsing the network configuration
    let config = parse_config("/Volumes/SSD FILIPPO/UNI/Advanced Programming/Drone Project/LeDron-James/src/simulation_controller/src/config");
    // config contains {
    //  vector of drones
    //  vector of clients
    //  vector of servers
    // }
    let _config_check = check_config(&config);

    /// - Creating and assigning the packet channels to each node
    let mut packet_senders = HashMap::new();
    let mut packet_receivers = HashMap::new();
    // packet channels are meant to exchange packets, not commands nor events

    for drone in config.drone.iter() {
        // for each drone in the config we create the packet sender and receiver then we insert them
        // in the related hashmap
        let (s, r) = unbounded();
        packet_senders.insert(drone.id, s);
        packet_receivers.insert(drone.id, r);
    }

    for client in config.client.iter() {
        // for each client in the config we create the packet sender and receiver and we insert them
        // in the related hashmap
        let (s, r) = unbounded();
        packet_senders.insert(client.id, s);
        packet_receivers.insert(client.id, r);
    }
    for server in config.server.iter() {
        // for each server in the config we create the packet sender and receiver and we insert them
        // in the related hashmap
        let (s, r) = unbounded();
        packet_senders.insert(server.id, s);
        packet_receivers.insert(server.id, r);
    }

    /// - handles contains all the node's threads handles with NodeId as key
    let mut handles = HashMap::new();

    /// - preparing command and event channels hashmaps
    let mut drone_command_sender = HashMap::new();
    let mut drone_event_receiver = HashMap::new();
    let mut client_command_sender = HashMap::new();
    let mut client_event_receiver = HashMap::new();
    let mut server_event_receiver = HashMap::new();
    // these hashmaps will contain the senders and receivers paired with each NodeId

    /// - Initializing the drones
    for drone in config.drone.iter() {
        // creating the actual drone channels for commands and events
        let (controller_drone_send, controller_drone_recv) = unbounded();
        let (drone_event_sender, drone_event_recv) = unbounded();

        // inserting channels in related hashmaps
        drone_command_sender.insert(drone.id, controller_drone_send);
        drone_event_receiver.insert(drone.id, drone_event_recv);
        // taking the unique drone packet receiver that can't be cloned
        let drone_packet_recv = packet_receivers.remove(&drone.id).unwrap();
        // gathering all the senders for the connected nodes cloning them
        let drone_packet_send: HashMap<NodeId, Sender<Packet>> = drone.connected_node_ids
            .iter()
            .filter_map(|id| packet_senders.get(id).cloned().map(|sender| (*id, sender)))
            .collect();


        let drone_clone = drone.clone();

        /// - spawning the thread of the drone and collecting the handle in a vector
        handles.insert(drone.id, thread::spawn(move || {
            let mut drone = LeDron::new(
                drone_clone.id,
                drone_event_sender,
                controller_drone_recv,
                drone_packet_recv,
                drone_packet_send,
                drone_clone.pdr,
            );
            /// - running the drone
            drone.run();
        }));
    }

    /// - Initializing the clients
    for client in config.client.iter() {
        let client_id = client.id;
        let connected_drone_ids = client.connected_drone_ids.clone();

        // creating the actual client channels for commands and events
        let (controller_client_send, controller_client_recv) = unbounded();
        let (client_event_sender, client_event_recv) = unbounded();

        // inserting channels in related hashmaps
        client_command_sender.insert(client_id, controller_client_send);
        client_event_receiver.insert(client_id, client_event_recv);

        // taking the unique client packet receiver that can't be cloned
        let packet_recv = packet_receivers.remove(&client_id).unwrap();
        // gathering all the senders for the connected nodes cloning them
        let packet_send: HashMap<_, _> = connected_drone_ids
            .iter()
            .map(|&id| (id, packet_senders[&id].clone()))
            .collect();

        /// - spawning the thread of the client and collecting the handle in a vector
        handles.insert(client.id, thread::spawn(move || {
            let mut client = LeClient::new(
                client_id,
                NodeType::Client,
                connected_drone_ids,
                client_event_sender,
                controller_client_recv,
                packet_recv,
                packet_send,
            );
            /// - running the client
            client.run();
        }));
    }

    /// - Initializing the servers
    for (i, server) in config.server.iter().enumerate() {
        let server_type = match i {
            0 => ServerType::Chat,
            1 => ServerType::Content,
            _ => {
                println!("Too many servers ({}). Ignoring extra server with id {}", i, server.id);
                continue;
            }
        };

        let server_id = server.id;
        let connected_drone_ids = server.connected_drone_ids.clone();

        let (server_event_sender, server_event_recv) = unbounded();
        server_event_receiver.insert(server_id, server_event_recv);

        let packet_recv = packet_receivers.remove(&server_id).unwrap();
        let packet_send: HashMap<_, _> = connected_drone_ids
            .iter()
            .map(|&id| (id, packet_senders[&id].clone()))
            .collect();

        let handle = match server_type {
            ServerType::Chat => thread::spawn(move || {
                let mut server = LeChatServer::new(
                    server_id,
                    NodeType::Server,
                    connected_drone_ids,
                    server_event_sender,
                    packet_recv,
                    packet_send,
                );
                server.run();
            }),
            ServerType::Content => thread::spawn(move || {
                let mut server = LeContentServer::new(
                    server_id,
                    NodeType::Server,
                    connected_drone_ids,
                    server_event_sender,
                    packet_recv,
                    packet_send,
                );
                server.run();
            }),
        };

        handles.insert(server_id, handle);
    }

    /// - Connecting the Simulation Controller to all the channels and nodes
    let controller = SimulationController::new(
        config.clone(),
        drone_command_sender,
        drone_event_receiver,
        client_command_sender,
        client_event_receiver,
        server_event_receiver,
        packet_senders,
    );

    log("Exiting initialization");

    (controller, handles)
}

pub fn parse_config(file: &str) -> Config {
    let file_str = fs::read_to_string(file).unwrap();
    toml::from_str(&file_str).unwrap()
}

// err pdr > 1
pub fn check_config(config: &Config) -> Result<(), String> {
    use std::collections::HashSet;

    let drone_map: HashMap<_, _> = config.drone.iter().map(|d| (d.id, d)).collect();
    let client_map: HashMap<_, _> = config.client.iter().map(|c| (c.id, c)).collect();
    let server_map: HashMap<_, _> = config.server.iter().map(|s| (s.id, s)).collect();

    let mut errors = Vec::new();

    // === Unique ID check across all node types ===
    let mut id_set = HashSet::new();
    for drone in &config.drone {
        if !id_set.insert(drone.id) {
            errors.push(format!("Duplicate NodeId found: {}", drone.id));
        }
    }
    for client in &config.client {
        if !id_set.insert(client.id) {
            errors.push(format!("Duplicate NodeId found: {}", client.id));
        }
    }
    for server in &config.server {
        if !id_set.insert(server.id) {
            errors.push(format!("Duplicate NodeId found: {}", server.id));
        }
    }

    // === Existing check: drone -> node ===
    for drone in &config.drone {
        for node_id in &drone.connected_node_ids {
            if let Some(client) = client_map.get(node_id) {
                if !client.connected_drone_ids.contains(&drone.id) {
                    errors.push(format!("Client {} does not link back to Drone {}", client.id, drone.id));
                }
            } else if let Some(server) = server_map.get(node_id) {
                if !server.connected_drone_ids.contains(&drone.id) {
                    errors.push(format!("Server {} does not link back to Drone {}", server.id, drone.id));
                }
            } else if let Some(conn_drone) = drone_map.get(node_id) {
                if !conn_drone.connected_node_ids.contains(&drone.id) {
                    errors.push(format!("Drone {} does not link back to Drone {}", conn_drone.id, drone.id));
                }
            } else {
                errors.push(format!("Node {} connected to Drone {} does not exist", node_id, drone.id));
            }
        }
    }

    // === Existing check: client -> drone ===
    for client in &config.client {
        for drone_id in &client.connected_drone_ids {
            if let Some(drone) = drone_map.get(drone_id) {
                if !drone.connected_node_ids.contains(&client.id) {
                    errors.push(format!("Drone {} does not link back to Client {}", drone.id, client.id));
                }
            } else {
                errors.push(format!("Client {} refers to unknown Drone {}", client.id, drone_id));
            }
        }
    }

    // === Existing check: server -> drone ===
    for server in &config.server {
        for drone_id in &server.connected_drone_ids {
            if let Some(drone) = drone_map.get(drone_id) {
                if !drone.connected_node_ids.contains(&server.id) {
                    errors.push(format!("Drone {} does not link back to Server {}", drone.id, server.id));
                }
            } else {
                errors.push(format!("Server {} refers to unknown Drone {}", server.id, drone_id));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}