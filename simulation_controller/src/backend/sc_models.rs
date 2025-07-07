use std::collections::{HashMap, HashSet};
use host::{ClientId, FileId, HostCommand, Image, MediaId, ServerId};
use std::fmt::{Debug, Display, Formatter};
use std::sync::mpsc::{Receiver, Sender};          // <-- concrete type
use server::chat::{Chat, MessageSC::*};
use ratatui_image::thread::{ResizeRequest, ResizeResponse, ThreadProtocol};
use std::io::Write;
use std::path::PathBuf;
use dashmap::DashMap;
use wg_2024::controller::DroneCommand;
use wg_2024::network::NodeId;
use client::ClientCommand;
use crate::backend::simulation_controller::SimulationControllerData;
use crate::frontend::app::topology_algorithm::alg_models::TopologyAlgorithm;
use std::sync::{Arc, RwLock};


pub(crate) trait NodeCommand {}
impl NodeCommand for DroneCommand {}
impl NodeCommand for HostCommand {}
impl NodeCommand for ClientCommand {}

/// Used in multiple crates, needs to be pub
#[derive(Clone, Debug)]
pub enum LogType{
    WARN, ERROR, INFO, DEBUG
}


pub struct Statistics{
    pub total_packet_exchanged: usize,
    pub total_packet_dropped: usize,
    pub total_active_connections: usize,
    pub network_reliability: u8,
    pub node_statistic: HashMap<NodeId, NodeStatistics>,
}
#[derive(Default)]
pub struct NodeStatistics{
    pub pdr_expected: HashMap<NodeId, f64>, // Only for clients
    pub download_at_sec: Vec<f32>, // Only for clients
    pub upload_at_sec: Vec<f32>, // Only for clients
    pub route: Vec<NodeId>
    //
}

// Implementations

impl Statistics{
    pub fn default()->Self{
        Self{
            total_packet_exchanged: 0,
            total_packet_dropped: 0,
            total_active_connections: 0,
            network_reliability: 0,
            node_statistic: Default::default(),
        }
    }
    pub fn received_packet(&mut self) { self.total_packet_exchanged += 1; }
    pub fn dropped_packet(&mut self) { self.total_packet_dropped += 1; }
}

pub enum RendererCommand{
    Stop,
    EncodeMedia((MediaId, Image)),
    EncodeFile((FileId, Vec<u8>)),
    ComputeSetOfTopologies((Vec<(NodeId,TopologyAlgorithm)>)),
    ChangeDownloadFolder(PathBuf),
    SaveMediaFromChatList(Arc<RwLock<Chat>>),
}
pub enum RendererResponse{
    EncodedMedia((MediaId, NetCacheMedia, Option<Receiver<FileRendererUpdate>>)),
    EncodedFile((FileId, NetCacheFile)),
    EncodedMediaChats(Vec<(MediaId,PathBuf)>),
    Computed(Vec<(NodeId,TopologyAlgorithm)>)
}

#[derive(Debug)]
pub struct MediaEntry {
    pub rx: Option<Receiver<FileRendererUpdate>>,
    pub cache: NetCacheMedia,
}

impl MediaEntry {
    pub fn new(cache: NetCacheMedia) -> Self {
        Self {
            rx: None,
            cache,
        }
    }

    pub fn with_channels(_tx: Sender<ResizeRequest>, rx: Receiver<FileRendererUpdate>, cache: NetCacheMedia) -> Self {
        Self {
            rx: Some(rx),
            cache,
        }
    }

    pub fn update_cache_media(&mut self, new_cache: NetCacheMedia) {
        self.cache = new_cache;
    }

}

pub struct NetCache{
    // Simply all the chat server the Client knows.
    pub known_content_servers: HashMap<ClientId, Vec<ServerId>>,
    
    pub known_file_list: HashMap<ClientId, HashMap<ServerId, Vec<FileId>>>,
    pub known_media_list: HashMap<ClientId, HashMap<ServerId, Vec<MediaId>>>,

