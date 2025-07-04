use crossbeam_channel::{Receiver, Sender};
use wg_2024::network::{NodeId};
use wg_2024::packet::{Packet,NodeType};
use std::collections::{HashMap};
use std::fmt::{Debug, Display, Formatter};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use crate::network::{Network};
use crate::session::{RecSessionData, SentSessionData};

pub type FileId = String;
pub type MediaId = String;
pub type ClientId = NodeId;
pub type ServerId = NodeId;
pub type Upload = f32;
pub type Download = f32;
pub type Usage = f32;

#[derive(Clone)]
pub struct Host{
    pub id:NodeId,
    pub type_host:NodeType,
    pub controller_send:Sender<HostEvent>,
    pub packet_recv:Receiver<Packet>,
    pub packet_send: HashMap<NodeId, Sender<Packet>>,
    pub flood_cache:HashMap<NodeId, Vec<u64>>,
    pub network: Network,
    pub received_fragments: Arc<Mutex<HashMap<u64, RecSessionData>>>,
    pub sent_fragments: Arc<Mutex<HashMap<u64, SentSessionData>>>,
    /// quanto vogliamo aspettare tra un flood e il successivo
    pub(crate) nack_flood_backoff: Duration,
    /// timestamp dell’ultimo flood scatenato da un NACK
    pub(crate) last_nack_flood: Instant,

}
impl Host{
    pub fn new(id:NodeId,
               type_host:NodeType,
               controller_send:Sender<HostEvent>,
               packet_recv:Receiver<Packet>,
               packet_send:HashMap<NodeId, Sender<Packet>>) -> Host {
        let host = Self {
            id,
            type_host,
            controller_send,
            packet_recv,
            packet_send,
            flood_cache: HashMap::new(),
            network: Network::new(),
            received_fragments: Arc::new(Mutex::new(HashMap::new())),
            sent_fragments: Arc::new(Mutex::new(HashMap::new())),
            nack_flood_backoff: Duration::from_secs(5),
            last_nack_flood: Instant::now() - Duration::from_secs(5),
        };
        host
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub enum ClientNetCommand {
    //client command
    ServerType,
    //content-server
    FilesList(ServerId),
    File(ServerId, FileId),
    NewFile(ServerId, FileId, Vec<u8>),

    MediaList(ServerId),
    Media(ServerId,MediaId),
    NewMedia(ServerId, MediaId, Image),

    //chat-server
    RegistrationToChat(ServerId),
    ClientList(ServerId),
    MessageFor(ClientId,ServerId, String),
    FileFor(ClientId,ServerId, FileId, Vec<u8>),
    MediaFor(ClientId,ServerId, MediaId, Image),
}
pub enum HostCommand {
    AddSender(NodeId, Sender<Packet>),
    RemoveSender(NodeId),
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerToClient{
    ServerType(ServerTypes),
    Sending(ContentID, usize),

    //content server
    FilesList(ClientId, ServerId, Vec<FileId>),
    File(ServerId, FileId, Vec<u8>),

    MediaList(ClientId, ServerId, Vec<MediaId>),
    Media(ServerId, MediaId, Image),

    //chat server
    ClientList(ClientId, ServerId, Vec<ClientId>),
    MessageFrom(ServerId, ClientId, String),
    FileFrom(ServerId, ClientId, FileId, Vec<u8>),
    MediaFrom(ServerId, ClientId, MediaId, Image),

    ErrNotFound(ServerId),
    ErrClientId(ServerId),
    WrongServer(ServerId)
}
/// Application events that comes to the SimulationController
pub enum HostEvent {
    //both
    /// I report sending a low level packet (to be wrapped in the controller if needed)
    PacketSent(Packet),
    ///Route used by host
    Route(HostID, Vec<NodeId>),

    //client
    /// Rebuilt entire high level message(es. from server to client)
    PacketReconstructed(ServerToClient),
    ///Servers discovered, response at ServerType
    ServersDiscovered(ClientId, HashMap<ServerTypes,Vec<ServerId>>),
    ///Expected pdr from client
    ExpectedPDR(HashMap<NodeId, f64>),
    ///Network speed
    NetSpeed(Upload, Download),
    ///Info on downloading/uploading the entire data
    DownloadingInfo(ServerId, ContentID, f32),
    UploadingInfo(ServerId, ContentID, f32),
    //server
    ///for real time chat
    ChatUpdate(ServerId, String)
}
pub enum HostID{
    ServerId(ServerId),
    ClientId(ClientId),
}
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ContentID{
    Media(MediaId),
    File(FileId),
}
#[derive(Hash, Eq,Clone, PartialEq, Debug,Deserialize, Serialize)]
pub enum ServerTypes{
    Chat,
    Content
}
///MESSAGES WG_2024
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub session_id: u64,
    pub content: Vec<u8>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Image{
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

pub enum SendingCodes { // For performance and logic enhancements
    SuccessfullySent(u64),
    ErrorSending(String),
    NoNextHop(String)
}
impl Debug for SendingCodes {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            SendingCodes::SuccessfullySent(n) => write!(f, "Successfully sent [Packet {}]", n),
            SendingCodes::ErrorSending(s) => write!(f, "Error sending the packet [{}]", s),
            SendingCodes::NoNextHop(s) => write!(f, "No next hop available [{}]", s)
        }
    }
}
impl Display for SendingCodes {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}
