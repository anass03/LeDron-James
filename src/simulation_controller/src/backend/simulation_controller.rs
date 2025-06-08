use crossbeam_channel::{Receiver, Sender};
use std::collections::HashMap;
use client::ClientCommand;
use wg_2024::config::Config;
use wg_2024::config::Drone as DroneConfig;
use wg_2024::config::Client as ClientConfig;
use wg_2024::config::Server as ServerConfig;
use wg_2024::controller::*;
use wg_2024::network::NodeId;
use wg_2024::packet::{Packet};
use host::{HostCommand, HostEvent};
use crate::backend::sc_models::{Logs, Statistics};
use crate::frontend::app::topology_algorithm::alg_models::TopologyAlgorithm;

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub enum NetNode { Drone(NodeId), Client(NodeId), Server(NodeId) }

/// Simulation Controller struct contains:
/// - the drones
/// - the clients
/// - the servers
/// - the command sender channels
/// - the event receiver channels

pub struct SimulationControllerData {
    pub logs: Logs,

    // Each of the next data is going to get re-initialized each time the simulation stops!

    pub drones: Vec<DroneConfig>,   //vector of drones
    pub clients: Vec<ClientConfig>, //vector of clients
    pub servers: Vec<ServerConfig>, //vector of servers
    pub running_network: HashMap<NetNode, Vec<NodeId>>, // Value is all the links the given NodeID has!
    pub drone_command_sender: HashMap<NodeId, Sender<DroneCommand>>,  //command sender channel to drone  //event receiver channel from drone
    pub client_command_sender: HashMap<NodeId, Sender<ClientCommand>>,
    pub server_command_sender: HashMap<NodeId, Sender<HostCommand>>,
    pub drone_event_receiver: Receiver<DroneEvent>,
    pub host_event_receiver: Receiver<HostEvent>,
    pub packet_sender: HashMap<NodeId, Sender<Packet>>,         //packet sender of each node
    pub log_receiver: Receiver<String>,
    pub topology_algorithm_data: TopologyAlgorithm,
    pub stats: Statistics,
}

impl SimulationControllerData {
    pub fn new(
        config: Config,
        drone_command_sender: HashMap<NodeId, Sender<DroneCommand>>,
        client_command_sender: HashMap<NodeId, Sender<ClientCommand>>,
        server_command_sender: HashMap<NodeId, Sender<HostCommand>>,
        drone_event_receiver: Receiver<DroneEvent>,
        host_event_receiver: Receiver<HostEvent>,
        packet_sender: HashMap<NodeId, Sender<Packet>>,
        log_receiver: Receiver<String>,
    ) -> Self {
        let mut running_network:HashMap<NetNode, Vec<NodeId>> = HashMap::new();

        let mut drones = vec![];
        for drone in config.drone {
            let connected_ids = drone.connected_node_ids.clone();
            running_network.insert(NetNode::Drone(drone.id), connected_ids);
            drones.push(drone);
        }
        let mut clients = vec![];
        for client in config.client {
            let connected_ids = client.connected_drone_ids.clone();
            running_network.insert(NetNode::Client(client.id), connected_ids);
            clients.push(client);
        }
        let mut servers = vec![];
        for server in config.server {
            let connected_ids = server.connected_drone_ids.clone();
            running_network.insert(NetNode::Client(server.id), connected_ids);
            servers.push(server);
        }
        Self {
            drones,
            clients,
            servers,
            drone_command_sender,
            client_command_sender,
            server_command_sender,
            drone_event_receiver,
            host_event_receiver,
            packet_sender,
            log_receiver,
            running_network,
            logs: Logs::default(),
            stats: Statistics::default(),
            topology_algorithm_data: TopologyAlgorithm::default(),
        }
    }
}
