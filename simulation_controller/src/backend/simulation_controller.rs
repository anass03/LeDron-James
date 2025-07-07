use crossbeam_channel::{unbounded, Receiver, Sender};
use std::collections::HashMap;
use std::fmt::Display;
use client::ClientCommand;
use std::collections::HashSet;
use std::path::PathBuf;
use wg_2024::config::Config;
use wg_2024::config::Drone as DroneConfig;
use wg_2024::config::Client as ClientConfig;
use wg_2024::config::Server as ServerConfig;
use wg_2024::controller::*;
use wg_2024::network::NodeId;
use wg_2024::packet::Packet;
use host::{HostCommand, HostEvent};
use log::LevelFilter;
use tui_logger::TuiWidgetState;
use crate::backend::sc_models::{ChatCache, DroneType, NetCache, NetNode, RendererCommand, RendererResponse, Statistics};
use crate::frontend::app::topology_algorithm::alg_models::TopologyAlgorithm;

/// Simulation Controller struct contains:
/// - the drones
/// - the clients
/// - the servers
/// - the command sender channels
/// - the event receiver channels

pub struct SimulationControllerData {
    // Each of the next data is going to get re-initialized each time the simulation stops!

    pub drones: Vec<DroneConfig>,   //vector of drones
    pub clients: Vec<ClientConfig>, //vector of clients
    pub servers: Vec<ServerConfig>, //vector of servers
    pub drone_types: HashMap<NodeId, DroneType>,
    pub running_network: HashMap<NetNode, Vec<NodeId>>, // Value is all the links the given NodeID has! Has to be updated by Initializer / SC Backend
    pub drone_packet_count: HashMap<NodeId, u64>,
    pub drone_command_sender: HashMap<NodeId, Sender<DroneCommand>>,  //command sender channel to drone  //event receiver channel from drone
    pub client_command_sender: HashMap<NodeId, Sender<ClientCommand>>,
    pub server_command_sender: HashMap<NodeId, Sender<HostCommand>>,
    pub drone_event_receiver: Receiver<DroneEvent>,
    pub host_event_receiver: Receiver<HostEvent>,
    pub packet_sender: HashMap<NodeId, Sender<Packet>>,         //packet sender of each node
    pub topology_algorithm_data: TopologyAlgorithm,
    pub stats: Statistics,
    pub download_folder: PathBuf,

    pub host_log_state: HashMap<NodeId, (String, TuiWidgetState)>,

    pub chat_cache: ChatCache,
    pub net_cache: NetCache,

    pub tx_to_renderer: Option<std::sync::mpsc::Sender<RendererCommand>>,

    pub rx_from_renderer: Option<std::sync::mpsc::Receiver<RendererResponse>>,
}

impl SimulationControllerData {
    pub fn new(
        config: Config,
        drone_types: HashMap<NodeId, DroneType>,
        drone_command_sender: HashMap<NodeId, Sender<DroneCommand>>,
        client_command_sender: HashMap<NodeId, Sender<ClientCommand>>,
        server_command_sender: HashMap<NodeId, Sender<HostCommand>>,
        drone_event_receiver: Receiver<DroneEvent>,
        host_event_receiver: Receiver<HostEvent>,
        packet_sender: HashMap<NodeId, Sender<Packet>>,
    ) -> Self
    {
        let mut running_network:HashMap<NetNode, Vec<NodeId>> = HashMap::new();
        let mut host_log_state: HashMap<NodeId, (String, TuiWidgetState)> = HashMap::new();
        let mut total_active_connections = 0;

        let mut drones = vec![];
        for drone in config.drone {
            let connected_ids = drone.connected_node_ids.clone();
            running_network.insert(NetNode::Drone(drone.id), connected_ids);
            drones.push(drone);
            total_active_connections += 1;
        }

        let mut clients = vec![];
        for client in config.client {
            let client_id = client.id.clone();
            let connected_ids = client.connected_drone_ids.clone();
            running_network.insert(NetNode::Client(client_id), connected_ids);
            clients.push(client);
            host_log_state.insert(client_id, (format!("{client_id}_Client"), TuiWidgetState::default().set_default_display_level(LevelFilter::Off).set_level_for_target(format!("{client_id}_Client").as_str(), LevelFilter::max())));
            total_active_connections += 1;
        }

        let mut servers = vec![];
        for server in config.server {
            let server_id = server.id.clone();
            let connected_ids = server.connected_drone_ids.clone();
            running_network.insert(NetNode::Server(server_id), connected_ids);
            servers.push(server);
            host_log_state.insert(server_id, (format!("{server_id}_Server"), TuiWidgetState::default().set_default_display_level(LevelFilter::Off).set_level_for_target(format!("{server_id}_Server").as_str(), LevelFilter::max())));
            total_active_connections += 1;
        }

        // Collect unique, undirected links
        let mut links = HashSet::new();
        for (net_node, neighbors) in &running_network {
            // Extract the NodeId from NetNode
            let id = *net_node.id();
            for &neighbor in neighbors {
                // Order the pair to avoid duplicates (min, max)
                let (a, b) = if id <= neighbor { (id, neighbor) } else { (neighbor, id) };
                links.insert((a, b));
            }
        }
        let mut stats = Statistics::default();
        stats.total_active_connections = total_active_connections;
        Self {
            drones,
            clients,
            servers,
            drone_types,
            drone_command_sender,
            client_command_sender,
            server_command_sender,
            drone_event_receiver,
            host_event_receiver,
            packet_sender,
            running_network,
            stats,
            download_folder: PathBuf::from("assets"),
            topology_algorithm_data: TopologyAlgorithm::new(links.into_iter().collect()),

            host_log_state,

            chat_cache: ChatCache::default(),
            net_cache: NetCache::default(),

            tx_to_renderer: None,
            rx_from_renderer: None,
            drone_packet_count: Default::default(),
        }
    }
}

impl Default for SimulationControllerData {
    fn default() -> Self {
        Self {
            drones: vec![],
            clients: vec![],
            servers: vec![],
            drone_types: Default::default(),
            running_network: HashMap::new(),
            drone_packet_count: Default::default(),
            drone_command_sender: Default::default(),
            client_command_sender: Default::default(),
            server_command_sender: Default::default(),
            drone_event_receiver: unbounded().1,
            host_event_receiver: unbounded().1,
            packet_sender: Default::default(),
            stats: Statistics::default(),
            topology_algorithm_data: TopologyAlgorithm::default(),
            chat_cache: ChatCache::default(),
            net_cache: NetCache::default(),
            download_folder: PathBuf::from("assets"),
            tx_to_renderer: None,
            rx_from_renderer: None,
            host_log_state: Default::default(),
        }
    }
}

