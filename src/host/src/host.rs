// File: src/host.rs
use crate::functionality::Functionality;
use crate::flood::FloodControl;
use crate::logging::{LogEntry, LogType, LogModule, LogControl};
use crate::network::{Network};
use crate::session::{RecSessionData, SentSessionData};

use crossbeam_channel::{Receiver, Sender};
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use wg_2024::network::NodeId;
use wg_2024::packet::{Packet, NodeType};

pub struct Host {
    pub id: NodeId,
    pub type_host: NodeType,
    pub controller_send: Sender<HostEvent>,
    pub packet_recv: Receiver<Packet>,
    pub packet_send: HashMap<NodeId, Sender<Packet>>,
    pub flood_cache: HashMap<NodeId, Vec<u64>>,
    pub network: Network,
    pub received_fragments: Arc<Mutex<HashMap<u64, RecSessionData>>>,
    pub sent_fragments: Arc<Mutex<HashMap<u64, SentSessionData>>>,
    pub log_sender: Sender<LogEntry>,
    pub(crate) nack_flood_backoff: Duration,
    pub(crate) last_nack_flood: Instant,
}

impl Host {
    pub fn new(
        id: NodeId,
        type_host: NodeType,
        controller_send: Sender<HostEvent>,
        packet_recv: Receiver<Packet>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        log_sender: Sender<LogEntry>,
    ) -> Host {
        Self {
            id,
            type_host,
            controller_send,
            packet_recv,
            packet_send,
            flood_cache: HashMap::new(),
            network: Network::new(),
            received_fragments: Arc::new(Mutex::new(HashMap::new())),
            sent_fragments: Arc::new(Mutex::new(HashMap::new())),
            log_sender,
            nack_flood_backoff: Duration::from_secs(5),
            last_nack_flood: Instant::now() - Duration::from_secs(5),
        }
    }
}


// host can now call .host.function() thanks to trait impls

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientNetCommand {
    ServerType,
    FilesList(NodeId),
    File(NodeId, String),
    NewFile(NodeId, String, Vec<u8>),
    MediaList(NodeId),
    Media(NodeId, String),
    NewMedia(NodeId, String, super::Image),
    RegistrationToChat(NodeId),
    ClientList(NodeId),
    MessageFor(NodeId, NodeId, String),
    FileFor(NodeId, NodeId, String, Vec<u8>),
    MediaFor(NodeId, NodeId, String, super::Image),
}

pub enum HostCommand {
    AddSender(NodeId, Sender<Packet>),
    RemoveSender(NodeId),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerToClient {
    ServerType(ServerTypes),
    Sending(ContentID, usize),
    FilesList(NodeId, NodeId, Vec<String>),
    File(NodeId, String, Vec<u8>),
    MediaList(NodeId, NodeId, Vec<String>),
    Media(NodeId, String, super::Image),
    ClientList(NodeId, NodeId, Vec<NodeId>),
    MessageFrom(NodeId, NodeId, String),
    FileFrom(NodeId, NodeId, String, Vec<u8>),
    MediaFrom(NodeId, NodeId, String, super::Image),
    ErrNotFound(NodeId),
    ErrClientId(NodeId),
    WrongServer(NodeId),
}

pub enum HostEvent {
    PacketSent(Packet),
    PacketReconstructed(ServerToClient),
    ServersDiscovered(NodeId, HashMap<ServerTypes, Vec<NodeId>>),
    ExpectedPDR(HashMap<NodeId, f64>),
    NetSpeed(f32, f32),
    UsageCPU(f32),
    DownloadingInfo(ContentID, f32),
    UploadingInfo(ContentID, f32),
    ChatUpdate(NodeId, String),
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ContentID {
    Media(String),
    File(String),
}

#[derive(Hash, Eq, Clone, PartialEq, Debug, Deserialize, Serialize)]
pub enum ServerTypes {
    Chat,
    Content,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub session_id: u64,
    pub content: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

pub enum SendingCodes {
    SuccessfullySent(u64),
    ErrorSending(String),
    NoNextHop(String),
}

impl Debug for SendingCodes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SendingCodes::SuccessfullySent(id) => write!(f, "Successfully sent [{}]", id),
            SendingCodes::ErrorSending(msg) => write!(f, "Error sending [{}]", msg),
            SendingCodes::NoNextHop(msg) => write!(f, "No next hop [{}]", msg),
        }
    }
}

impl Display for SendingCodes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}
