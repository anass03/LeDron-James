use crossbeam_channel::{unbounded, Receiver, Sender};
use std::collections::{HashMap, HashSet};
use std::{fs, thread};
use std::fmt::Error;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;
use wg_2024::config::Config;
use wg_2024::drone::Drone;
use wg_2024::network::NodeId;
use wg_2024::packet::{Fragment, NodeType, Packet};
use LeDron_James::Drone as LeDron;
use petgraph::graph::{DiGraph, NodeIndex};
use wg_2024::controller::DroneEvent;

// IMPORTING CLIENT-SERVER
pub struct Client {
    host: Host,
    controller_recv:Receiver<ClientCommand>,
    servers: HashMap<ServerTypes, NodeId>
}
#[derive(Hash, Eq, PartialEq, Debug)]
pub enum ServerTypes{
    Chat,
    Content
}
pub type FileId = usize;
pub type MediaId = usize;
pub type ClientId = NodeId;
pub type ServerId = NodeId;
pub enum ClientCommand{
    ServerType,

    //content-server
    FilesList,
    File(FileId),

    MediaList,
    Media(MediaId),

    //chat-server
    RegistrationToChat,
    ClientList,
    MessageFor(ClientId, String),

    ControllerShortcut(Packet)
}
#[derive(Clone)]
pub struct Network {
    pub graph: DiGraph<NodeId, f64>,
    pub topology: HashMap<NodeId, (NodeIndex, Vec<NodeId>)>, // NodeId -> (Graph NodeIndex, Vec of neighbors)
    pub servers: HashSet<NodeId>,
    pub drones_packet_sent: HashMap<NodeId,(i32,i32)>
}
pub struct Host{
    pub id:NodeId,
    pub type_host:NodeType,
    pub controller_send:Sender<DroneEvent>,
    pub packet_recv:Receiver<Packet>,
    pub packet_send: HashMap<NodeId, Sender<Packet>>,
    pub flood_cache:HashMap<NodeId, u64>,
    pub network: Network,
    pub received_fragments: Arc<Mutex<HashMap<u64, RecSessionData>>>,
    pub sent_fragments: Arc<Mutex<HashMap<u64, SentSessionData>>>,
}
pub struct RecSessionData {
    pub buffer: Vec<Option<Fragment>>,
    pub last_activity: Instant, // Timestamp dell'ultima attività
}
pub struct SentSessionData {
    pub buffer: Vec<Option<Fragment>>, // Buffer dei frammenti inviati
    pub last_activity: Instant,        // Timestamp dell'ultima attività
    pub destination: NodeId,        // Destinatario della sessione
    pub route: Vec<NodeId>,        // percorso pre-calcolato
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerType {
    Chat,
    Content,
}

pub fn parse_config(file: &str) -> Result<Config,toml::de::Error> {
    let file_str = fs::read_to_string(file).unwrap();
    toml::from_str(&file_str)
}

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