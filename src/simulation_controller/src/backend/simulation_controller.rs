use crossbeam_channel::{Receiver, Sender};
use std::collections::HashMap;
use client::ClientCommand;
use colored::Colorize;
use wg_2024::config::Config;
use wg_2024::config::Drone as DroneConfig;
use wg_2024::config::Client as ClientConfig;
use wg_2024::config::Server as ServerConfig;
use wg_2024::controller::*;
use wg_2024::network::NodeId;
use wg_2024::packet::{Packet};
use host::{HostCommand, HostEvent};
#[derive(Clone)]
pub enum LogType{
    WARN, ERROR, INFO, DEBUG, FATAL
}
#[derive(Clone)]
pub struct Logs{
    pub logs_memory: Vec<(LogType,String)>,
    pub log_scroll_offset: usize,
}

pub struct Statistics{
    pub total_packet_exchanged: usize,
    pub total_active_connections: usize,
    pub network_reliability: u8,
}

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
    pub drone_command_sender: HashMap<NodeId, Sender<DroneCommand>>,  //command sender channel to drone  //event receiver channel from drone
    pub client_command_sender: HashMap<NodeId, Sender<ClientCommand>>,
    pub server_command_sender: HashMap<NodeId, Sender<HostCommand>>,
    pub drone_event_receiver: Receiver<DroneEvent>,
    pub host_event_receiver: Receiver<HostEvent>,
    pub packet_sender: HashMap<NodeId, Sender<Packet>>,         //packet sender of each node
    pub log_receiver: Receiver<String>,
    pub logs: Logs,
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
            client_command_sender,
            server_command_sender,
            drone_event_receiver,
            host_event_receiver,
            packet_sender,
            log_receiver,
            logs: Logs::default(),
            stats: Statistics::default(),
        }
    }
}

impl Logs{
    pub(super) fn default()->Self{
        Self{
            logs_memory: vec![],
            log_scroll_offset: 0,
        }
    }
    pub/*(crate)*/ fn push_log(&mut self, type_of_log: LogType, text: &str){
        if self.logs_memory.len()>=1000{
            self.logs_memory.remove(0);
            self.logs_memory.push((type_of_log, text.to_string()));
        }else{
            self.logs_memory.push((type_of_log, text.to_string()));
            self.log_scroll_offset += 1;
        }
    }
}
impl Statistics{
    pub fn default()->Self{
        Self{
            total_packet_exchanged: 0,
            total_active_connections: 0,
            network_reliability: 0,
        }
    }
    pub fn received_packet(&mut self){
        self.total_packet_exchanged += 1;
    }
}
pub fn log(txt: &str) {
    println!("{} - {}",Colorize::on_green("SimulationController"), Colorize::on_green(txt))
}
