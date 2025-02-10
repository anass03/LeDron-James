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
    let config = parse_config("/Volumes/SSD FILIPPO/UNI/Advanced Programming/Drone Project/LeDron-James/src/simulation_controller/src/config");

    //let mut packet_channels = HashMap::new();
    let mut packet_senders = HashMap::new();
    let mut packet_receivers = HashMap::new();
    for drone in config.drone.iter() {
        let (s, r) = unbounded();
        packet_senders.insert(drone.id, s);
        packet_receivers.insert(drone.id, r);
    }
    for client in config.client.iter() {
        let (s, r) = unbounded();
        packet_senders.insert(client.id, s);
        packet_receivers.insert(client.id, r);
    }
    for server in config.server.iter() {
        let (s, r) = unbounded();
        packet_senders.insert(server.id, s);
        packet_receivers.insert(server.id, r);
    }

    let mut handles = Vec::new();
    let mut drone_command_sender = HashMap::new();
    let mut drone_event_receiver = HashMap::new();
    let mut client_command_sender = HashMap::new();
    let mut client_event_receiver = HashMap::new();
    let mut server_event_receiver = HashMap::new();

    /// - Drones initialization
    for drone in config.drone.iter() {
        let (controller_drone_send, controller_drone_recv) = unbounded();
        let (drone_event_sender, drone_event_recv) = unbounded();

        drone_command_sender.insert(drone.id, controller_drone_send);
        drone_event_receiver.insert(drone.id, drone_event_recv);

        //let drone_packet_recv = packet_channels[&drone.id].1.clone();
        let drone_packet_recv = packet_receivers.remove(&drone.id).unwrap();
        let drone_packet_send = drone.connected_node_ids
            .iter()
            .map(|id| (*id, packet_senders[id].clone()))
            .collect();

        let drone_clone = drone.clone();
        handles.push(thread::spawn(move || {
            let mut drone = LeDron::new(
                drone_clone.id,
                drone_event_sender,
                controller_drone_recv,
                drone_packet_recv,
                drone_packet_send,
                drone_clone.pdr,
            );
            drone.run();
        }));
    }

    // Inizializzazione dei client (codice simile per client)
    /*for client in config.client.iter() {  // ✅ Use `iter()` instead of `into_iter()`
        let client_id = client.id;  // ✅ Copy primitive value
        let connected_drone_ids = client.connected_drone_ids.clone(); // ✅ Clone so it can be moved later

        // Client command channel
        let (controller_client_send, controller_client_recv) = unbounded();
        let (client_event_sender, client_event_recv) = unbounded();

        // Insert into HashMaps
        client_command_sender.insert(client_id, controller_client_send);
        client_event_receiver.insert(client_id, client_event_recv);

        // Packet channels
        let packet_recv = packet_receivers.remove(&client_id).unwrap(); // ✅ Clone before move
        let packet_send: HashMap<_, _> = connected_drone_ids
            .iter()
            .map(|&id| (id, packet_senders[&id].clone())) // ✅ Clone values, avoid move
            .collect();

        // Spawn the thread with only the necessary cloned values
        handles.push(thread::spawn(move || {
            let mut client = LeClient::new(
                client_id,
                NodeType::Client,
                connected_drone_ids, // ✅ Uses cloned values
                client_event_sender,
                controller_client_recv,
                packet_recv,
                packet_send,
            );
            //client.run();
        }));
    }*/

    /*let mut curr_server = 0;
    // Inizializzazione dei server (codice simile per server)
    for server in config.server.iter() {  // ✅ Use `iter()` instead of `into_iter()`

        let server_id = server.id;  // ✅ Copy primitive value
        let connected_drone_ids = server.connected_drone_ids.clone(); // ✅ Clone so it can be moved later
        //let server_clone = server.clone(); // ✅ Clone full `client` if needed

        // Client event channel
        let (server_event_sender, server_event_recv) = unbounded();


        // Insert into HashMaps
        server_event_receiver.insert(server_id, server_event_recv);

        // Packet channels
        let packet_recv = packet_receivers.remove(&server_id).unwrap();
        let packet_send: HashMap<_, _> = connected_drone_ids
            .iter()
            .map(|&id| (id, packet_senders[&id].clone()))
            .collect();

        // Spawn the thread with only the necessary cloned values
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
                //server.run();
            }));
            curr_server = curr_server + 1;
        } else if curr_server == 1 {
            handles.push(thread::spawn(move || {
                let mut server = LeContentServer::new(
                    server_id,
                    NodeType::Server,
                    connected_drone_ids, // ✅ Uses cloned values
                    server_event_sender,
                    packet_recv,
                    packet_send,
                );
                //server.run();
            }));
            curr_server = curr_server + 1;
        } else {
            println!("Error running servers")
        }

    }*/
    // Preparazione packet_channels per il controller
    /*let mut controller_packet_senders: HashMap<NodeId, Sender<Packet>> = HashMap::new();
    for (node_id, sender) in packet_senders.iter() {
        controller_packet_senders.insert(*node_id, sender.clone());
    }*/

    // Creiamo il controller con canali vuoti
    let controller = SimulationController::new(
        config.clone(),
        drone_command_sender,
        drone_event_receiver,
        client_command_sender,
        client_event_receiver,
        server_event_receiver,
        packet_senders,
    );

    log("All the threads have been initialized correctly");

    (controller, handles)
}

pub fn parse_config(file: &str) -> Config {
    let file_str = fs::read_to_string(file).unwrap();
    toml::from_str(&file_str).unwrap()
}