    pub downloaded_medias: DashMap<MediaId,MediaEntry>,
    pub downloaded_files: DashMap<FileId, NetCacheFile>
    // Each file has its own communication channel to identify it in a univocal way in the renderer thread.
    //
}


pub enum FileRendererUpdate {
    Redraw(ResizeResponse), Error(String)
}

impl NetCache{
    pub fn default() -> Self{
        Self{
            known_content_servers: Default::default(),
            known_file_list: Default::default(),
            known_media_list: Default::default(),
            downloaded_medias: Default::default(),
            downloaded_files: Default::default(),
        }
    }
}


pub enum NetCacheMedia {
    Downloading(f32), // f32 € [0,100] representing client loading status.
    Downloaded(PathBuf), // ByteStream of the file the clients sends to the Sim. Controller
    Rendering(f32), // f32 € [0,100] representing Sim. Controller loading status.
    Rendered(ThreadProtocol), // Rendered file that has to be
    // re-rendered each time Terminal changes its size.
    FatalErrorRendering(), // Fatal error rendering, file has to be re-sent!
}
impl Debug for NetCacheMedia{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self{
            NetCacheMedia::Downloading(it) => {write!(f, "Downloading {:?}",it)}
            NetCacheMedia::Downloaded(it) => {write!(f, "Downloaded {:?}",it)}
            NetCacheMedia::Rendering(it) => {write!(f, "Rendering {:?}",it)}
            NetCacheMedia::Rendered(_) => {write!(f, "Rendered")}
            NetCacheMedia::FatalErrorRendering() => {write!(f, "Fatal ErrorRendering")}
        }
    }
}
pub enum NetCacheFile{
    Downloading(f32), // f32 € [0,100] representing client loading status.
    Downloaded(PathBuf), // ByteStream of the file the clients sends to the Sim. Controller
    Rendering(f32),
    Rendered(FileType)
}
pub enum FileType{
    UTF8(Vec<String>),
    Other(PathBuf)
}

pub struct ChatCache{
    // Gets updated each time a chat-history gets modified via-server
    // Each server has its own ChatList saved
    pub local_chats: HashMap<ServerId, ChatList>,

    // Gets updated each time we receive the relative data-packet
    // Each client has its own list in each server.
    // First Key is ClientID, Value is all the contacts it has associated with the relative server.
    pub known_contacts: HashMap<ClientId, HashMap<ServerId, Vec<ClientId>>>,

    // Simply all the chat server the Client knows.
    pub known_chat_servers: HashMap<ClientId, Vec<ServerId>>,

    // First menu data
    // Starts to 0 and ends at known_chat_servers -> Vec<NodeId>.len()
    // Last number points to the button
    pub first_menu_scroll_offset: usize
}

impl ChatCache{
    pub fn default() ->Self{
        Self{
            local_chats: Default::default(), /*{
                let mut ret_hash = HashMap::new();
                ret_hash.insert(11, ChatList::default_dummy());
                ret_hash
            },*/
            known_contacts: Default::default(),
            known_chat_servers: Default::default(),
            first_menu_scroll_offset: 0,
        }
    }
}


#[derive(Debug)]
pub(crate) struct ChatList {
    pub chats_stored: HashMap<(ClientId, ClientId), Arc<RwLock<Chat>>>
}

impl ChatList {
    pub fn new() -> Self {
        Self {
            chats_stored: HashMap::new()
        }
    }

    // Insert a new chat
    pub fn insert_chat(&mut self, key: (ClientId, ClientId), chat: Chat) {
        let chat_arc = Arc::new(RwLock::new(chat));
        self.chats_stored.insert(key, chat_arc);
    }

    // Insert an already wrapped chat
    pub fn insert_chat_arc(&mut self, key: (ClientId, ClientId), chat_arc: Arc<RwLock<Chat>>) {
        self.chats_stored.insert(key, chat_arc);
    }

