use crossbeam_channel::{Receiver, Sender};
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Packet, PacketType, FloodRequest, FloodResponse, Ack, Nack, NackType, NodeType, Fragment};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use serde::{ Serialize, Deserialize };
use serde::de::DeserializeOwned;

type FileId = usize;
type MediaId = usize;
type ClientId = NodeId;

pub trait Host{
    fn handle_packet(&mut self, packet: Packet);
    fn handle_ack(&self, ack: Ack, session_id: u64);
    fn handle_nack(&self, nack: Nack, session_id: u64);
    fn initiate_flood(&mut self);
    fn process_flood_request(&mut self, flood_request: FloodRequest);
    fn process_flood_response(&mut self, response: FloodResponse);
    fn handle_fragment(&self, fragment: Fragment);
    fn evaluate_drones(&mut self);
    fn find_best_route(&self, destination: NodeId) -> Vec<NodeId>;
}
pub enum ClientToServer<M: DroneSend>{
    ServerType,
    FilesList,
    File(FileId),
    MediaList,
    Media(MediaId),

    RegistrationToChat,
    ClientList,
    MessageFor(ClientId, Message<M>)
}
pub enum ServerToClient<M: DroneSend>{
    ServerType(ServerType),
    FilesList(Vec<FileId>),
    MediaList(Vec<MediaId>),
    //File(file_size, file),
    //Media(),
    ErrNotFound,
    ErrNotSupported,
    ClientList(Vec<ClientId>),
    MessageFrom(ClientId, Message<M>),
    ErrClientId
}
enum ServerType{
    Chat,
    Media
}

///MESSAGES WG_2024
pub struct Message<M: DroneSend> {
    pub source_id: NodeId,
    pub session_id: u64,
    pub content: M,
}
pub trait DroneSend: Serialize + DeserializeOwned {
    fn stringify(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
    fn from_string(raw: String) -> Result<Self, String> {
        serde_json::from_str(raw.as_str()).map_err(|e| e.to_string())
    }
}

pub trait Request: DroneSend {}
pub trait Response: DroneSend {}

// ReqServerType,
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextRequest {
    TextList,
    Text(u64),
}

impl DroneSend for TextRequest {}
impl Request for TextRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaRequest {
    MediaList,
    Media(u64),
}

impl DroneSend for MediaRequest {}
impl Request for MediaRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatRequest {
    ClientList,
    Register(NodeId),
    SendMessage {
        from: NodeId,
        to: NodeId,
        message: String,
    },
}

impl DroneSend for ChatRequest {}
impl Request for ChatRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextResponse {
    TextList(Vec<u64>),
    Text(String),
    NotFound,
}

impl DroneSend for TextResponse {}
impl Response for TextResponse {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaResponse {
    MediaList(Vec<u64>),
    Media(Vec<u8>), // should we use some other type?
}

impl DroneSend for MediaResponse {}
impl Response for MediaResponse {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatResponse {
    ClientList(Vec<NodeId>),
    MessageFrom { from: NodeId, message: Vec<u8> },
    MessageSent,
}

impl DroneSend for ChatResponse {}
impl Response for ChatResponse {}