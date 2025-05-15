use crossbeam_channel::{Receiver, Sender};
use std::collections::HashMap;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use colored::Colorize;
use wg_2024::config::Config;
use wg_2024::config::Drone as DroneConfig;
use wg_2024::config::Client as ClientConfig;
use wg_2024::config::Server as ServerConfig;
use wg_2024::controller::*;
use wg_2024::network::NodeId;
use wg_2024::packet::{Packet};
use host::{ClientCommand, HostEvent, NetCommand};

/// Simulation Controller struct contains:
/// - the drones
/// - the clients
/// - the servers
/// - the command sender channels
/// - the event receiver channels

pub struct SimulationControllerData {
    pub drones: Vec<DroneConfig>,   //vector of drones
    pub clients: Vec<ClientConfig>, //vector of clients
    pub servers: Vec<ServerConfig>, //vector of servers
    pub drone_command_sender: HashMap<NodeId, Sender<DroneCommand>>,  //command sender channel to drone
    pub drone_event_receiver: HashMap<NodeId, Receiver<DroneEvent>>,   //event receiver channel from drone
    pub client_command_sender: HashMap<NodeId, Sender<NetCommand>>,
    pub client_event_receiver: HashMap<NodeId, Receiver<HostEvent>>,
    pub server_event_receiver: HashMap<NodeId, Receiver<DroneEvent>>,
    pub packet_sender: HashMap<NodeId, Sender<Packet>>,         //packet sender of each node
}

impl SimulationControllerData {
    pub fn new(
        config: Config,
        drone_command_sender: HashMap<NodeId, Sender<DroneCommand>>,
        drone_event_receiver: HashMap<NodeId, Receiver<DroneEvent>>,
        client_command_sender: HashMap<NodeId, Sender<NetCommand>>,
        client_event_receiver: HashMap<NodeId, Receiver<HostEvent>>,
        server_event_receiver: HashMap<NodeId, Receiver<DroneEvent>>,
        packet_sender: HashMap<NodeId, Sender<Packet>>,
        //packet_receiver: HashMap<NodeId, Receiver<Packet>>,
    ) -> Self {
        let mut drones = vec![];
        for drone in config.drone {
            drones.push(drone);
        }
        let mut clients = vec![];
        for client in config.client {
            clients.push(client);
        }
        let mut servers = vec![];
        for server in config.server {
            servers.push(server);
        }
        Self {
            drones,
            clients,
            servers,
            drone_command_sender,
            drone_event_receiver,
            client_command_sender,
            client_event_receiver,
            server_event_receiver,
            packet_sender,
            //packet_receiver,
        }
    }
}

pub fn simulate(mut controller: SimulationControllerData, mut handles: HashMap<NodeId, JoinHandle<()>>) {

    log("Starting simulation controller");


    for (node_id, handle) in handles {
        handle.join().unwrap();
        log(&format!("Closing thread of node: {:?}", node_id));
    }
    log("Closing simulation controller");
}

pub fn log(txt: &str) {
    println!("{} - {}",Colorize::on_green("SimulationController"), colored::Colorize::on_green(txt))
}