    // Get a chat for reading
    pub fn get_chat(&self, key: &(ClientId, ClientId)) -> Option<Arc<RwLock<Chat>>> {
        self.chats_stored.get(key).cloned()
    }

    // Read from a chat safely
    pub fn read_chat<F, R>(&self, key: &(ClientId, ClientId), f: F) -> Option<R>
    where
        F: FnOnce(&Chat) -> R,
    {
        self.chats_stored.get(key).map(|chat_arc| {
            let chat = chat_arc.read().unwrap();
            f(&*chat)
        })
    }

    // Write to a chat safely
    pub fn write_chat<F, R>(&self, key: &(ClientId, ClientId), f: F) -> Option<R>
    where
        F: FnOnce(&mut Chat) -> R,
    {
        self.chats_stored.get(key).map(|chat_arc| {
            let mut chat = chat_arc.write().unwrap();
            f(&mut *chat)
        })
    }
}

pub(crate) enum MediaSC {
    Image(/* A Reconstructed Image File */), File(/* A Reconstructed File */)
}

pub enum DroneType {
    RollingDrone,
    LockheedRustin,
    BagelBomber,
    FlyPath,
    RustDrone,
    RustasticDrone,
    BoberDrone,
    RustyDrone,
    RustaceansDrone,
    KrustyDrone,
    Unknown
}

impl Display for DroneType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            DroneType::RollingDrone => "RollingDrone",
            DroneType::LockheedRustin => "LockheedRustin",
            DroneType::BagelBomber => "BagelBomber",
            DroneType::FlyPath => "FlyPath",
            DroneType::RustDrone => "RustDrone",
            DroneType::RustasticDrone => "RustasticDrone",
            DroneType::BoberDrone => "BoberDrone",
            DroneType::RustyDrone => "RustyDrone",
            DroneType::RustaceansDrone => "RustaceansDrone",
            DroneType::KrustyDrone => "KrustyDrone",
            DroneType::Unknown => "Unknown"
        };
        write!(f, "{}", name)
    }
}


#[derive(Default, Clone, Copy)]
pub(crate) enum NodeType {
    #[default]
    Drone,
    Client,
    Server,
}

impl Display for NodeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeType::Drone => f.write_str("Drone"),
            NodeType::Client => f.write_str("Client"),
            NodeType::Server => f.write_str("Server"),
        }
    }
}
#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub enum NetNode { Drone(NodeId), Client(NodeId), Server(NodeId) }
impl NetNode {
    pub fn id(&self) -> &NodeId {
        match self {
            NetNode::Drone(id) => id,
            NetNode::Client(id) => id,
            NetNode::Server(id) => id,
        }
    }

    pub(crate) fn from(sim: & SimulationControllerData, node_id: NodeId) -> Option<NetNode> {
        for drone in sim.drones.iter() {
            if drone.id == node_id {
                return Some(NetNode::Drone(node_id));
            }
        }
        for client in sim.clients.iter() {
            if client.id == node_id {
                return Some(NetNode::Client(node_id));
            }
        }
        for server in sim.servers.iter() {
            if server.id == node_id {
                return Some(NetNode::Server(node_id));
            }
        }
        None
    }

    pub(crate) fn node_type(&self) -> NodeType {
        match self {
            NetNode::Drone(_) => NodeType::Drone,
            NetNode::Client(_) => NodeType::Client,
            NetNode::Server(_) => NodeType::Server,
        }
    }
}
pub(crate) type NodeMap = HashMap<NetNode, Vec<NodeId>>;

#[derive(Debug, Clone)]
pub(crate) enum NetworkOperation {
    CrashDrone(NodeId),
    RemoveConnection(NetNode, NetNode),
    AddConnection(NetNode, NetNode),
}
// Struct to cache network state and avoid rebuilding maps
pub(crate) struct NetworkState {
    pub(crate) node_map: NodeMap,
    pub(crate) drone_graph: HashMap<NodeId, Vec<NodeId>>,
    pub(crate) host_ids: HashSet<NodeId>,
}