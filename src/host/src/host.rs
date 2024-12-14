use crossbeam_channel::{Receiver, Sender};
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Packet, PacketType, FloodRequest, FloodResponse, Ack, Nack, NackType, NodeType, Fragment};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use petgraph::algo::dijkstra;
use serde::{Serialize, Deserialize };
use serde::de::DeserializeOwned;
use petgraph::graph::{DiGraph, NodeIndex};

type FileId = usize;
type MediaId = usize;
type ClientId = NodeId;
//pub struct Client {
//     topology: HashMap<NodeId, Vec<NodeId>>, // Memorizza la topologia
//     pdr_data: HashMap<NodeId, f32>, // PDR stimata per ogni nodo connesso
// }


pub struct Host{
    pub id:NodeId,
    pub controller_send:Sender<DroneEvent>,
    pub controller_recv:Receiver<DroneCommand>,
    pub packet_recv:Receiver<Packet>,
    pub packet_send: HashMap<NodeId, Sender<Packet>>,
    pub flood_cache:HashMap<u64,bool>,
    topology: HashMap<NodeId,(NodeIndex,Vec<NodeId>)>,
    pub graph:DiGraph<u8, f64>,
    pub connected_drone_ids:Vec<NodeId>,
}

pub trait HosT {
    fn find_best_route(&self, destination: NodeId) -> Vec<NodeId>;
    fn process_flood_response(&mut self, response: FloodResponse);
    fn process_flood_request(&mut self, flood_request: FloodRequest);
    fn initiate_flood(&mut self);
    fn create_graph(&mut self)->DiGraph<u8,f64>;
    fn update_graph(&mut self);
    fn log<S: AsRef<str>>(&self, message: S);
    fn modify_pdr(&mut self, drone_id: NodeId, pdr: f32);
}
impl<S: AsRef<str>> HosT for Host {
    fn find_best_route(&self, destination: NodeId){
        let mut graph = DiGraph::new();

        // Add nodes
        let a = graph.add_node("A"); // Node 0
        let b = graph.add_node("B"); // Node 1
        let c = graph.add_node("C"); // Node 2

        // Add weighted edges
        graph.add_edge(a, b, 1); // A -> B (weight 1)
        graph.add_edge(b, c, 2); // B -> C (weight 2)
        graph.add_edge(a, c, 4); // A -> C (weight 4)

        // Perform Dijkstra's algorithm from node A
        let start = a;
        let goal = c;

        let shortest_paths = dijkstra(&graph, start, Some(goal), |edge| *edge.weight());

        // Check the result
        if let Some(cost) = shortest_paths.get(&goal) {
            println!("The shortest path cost from {:?} to {:?} is {}", graph[start], graph[goal], cost);
        } else {
            println!("There is no path from {:?} to {:?}", graph[start], graph[goal]);
        }
    }
    fn process_flood_response(&mut self, response: FloodResponse) {
        // Itera sui nodi nel percorso della FloodResponse
        let path = &response.path_trace;
        for j in 0..path.len()-1{
            let i = path[j];
           match self.topology.contains_key(&i.0){
               true =>{
                   let (_,v)=self.topology.get_mut(&i.0).unwrap();
                   match v.contains(&path[j+1].0) {
                       true => {}
                       false => {v.push(path[j+1].0);}
                   }
               }
               false => {
                   self.topology.insert(i.0, (self.graph.add_node(i.0), vec![]));
               }
           }
        }
        self.update_graph();
        // adding nodes how?
    }
    fn process_flood_request(&mut self, flood_request: FloodRequest) {
        todo!()
    }
    fn initiate_flood(&mut self) {
        todo!()
    }
    fn create_graph(&mut self)->DiGraph<u8,f64>{
        DiGraph::new()
    }
    fn update_graph(&mut self){
        for (i,j) in self.topology.values(){
            for k in j{
                match self.graph.contains_edge(*i, self.topology.get(k).unwrap().0) {
                    true => {match self.graph.contains_edge( self.topology.get(k).unwrap().0,*i){
                        true => {}
                        false => {self.graph.add_edge( self.topology.get(k).unwrap().0,*i, 0.0);}
                    }}
                    false => {self.graph.add_edge(*i, self.topology.get(k).unwrap().0, 0.0);}
                }
            }
        }
        println!("Client {}: Updated topology: {:?}", self.id, self.graph);
    }
    fn log<S: AsRef<str>>(&self, message: S) {
        println!("Host ID {} - {}", self.id, message.as_ref());
    }
    fn modify_pdr(&mut self, drone_id: NodeId, pdr: f32) {
       // sto lavorando
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