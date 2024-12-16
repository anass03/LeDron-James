use crossbeam_channel::{Receiver, Sender};
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Packet, PacketType, FloodRequest, FloodResponse, Ack, Nack, NackType, NodeType};
use std::collections::{HashMap};
use std::fmt::{Debug, Display, Formatter};
use petgraph::algo::dijkstra;
use petgraph::Direction;
use serde::{Serialize, Deserialize };
use serde::de::DeserializeOwned;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::{EdgeRef, IntoEdgesDirected};
use wg_2024::controller::DroneEvent::{PacketDropped, PacketSent};
type FileId = usize;
type MediaId = usize;
type ClientId = NodeId;
pub struct Host{
    pub id:NodeId,
    pub type_host:NodeType,
    pub controller_send:Sender<DroneEvent>,
    pub controller_recv:Receiver<DroneCommand>,
    pub packet_recv:Receiver<Packet>,
    pub packet_send: HashMap<NodeId, Sender<Packet>>,
    pub flood_cache:HashMap<NodeId, u64>,
    topology: HashMap<NodeId,(NodeIndex,Vec<NodeId>)>,
    pub graph:DiGraph<u8, f64>,
    pub connected_drone_ids:Vec<NodeId>,
}
pub trait HosT{
    fn log<S: AsRef<str>>(&self, message: S);
    fn initiate_flood(&mut self);
    fn process_flood_request(&mut self, flood_request: FloodRequest,packet: Packet);
    fn process_flood_response(&mut self, response: FloodResponse);
    fn create_graph(&mut self)->DiGraph<u8,f64>;
    fn update_graph(&mut self);
    fn modify_pdr(&mut self, drone_id: NodeId, pdr: f64);
    fn find_best_route(&self, destination: NodeId) -> Vec<NodeId>;
    fn send_packet(&self, packet: Packet, channel: Option<&mut Sender<Packet>>) -> SendingCodes;
    fn sendto_controller(&self, packet: Packet, sent: bool) -> Result<(),String>;
    fn handle_ack(&self, ack: Ack, session_id: u64);
    fn handle_nack(&self, nack: Nack, session_id: u64);
    fn build_packet_flood_response(&self,floodrequest_header: FloodRequest, search_id:u64) -> Packet;
}
impl HosT for Host {
    fn log<S: AsRef<str>>(&self, message: S) {
        println!("Host ID {} - {}", self.id, message.as_ref());
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
        for i in &mut self.packet_send.clone(){
                self.send_packet(flood_request.clone(), Some(i.1));
        }
    }
    fn process_flood_request(&mut self, mut flood_request: FloodRequest,packet: Packet) {
        self.log("Handling FloodRequest...");
        if self.flood_cache.iter().any(|(k,v)| *v == flood_request.flood_id && *k == flood_request.initiator_id) { // Already received this FloodReq,
            // we need
            // to build a FloodResponse
            flood_request.path_trace.push((self.id.clone(), NodeType::Drone)); // We add our ID to the path
            self.send_packet(self.build_packet_flood_response(flood_request, packet.session_id),None);
        }else{
            let packetreceivedfrom = flood_request.path_trace.get(flood_request.path_trace.len()-1usize)
                .expect(("Server ID ".to_string()+self.id.to_string().as_str()+" - Received Flood Request with empty path-trace!").as_str()).clone();
            self.flood_cache.insert(flood_request.initiator_id.clone(), flood_request.flood_id.clone()); // **
            let mut return_packet = Packet{
                session_id: packet.session_id,
                routing_header: packet.routing_header,
                pack_type: {
                    let mut return_packet_typefield = flood_request.clone();
                    return_packet_typefield.path_trace.push((self.id.clone(), self.type_host)); // We're adding our ID to the path trace
                    PacketType::FloodRequest(return_packet_typefield)
                }
            };
            // We send the packet to all the neighbors besides the one we received the packet from
            for i in &mut self.packet_send.clone(){
                if *i.0 !=  packetreceivedfrom.0{
                    self.send_packet(return_packet.clone(), Some(i.1));
                }
            }
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
        println!("Host {}: Updated topology: {:?}", self.id, self.graph);
    }
    fn modify_pdr(&mut self, drone_id: NodeId, pdr: f64) {
       for i in self.graph.clone().edges_directed(self.topology.get(&drone_id).unwrap().0, Direction::Outgoing){
           if *i.weight() != pdr{
               if let Some(weight_mut) = self.graph.edge_weight_mut(i.id()) {
                   *weight_mut =pdr; // Example modification: increment weight by 1
               }
           }
       }
    }
    fn find_best_route(&self, destination: NodeId)->Vec<NodeId>{
        let shortest_paths = dijkstra(&self.graph, self.topology.get(&self.id).unwrap().0, Some(self.topology.get(&destination).unwrap().0), |edge| *edge.weight());
        // Reconstruct the path
        let mut current = self.topology.get(&destination).unwrap().0;
        let mut path = vec![current];
        // Traverse back using the cost map
        while let Some(prev_node) = shortest_paths.iter().find_map(|(node, &dist)| {
                self.graph
                    .edges_connecting(*node, current)
                    .find(|edge| dist + edge.weight() == shortest_paths[&current]).map(|_| *node)
             }){
            current = prev_node;
            path.push(current);
        } // Reverse the path to get it in start-to-end order
        path.reverse();
        let path_names: Vec<_> = path.iter().map(|&n| self.graph[n]).collect();
        println!("Shortest path: {:?}", path_names);
        path_names
    }
    fn send_packet(&self, packet: Packet, channel: Option<&mut Sender<Packet>>) -> SendingCodes { // OK
        self.log("Sending packet...");
        match channel {
            Some(ch) => {
                match ch.send(packet.clone()) {
                    Ok(_) => {
                        let e =SendingCodes::SuccessfullySent(packet.session_id);
                        self.sendto_controller(packet, false).expect("TODO: panic message"); //Ack to Sim. Controller
                        self.log("Successfully sent packet...");
                        e
                    },
                    Err(er) => {
                        println!("Server ID {:?} - {:?}", self.id, er.to_string());
                        let _ = self.sendto_controller(packet, false); // We send the packet to Sim.Controller
                        SendingCodes::ErrorSending(er.to_string())
                    }
                }
            },
            None => {
                let channeltosend = self.packet_send.get(&packet.clone().routing_header.hops[packet.routing_header.hop_index]);
                match channeltosend {
                    Some(ch) => {
                        match ch.send(packet.clone()) {
                            Ok(_) => {
                                self.log("Successfully sent packet...");
                                // let _ = self.sendto_controller(packet.clone(), true); //Ack to Sim. Controller
                                SendingCodes::SuccessfullySent(packet.session_id)
                            },
                            Err(er) => {
                                println!("Host ID {:?} - {:?}", self.id, er.to_string());
                                // let _ = self.sendto_controller(packet, true);
                                // We send the packet to Sim.Controller
                                SendingCodes::ErrorSending(er.to_string())
                            }
                        }
                    },
                    None => {
                        self.log("Neighbour not found...");
                        let _ = self.sendto_controller(packet, true); // We send the packet to Sim.Controller
                        SendingCodes::NoNextHop("Neighbour not found".to_string())
                    }
                }
            }
        }
    }
    fn sendto_controller(&self, packet: Packet, sent: bool) -> Result<(),String>{
        self.log("Sending to controller...");
        if sent {
            match self.controller_send.send(PacketSent(packet)){
                Ok(_) => {
                    self.log("Successfully sent to controller.");
                    Ok(())
                },
                Err(er) => {
                    println!("Drone ID {} - {:?}", self.id, er.to_string());
                    Err(er.to_string())
                }
            }
        }else{
            match self.controller_send.send(PacketDropped(packet)) {
                Ok(_) => {
                    self.log("Successfully sent to controller.");
                    Ok(())
                },
                Err(er) => {
                    println!("Drone ID {} - {:?}", self.id, er.to_string());
                    Err(er.to_string())
                }
            }
        }
    }
    fn handle_ack(&self, ack: Ack, session_id: u64) {
        println!(
            "Host {}: Received Ack for fragment index {} (session ID: {})",
            self.id, ack.fragment_index, session_id
        );
        // Aggiorna stato o logica per confermare la ricezione
        // Ad esempio: rimuovere il frammento dalla lista dei frammenti da inviare
    }
    fn handle_nack(&self, nack: Nack, session_id: u64) {
        println!("Host {}: Received Nack for fragment index {} (session ID: {}). Reason: {:?}", self.id, nack.fragment_index, session_id, nack.nack_type);
        match nack.nack_type {
            NackType::Dropped => {
                println!("Host {}: Retrying fragment due to drop.", self.id);
                // Logica per ritrasmettere il frammento
            }
            NackType::ErrorInRouting(node_id) => {
                println!(
                    "Host {}: Routing error encountered. Node ID: {}",
                    self.id, node_id
                );
                // Adotta azioni per correggere il percorso, se possibile
            }
            _ => {
                println!("Host {}: Unhandled Nack type: {:?}", self.id, nack.nack_type);
            }
        }
    }
    fn build_packet_flood_response(&self,mut floodrequest_header: FloodRequest, search_id:u64) -> Packet{
        let return_packet:Packet = Packet{
            routing_header: SourceRoutingHeader{
                // Hop Index 1,
                hop_index:1,
                hops: {
                    floodrequest_header.path_trace.iter().cloned().rev().map(|(id, _)| id).collect()
                }
            },
            pack_type: {
                PacketType::FloodResponse(FloodResponse{
                    flood_id:floodrequest_header.flood_id,
                    path_trace:floodrequest_header.path_trace,
                })
            },
            session_id: search_id// random number or session id from the old packet?,
        };
        println!("Host ID {} - Building Flood Response", return_packet.routing_header.hops[0]);
        return_packet
    }
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

pub trait DroneSend: Serialize + DeserializeOwned {
    fn stringify(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
    fn from_string(raw: String) -> Result<Self, String> {
        serde_json::from_str(raw.as_str()).map_err(|e| e.to_string())
    }
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
#[test]
fn main(){
    println!("Hello, world!");
}
