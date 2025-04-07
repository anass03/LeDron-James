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
use simulation_controller::simulation_controller::{log, SimulationController};

pub fn initialize() -> (SimulationController, Vec<JoinHandle<()>>) {
    /// - Parsing the network configuration
    let config = parse_config("/Volumes/SSD FILIPPO/UNI/Advanced Programming/Drone Project/LeDron-James/src/simulation_controller/src/config");
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
/*
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
*/
    /// - handles contains all the node's threads handles
    let mut handles = Vec::new();

    /// - preparing command and event channels hashmaps
    let mut drone_command_sender = HashMap::new();
    let mut drone_event_receiver = HashMap::new();
    /*let mut client_command_sender = HashMap::new();
    let mut client_event_receiver = HashMap::new();
    let mut server_event_receiver = HashMap::new();*/
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
        println!("Drone {} -> Sender {:?}, Receiver {:?}", drone_clone.id, drone_packet_send, drone_packet_recv);

        /// - spawning the thread of the drone and collecting the handle in a vector
        handles.push(thread::spawn(move || {
            let mut drone = LeDron::new(
                drone_clone.id,
                drone_event_sender,
                controller_drone_recv,
                drone_packet_recv,
                drone_packet_send,
                drone_clone.pdr,
            );
        // /// - running the drone
            //drone.run();
        }));
    }
/*
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

        println!("Client {} -> Sender {:?}, Receiver {:?}", client_id, packet_send, packet_recv);

        /// - spawning the thread of the client and collecting the handle in a vector
        handles.push(thread::spawn(move || {
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
    let mut curr_server = 0;
    for server in config.server.iter() {

        let server_id = server.id;
        let connected_drone_ids = server.connected_drone_ids.clone();

        // creating the actual server channel for events
        let (server_event_sender, server_event_recv) = unbounded();


        // inserting channel in related hashmap
        server_event_receiver.insert(server_id, server_event_recv);

        // taking the unique server packet receiver that can't be cloned
        let packet_recv = packet_receivers.remove(&server_id).unwrap();
        // gathering all the senders for the connected nodes cloning them
        let packet_send: HashMap<_, _> = connected_drone_ids
            .iter()
            .map(|&id| (id, packet_senders[&id].clone()))
            .collect();

        println!("Server {} -> Sender {:?}, Receiver {:?}", server_id, packet_send, packet_recv);

        /// - spawning the thread of the Chat server and collecting the handle in a vector
        if curr_server == 0 {
            handles.push(thread::spawn(move || {
                let mut server = LeChatServer::new(
                    server_id,
                    NodeType::Server,
                    connected_drone_ids,
                    server_event_sender,
                    packet_recv,
                    packet_send,
                );
                /// - running the Chat server
                server.run();
            }));
            curr_server = curr_server + 1;
        } else if curr_server == 1 {
            /// - spawning the thread of the Content server and collecting the handle in a vector
            handles.push(thread::spawn(move || {
                let mut server = LeContentServer::new(
                    server_id,
                    NodeType::Server,
                    connected_drone_ids,
                    server_event_sender,
                    packet_recv,
                    packet_send,
                );
                /// - running the Content server
                server.run();
            }));
            curr_server = curr_server + 1;
        } else {
            println!("Error running servers")
        }

    }
*/
    println!("Packet senders: {:?}", packet_senders);
    println!("Packet receivers: {:?}", packet_receivers);

    /// - Connecting the Simulation Controller to all the channels and nodes
    /*let controller = SimulationController::new(
        config.clone(),
        drone_command_sender,
        drone_event_receiver,
        client_command_sender,
        client_event_receiver,
        server_event_receiver,
        packet_senders,
    );*/
    let controller = SimulationController::new(
        config.clone(),
        drone_command_sender,
        drone_event_receiver,
        //client_command_sender,
        //client_event_receiver,
        //server_event_receiver,
        packet_senders,
    );

    log("All the threads have been initialized correctly");

    (controller, handles)
}

pub fn parse_config(file: &str) -> Config {
    let file_str = fs::read_to_string(file).unwrap();
    toml::from_str(&file_str).unwrap()
}