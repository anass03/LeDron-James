use crossbeam_channel::{unbounded, Sender};
use std::collections::HashMap;
use std::thread;
use std::thread::JoinHandle;
use wg_2024::drone::Drone;
use wg_2024::network::NodeId;
use wg_2024::packet::{NodeType, Packet};
use drone::Drone as LeDron;
use client::Client as LeClient;
use host::ServerTypes;
use server::{ContentServer as LeContentServer, GenServer};
use server::ChatServer as LeChatServer;
use simulation_controller::backend::simulation_controller::{log, SimulationControllerData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerType {
    Chat,
    Content,
}

pub fn initialize(path: &str) -> Result<(SimulationControllerData, HashMap<NodeId, JoinHandle<()>>), String> {
    /// - Parsing the network configuration
    let config;
    match initializer_utils::utils::parse_and_check_config(&path.to_string()){
        Ok(temp) => {
            config = temp;
        },
        Err(e) => {
            // Possibilità di recupero generando una nuova toml oppure inizializzando solo
            // il simulation_controller che poi si preouccuperà di rifar
            // partire la simulazione
            return Err(e.to_string());
        }
    }

    // config contains {
    //  vector of drones
    //  vector of clients
    //  vector of servers
    // }

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

    /// - preparing command channels hashmaps
    let mut drone_command_senders = HashMap::new();
    //let mut drone_event_receivers = HashMap::new();
    let mut client_command_senders = HashMap::new();
    //let mut client_event_receivers = HashMap::new();
    let mut server_command_senders = HashMap::new();
    //let mut server_event_receivers = HashMap::new();
    // these hashmaps will contain the senders and receivers paired with each NodeId

    /// - Initializing the drones
    //simulation controller has a single receiver for all drone events, while the sender is cloned
    //among all drones
    let (drone_event_send, drone_event_recv) = unbounded();
    for drone in config.drone.iter() {

        // creating the actual drone channel for commands
        let (controller_drone_send, controller_drone_recv) = unbounded();
        //cloning the shared sender
        let drone_event_send = drone_event_send.clone();

        // inserting channels in related hashmaps
        drone_command_senders.insert(drone.id, controller_drone_send);
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
                drone_event_send,
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
    let (host_event_send, host_event_recv) = unbounded();
    let (log_sender, log_receiver) = unbounded();
    for client in config.client.iter() {
        let client_id = client.id;
        let connected_drone_ids = client.connected_drone_ids.clone();

        // creating the actual client channels for commands and events
        let (controller_client_send, controller_client_recv) = unbounded();
        let host_event_send = host_event_send.clone();

        // inserting channels in related hashmaps
        client_command_senders.insert(client_id, controller_client_send);

        // taking the unique client packet receiver that can't be cloned
        let packet_recv = packet_receivers.remove(&client_id).unwrap();
        // gathering all the senders for the connected nodes cloning them
        let packet_send: HashMap<_, _> = connected_drone_ids
            .iter()
            .map(|&id| (id, packet_senders[&id].clone()))
            .collect();
        let log_send = log_sender.clone();
        /// - spawning the thread of the client and collecting the handle in a vector
        handles.insert(client.id, thread::spawn(move || {
            let mut client = LeClient::new(
                client_id,
                NodeType::Client,
                connected_drone_ids,
                host_event_send,
                controller_client_recv,
                packet_recv,
                packet_send,
                log_send
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

        let (controller_server_send, controller_server_recv) = unbounded();
        let host_event_send = host_event_send.clone();

        server_command_senders.insert(server_id, controller_server_send);

        let packet_recv = packet_receivers.remove(&server_id).unwrap();
        let packet_send: HashMap<_, _> = connected_drone_ids
            .iter()
            .map(|&id| (id, packet_senders[&id].clone()))
            .collect();

        let log_send = log_sender.clone();
        let handle = match server_type {
            ServerType::Chat => thread::spawn(move || {
                let mut server = LeChatServer::new(
                    server_id,
                    NodeType::Server,
                    connected_drone_ids,
                    host_event_send,
                    controller_server_recv,
                    packet_recv,
                    packet_send,
                    log_send
                );
                server.run();
            }),
            ServerType::Content => thread::spawn(move || {
                let mut server = LeContentServer::new(
                    GenServer::new(
                    server_id,
                    NodeType::Server,
                    connected_drone_ids,
                    host_event_send,
                    controller_server_recv,
                    packet_recv,
                    packet_send,
                    ServerTypes::Content,
                    log_send
                    )
                );
                server.run();
            }),
        };

        handles.insert(server_id, handle);
    }

    /// - Connecting the Simulation Controller to all the channels and nodes
    let controller = SimulationControllerData::new(
        config.clone(),
        drone_command_senders,
        client_command_senders,
        server_command_senders,
        drone_event_recv,
        host_event_recv,
        packet_senders,
        log_receiver
    );

    Ok((controller, handles))
}