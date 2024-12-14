use crossbeam_channel::{Receiver, Sender};
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Packet, PacketType, FloodRequest, FloodResponse, Ack, Nack, NackType, NodeType, Fragment};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use serde::{ Serialize, Deserialize };
use serde::de::DeserializeOwned;
use petgraph::graph::{DiGraph, NodeIndex};

type FileId = usize;
type MediaId = usize;
type ClientId = NodeId;

pub trait Host{
    fn create_graph(&self)->DiGraph<u8,i32>{
        DiGraph::new()
    }
    fn initiate_flood(&mut self) {
        let flood_request = Packet {
            pack_type: PacketType::FloodRequest(FloodRequest {
                flood_id: rand::random(),
                initiator_id: self.id,
                path_trace: vec![(self.id, NodeType::Client)],
            }),
            routing_header: SourceRoutingHeader {
                hop_index: 0,
                hops: vec![self.id],
            },
            session_id: rand::random(),
        };

        if self.packet_send.send(flood_request).is_err() {
            println!("Client {}: Error initiating flood", self.id);
        }
    }

    fn process_flood_request(&mut self, flood_request: FloodRequest) {
        if self.flood_cache.contains_key(&flood_request.flood_id) {
            return; // Già processata
        }
        self.flood_cache.insert(flood_request.flood_id, true);

        println!("Client {}: Processing FloodRequest {:?}", self.id, flood_request);

        // Inoltra la richiesta di Flood a droni collegati
        for &drone_id in &self.connected_drone_ids {
            let mut request = flood_request.clone();
            request.path_trace.push((drone_id, NodeType::Drone));
            let packet = Packet {
                pack_type: PacketType::FloodRequest(request),
                routing_header: SourceRoutingHeader {
                    hop_index: 1,
                    hops: vec![self.id, drone_id],
                },
                session_id: rand::random(),
            };
            if let Err(e) = self.packet_send.send(packet) {
                println!("Client {}: Error forwarding FloodRequest: {:?}", self.id, e);
            }
        }
    }


    fn process_flood_response(&mut self, response: FloodResponse) {
        println!(
            "Client {}: Processing FloodResponse for flood ID: {}",
            self.id, response.flood_id
        );

        // Itera sui nodi nel percorso della FloodResponse
        let path = &response.path_trace;
        for i in 0..path.len() - 1 {
            let (current_node, _) = path[i];
            let (next_node, _) = path[i + 1];

            // Ottieni la PDR tra i nodi
            let pdr = self.pdr_data.get(&next_node).cloned().unwrap_or(1.0); // Default PDR 1.0

            // Aggiungi il collegamento al grafo
            self.graph.add_edge(current_node, next_node, pdr);
            self.graph.add_edge(next_node, current_node, pdr); // Per grafi bidirezionali
        }

        println!("Client {}: Updated topology: {:?}", self.id, self.graph);
    }
}

// fn evaluate_drones(&mut self) {
//     for &drone_id in &self.connected_drone_ids {
//         let mut sent = 0;
//         let mut acked = 0;
//
//         // Testa con un certo numero di pacchetti
//         for _ in 0..10 {
//             let test_packet = Packet {
//                 pack_type: PacketType::MsgFragment(Fragment {
//                     fragment_index: 0,
//                     total_n_fragments: 1,
//                     length: 0,
//                     data: [0; 128],
//                 }),
//                 routing_header: SourceRoutingHeader {
//                     hop_index: 0,
//                     hops: vec![self.id, drone_id],
//                 },
//                 session_id: rand::random(),
//             };
//
//             if self.packet_send.send(test_packet).is_ok() {
//                 sent += 1;
//             }
//
//             // Verifica se riceviamo un Ack
//             if let Ok(DroneEvent::PacketSent(_)) = self.controller_recv.recv_timeout(Duration::from_millis(100)) {
//                 acked += 1;
//             }
//         }
//
//         // Calcola la PDR
//         let pdr = if sent > 0 { acked as f32 / sent as f32 } else { 1.0 };
//         self.pdr_data.insert(drone_id, pdr);
//
//         println!(
//             "Client {}: Estimated PDR for drone {}: {:.2}",
//             self.id, drone_id, pdr
//         );
//     }
// }

fn find_best_route(&self, destination: NodeId) -> Vec<NodeId> {
    let mut dist = HashMap::new();
    let mut prev = HashMap::new();
    let mut unvisited: HashSet<NodeId> = self.graph.adj_list.keys().cloned().collect();

    for &node in &unvisited {
        dist.insert(node, f32::INFINITY);
    }
    dist.insert(self.id, 0.0);

    while let Some(&current) = unvisited.iter().min_by_key(|&&node| dist.get(&node).cloned().unwrap_or(f32::INFINITY) as i32) {
        unvisited.remove(&current);

        if current == destination {
            break;
        }

        for &(neighbor, weight) in &self.graph.get_neighbors(current) {
            let alt = dist.get(&current).cloned().unwrap_or(f32::INFINITY) + weight;
            if alt < dist.get(&neighbor).cloned().unwrap_or(f32::INFINITY) {
                dist.insert(neighbor, alt);
                prev.insert(neighbor, current);
            }
        }
    }

    // Ricostruisce il percorso
    let mut path = Vec::new();
    let mut current = destination;
    while let Some(&prev_node) = prev.get(&current) {
        path.push(current);
        current = prev_node;
    }
    path.push(self.id);
    path.reverse();
    path
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
    // File(),
    // Media(),
    ErrNotFound,
    ErrNotSupported,
    ClientList(Vec<ClientId>),
    MessageFrom(ClientId, Message<M>),
    ErrClientId
}
enum ServerType{
    Chat,
    Content
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

// ReqServerType,
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextRequest {
    TextList,
    Text(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaRequest {
    MediaList,
    Media(u64),
}
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextResponse {
    TextList(Vec<u64>),
    Text(String),
    NotFound,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaResponse {
    MediaList(Vec<u64>),
    Media(Vec<u8>), // should we use some other type?
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatResponse {
    ClientList(Vec<NodeId>),
    MessageFrom { from: NodeId, message: Vec<u8> },
    MessageSent,
}








pub trait Request: DroneSend {}
pub trait Response: DroneSend {}

impl DroneSend for TextRequest {}
impl Request for TextRequest {}
impl DroneSend for MediaRequest {}
impl Request for MediaRequest {}
impl DroneSend for ChatRequest {}
impl Request for ChatRequest {}
impl DroneSend for TextResponse {}
impl Response for TextResponse {}
impl DroneSend for MediaResponse {}
impl Response for MediaResponse {}

impl DroneSend for ChatResponse {}
impl Response for ChatResponse {}