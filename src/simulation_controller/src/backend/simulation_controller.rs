use crossbeam_channel::{Receiver, Sender};
use std::collections::HashMap;
use std::thread::JoinHandle;
use colored::Colorize;
use wg_2024::config::Config;
use wg_2024::config::Drone as DroneConfig;
use wg_2024::config::Client as ClientConfig;
use wg_2024::config::Server as ServerConfig;
use wg_2024::controller::*;
use wg_2024::network::NodeId;
use wg_2024::packet::{Packet};
use host::{HostEvent, NetCommand};

pub enum LogType{
    WARN, ERROR, INFO, DEBUG, FATAL
}
pub struct Logs{
    pub logs_memory: Vec<(LogType,String)>,
    pub log_scroll_offset: usize,
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
    pub client_command_sender: HashMap<NodeId, Sender<NetCommand>>,
    pub server_command_sender: HashMap<NodeId, Sender<NetCommand>>,
    pub drone_event_receiver: Receiver<DroneEvent>,
    pub host_event_receiver: Receiver<HostEvent>,
    pub packet_sender: HashMap<NodeId, Sender<Packet>>,         //packet sender of each node
    pub logs: Logs,
}

impl SimulationControllerData {
    pub fn new(
        config: Config,
        drone_command_sender: HashMap<NodeId, Sender<DroneCommand>>,
        client_command_sender: HashMap<NodeId, Sender<NetCommand>>,
        server_command_sender: HashMap<NodeId, Sender<NetCommand>>,
        drone_event_receiver: Receiver<DroneEvent>,
        host_event_receiver: Receiver<HostEvent>,
        packet_sender: HashMap<NodeId, Sender<Packet>>,
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
            logs: Logs::default(),
        }
    }
}

impl Logs{
    pub(super) fn default()->Self{
        Self{
            logs_memory: vec![
                (LogType::INFO,  "[2025-04-18 12:00:01] Server started on port 8080".into()),
                (LogType::DEBUG, "[2025-04-18 12:00:02] Connected to database successfully".into()),
                (LogType::INFO,  "[2025-04-18 12:00:05] New client connected: 192.168.1.24".into()),
                (LogType::WARN,  "[2025-04-18 12:00:08] High memory usage detected".into()),
                (LogType::INFO,  "[2025-04-18 12:00:12] Job queue processed: 17 tasks in 432ms".into()),
                (LogType::ERROR, "[2025-04-18 12:00:15] Failed to open config file: config.toml".into()),
                (LogType::DEBUG, "[2025-04-18 12:00:18] Retrying connection to cache service...".into()),
                (LogType::INFO,  "[2025-04-18 12:00:20] Background sync completed".into()),
                (LogType::WARN,  "[2025-04-18 12:00:23] Connection to backup server timed out".into()),
                (LogType::INFO,  "[2025-04-18 12:00:25] Metrics updated (CPU: 84%, RAM: 72%)".into()),
                (LogType::DEBUG, "[2025-04-18 12:00:28] Heartbeat ping sent to 5 clients".into()),
                (LogType::ERROR, "[2025-04-18 12:00:30] Unexpected token in lin from 10.0.0.12".into()),
                (LogType::DEBUG, "[2025-04-18 12:00:34] Polling external services (3 targets)".into()),
                (LogType::INFO,  "[2025-04-18 12:00:36] Graceful shutdown initiated".into()),
            ],
            log_scroll_offset: 0,
        }
    }
    pub(crate) fn push_log(&mut self, type_of_log: LogType, text: &str){
        if self.logs_memory.len()>=50{
            self.logs_memory.remove(0);
            self.logs_memory.push((type_of_log, text.to_string()));
        }else{
            self.logs_memory.push((type_of_log, text.to_string()));
            self.log_scroll_offset += 1;
        }
    }
}
pub fn log(txt: &str) {
    println!("{} - {}",Colorize::on_green("SimulationController"), Colorize::on_green(txt))
}
