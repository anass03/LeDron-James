use crossbeam_channel::{Receiver, Sender};
use wg_2024::controller::{DroneEvent};
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Packet, FloodRequest, FloodResponse, Ack, Nack, NackType, NodeType, Fragment, FRAGMENT_DSIZE};
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use petgraph::algo::dijkstra;
use petgraph::Direction;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::{EdgeRef, IntoEdgesDirected};
use rand::Rng;
use rand::rngs::ThreadRng;
use serde::__private::de::IdentifierDeserializer;
use serde_json::to_string;
use wg_2024::controller::DroneEvent::{PacketSent};
type FileId = usize;
type MediaId = usize;
type ClientId = NodeId;
type ServerId = NodeId;

#[derive(Clone)]
pub struct Host{
    pub id:NodeId,
    pub type_host:NodeType,
    pub controller_send:Sender<DroneEvent>,
    pub controller_recv:Receiver<ClientCommand>,
    pub packet_recv:Receiver<Packet>,
    pub packet_send: HashMap<NodeId, Sender<Packet>>,
    pub flood_cache:HashMap<NodeId, u64>,
    pub network: Network,
    pub received_fragments: Arc<Mutex<HashMap<u64, RecSessionData>>>,
    pub sent_fragments: Arc<Mutex<HashMap<u64, SentSessionData>>>,
}
impl Host{
    pub fn new(id:NodeId,
               type_host:NodeType,
               controller_send:Sender<DroneEvent>,
               controller_recv:Receiver<ClientCommand>,
               packet_recv:Receiver<Packet>,
               packet_send:HashMap<NodeId, Sender<Packet>>) -> Host {

        let host = Self {
            id,
            type_host,
            controller_send,
            controller_recv,
            packet_recv,
            packet_send,
            flood_cache: HashMap::new(),
            network: Network::new(),
            received_fragments: Arc::new(Mutex::new(HashMap::new())),
            sent_fragments: Arc::new(Mutex::new(HashMap::new())),
        };
        let host_arc = Arc::new(host.clone());
        host_arc.start_tasks();
        host
    }
}

struct RecSessionData {
    buffer: Vec<Option<Fragment>>,
    last_activity: Instant, // Timestamp dell'ultima attività
}
pub struct SentSessionData {
    pub buffer: Vec<Option<Fragment>>, // Buffer dei frammenti inviati
    pub last_activity: Instant,        // Timestamp dell'ultima attività
    pub destination: NodeId,        // Destinatario della sessione
    pub route: Vec<NodeId>,        // percorso pre-calcolato
}
#[derive(Clone)]
pub struct Network {
    pub graph: DiGraph<NodeId, f64>,
    pub topology: HashMap<NodeId, (NodeIndex, Vec<NodeId>)>, // NodeId -> (Graph NodeIndex, Vec of neighbors)
    pub servers: HashSet<NodeId>,
}
impl Network {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            topology: HashMap::new(),
            servers: HashSet::new()
        }
    }
    /// Aggiorna il grafo basandosi sulla topologia attuale
    pub fn update_graph(&mut self) {
        // Rimuovi tutti gli archi esistenti
        self.graph.clear_edges();

        for (_, (node_index, neighbors)) in &self.topology {
            for neighbor_id in neighbors {
                let neighbor_index = self.topology.get(neighbor_id).unwrap().0;
                if !self.graph.contains_edge(*node_index, neighbor_index) {
                    self.graph.add_edge(*node_index, neighbor_index, 0.0);
                }
                if !self.graph.contains_edge(neighbor_index, *node_index) {
                    self.graph.add_edge(neighbor_index, *node_index, 0.0);
                }
            }
        }
    }

    /// Trova il percorso migliore usando Dijkstra
    pub fn find_best_route(&self, start: NodeId, destination: NodeId) -> Vec<NodeId> {
        let start_index = self.topology.get(&start).unwrap().0;
        let destination_index = self.topology.get(&destination).unwrap().0;

        let shortest_paths = dijkstra(&self.graph, start_index, Some(destination_index), |edge| *edge.weight());
        let mut current = destination_index;
        let mut path = vec![current];

        while let Some(prev_node) = shortest_paths.iter().find_map(|(node, &dist)| {
            self.graph
                .edges_connecting(*node, current)
                .find(|edge| dist + edge.weight() == shortest_paths[&current])
                .map(|_| *node)
        }) {
            current = prev_node;
            path.push(current);
        }

        path.reverse();
        path.iter().map(|&n| self.graph[n]).collect()
    }

    /// Trova un percorso alternativo, cercando comunque di soddisfare le condizioni di efficienza
    pub fn find_alternate_route(&self, start: NodeId, destination: NodeId, used_route: &[NodeId]) -> Option<Vec<NodeId>> {
        // Identifica gli indici dei nodi nel grafo
        let start_index = self.topology.get(&start).unwrap().0;
        let destination_index = self.topology.get(&destination).unwrap().0;

        // Crea una copia modificabile del grafo
        let mut graph = self.graph.clone();

        // Rimuovi gli archi che fanno parte del percorso precedente
        for window in used_route.windows(2) {
            if let [prev, next] = window {
                if let Some(&prev_index) = self.topology.get(prev).map(|(i, _)| i) {
                    if let Some(&next_index) = self.topology.get(next).map(|(i, _)| i) {
                        // Trova l'arco tra prev_index e next_index
                        if let Some(edge_index) = graph.find_edge(prev_index, next_index) {
                            graph.remove_edge(edge_index); // Rimuovi l'arco
                        }
                    }
                }
            }
        }

        // Applica Dijkstra sul grafo modificato
        let shortest_paths = dijkstra(&graph, start_index, Some(destination_index), |edge| *edge.weight());
        if !shortest_paths.contains_key(&destination_index) {
            return None; // Nessun percorso alternativo trovato
        }

        // Costruisci il percorso alternativo
        let mut current = destination_index;
        let mut path = vec![current];

        while let Some(prev_node) = shortest_paths.iter().find_map(|(node, &dist)| {
            graph
                .edges_connecting(*node, current)
                .find(|edge| dist + edge.weight() == shortest_paths[&current])
                .map(|_| *node)
        }) {
            current = prev_node;
            path.push(current);
        }

        path.reverse();
        Some(path.iter().map(|&n| self.graph[n]).collect())
    }


}
pub trait Functionality{
    fn log<S: AsRef<str>>(&self, message: S);
    fn initiate_flood(&mut self);
    fn process_flood_request(&mut self, flood_request: FloodRequest,packet: Packet);
    fn process_flood_response(&mut self, response: FloodResponse);
    fn send_packet(&self, packet: Packet, channel: Option<&Sender<Packet>>) -> SendingCodes;
    fn sendto_controller(&self, packet: Packet, sent: bool) -> Result<(),String>;
    fn handle_ack(&self, ack: Ack, session_id: u64);
    fn handle_nack(&self, nack: Nack, session_id: u64);
    fn handle_msg_fragment(&mut self, packet: Packet,fragment: Fragment) -> Option<Vec<u8>>;
    fn cleanup_expired_sessions(&self, timeout: Duration);
    fn start_session_cleanup(self: Arc<Self>, timeout: Duration, interval: Duration);
    fn get_fragment(&self, session_id: u64, fragment_index: u64) -> Option<Fragment>;
    fn send_fragment(&self, session_id: u64, fragment: Fragment);
    fn start_retransmission_task(self: Arc<Self>, retransmission_interval: Duration);
    fn build_packet_ack(&self,packet: Packet, fragment_index: Option<u64>) -> Packet;
    fn start_tasks(self: Arc<Self>);
}
fn generate_unique_flood_id() -> u64 {
    let mut rng: ThreadRng = rand::thread_rng();  // Usa il generatore di numeri casuali sicuro per il thread
    rng.gen::<u64>()  // Genera un numero casuale di tipo u64
}
pub fn generate_unique_session_id() -> u64 {
    let mut rng: ThreadRng = rand::rng();  // messo rng al posto di thread_rng pk quest'ultimo dalla prossima versione verrà rinominato in rng! [thread_rng --> rng]
    rng.random::<u64>()  // lo stesso qui, [gen  -->  random]
}
impl Functionality for Host {
    fn log<S: AsRef<str>>(&self, message: S) {
        println!("Host ID {} - {}", self.id, message.as_ref());
    }
    fn initiate_flood(&mut self) {
        let flood_request = Packet::new_flood_request(
            SourceRoutingHeader::new(vec![self.id], 0),
            generate_unique_session_id(),
            FloodRequest::initialize(generate_unique_flood_id(),self.id, self.type_host),
        );
        // Mando la richiesta di flood ai vicini
        for (neighbour_id, sender) in &self.packet_send {
            self.log(&format!(
                "Sending flood request to neighbour: {:?}",
                neighbour_id
            ));
            let result = self.send_packet(flood_request.clone(), Some(sender));
            self.log(&format!("Flood result for {:?}: {:?}", neighbour_id, result));
        }
    }
    fn process_flood_request(&mut self, flood_request: FloodRequest, packet: Packet) {
        self.log("Handling FloodRequest...");

        // Verifica se la richiesta di flood è già stata processata
        if self.flood_cache.contains_key(&flood_request.initiator_id)
            && self.flood_cache[&flood_request.initiator_id] == flood_request.flood_id
        {
            self.log("Flood request already processed, sending response...");
            self.send_packet(flood_request.generate_response(packet.session_id), None);
            return;
        }

        // Aggiorna il cache per evitare di processare di nuovo la stessa richiesta
        self.flood_cache.insert(flood_request.initiator_id.clone(), flood_request.flood_id);

        // Ottiene il nodo dal quale è arrivata la richiesta
        let packet_received_from = flood_request
            .path_trace
            .last()
            .expect(&format!(
                "Server ID {} - Received Flood Request with empty path trace!",
                self.id
            ))
            .clone();

        self.log(&format!(
            "FloodRequest received from node: {:?}",
            packet_received_from
        ));

        // Aggiorna il percorso del flood aggiungendo il nodo corrente
        let mut updated_flood_request = flood_request.clone();
        updated_flood_request.increment(self.id.clone(), self.type_host);

        // Controlla se ci sono altri vicini oltre al nodo da cui è arrivato il pacchetto
        let neighbors_to_send: Vec<_> = self
            .packet_send
            .iter()
            .filter(|(neighbor_id, _)| *neighbor_id != &packet_received_from.0)
            .collect();

        if neighbors_to_send.is_empty() {
            // Non ci sono vicini a cui inoltrare, genera direttamente una risposta
            self.log("No neighbors to forward FloodRequest, generating response...");

            let response_packet = updated_flood_request.generate_response(packet.session_id);
            self.send_packet(response_packet, None);
        } else {
            // Inoltra il pacchetto aggiornato ai vicini
            self.log("Forwarding FloodRequest to neighbors...");
            let updated_packet = Packet::new_flood_request(packet.routing_header,packet.session_id,updated_flood_request);

            for (neighbor_id, sender) in neighbors_to_send {
                self.log(&format!("Forwarding FloodRequest to neighbor: {:?}", neighbor_id));
                self.send_packet(updated_packet.clone(), Some(sender));
            }
        }
    }
    fn process_flood_response(&mut self, response: FloodResponse) {
        let path = &response.path_trace;

        for window in path.windows(2) {
            if let [current, next] = window {
                let entry = self.network
                    .topology
                    .entry(current.0)
                    .or_insert_with(|| (self.network.graph.add_node(current.0), vec![]));

                if !entry.1.contains(&next.0) {
                    entry.1.push(next.0);
                }

                // Se il nodo è un server, salvalo
                if current.1 == NodeType::Server {
                    self.network.servers.insert(current.0);
                }
            }
        }

        // Aggiorno il grafo
        self.network.update_graph();
    }

    // fn process_flood_response(&mut self, response: FloodResponse) {
    //     let path = &response.path_trace;
    //
    //     for window in path.windows(2) {
    //         if let [current, next] = window {
    //             self.network
    //                 .topology
    //                 .entry(current.0)
    //                 .or_insert_with(|| (self.network.graph.add_node(current.0), vec![]))
    //                 .1
    //                 .push(next.0);
    //         }
    //     }
    //
    //     // aggiorno il grafo
    //     self.network.update_graph();
    // }
    fn send_packet(&self, packet: Packet, channel: Option<&Sender<Packet>>) -> SendingCodes {
        self.log("Sending packet...");
        let result = match channel.or_else(|| {
            self.packet_send
                .get(&packet.routing_header.hops[packet.routing_header.hop_index])
        })
        {
            Some(ch) => match ch.send(packet.clone()) {
                Ok(_) => {
                    if let Err(err) = self.sendto_controller(packet.clone(), false) {
                        self.log(&format!("Error notifying controller: {}", err));
                    }
                    SendingCodes::SuccessfullySent(packet.session_id)
                }
                Err(err) => {
                    if let Err(err) = self.sendto_controller(packet, false) {
                        self.log(&format!("Error notifying controller: {}", err));
                    }
                    SendingCodes::ErrorSending(err.to_string())
                }
            },
            None => {
                self.log("No valid channel found for sending packet.");
                if let Err(err) = self.sendto_controller(packet, true) {
                    self.log(&format!("Error notifying controller: {}", err));
                }
                SendingCodes::NoNextHop("Neighbour not found".to_string())
            }
        };

        self.log(&format!("{:?}", result));
        result
    }
    fn sendto_controller(&self, packet: Packet, sent: bool) -> Result<(),String>{
        self.log("Sending to controller...");
        match self.controller_send.send(PacketSent(packet)){
            Ok(_) => { self.log("Successfully sent to controller."); Ok(()) },
            Err(er) => Err(er.to_string())
        }
    }
    fn handle_ack(&self, ack: Ack, session_id: u64) {
        self.log(&format!(
            "Host {}: Ack ricevuto per il frammento {} (sessione {}).",
            self.id, ack.fragment_index, session_id
        ));

        let mut sent_fragments = self.sent_fragments.lock().unwrap();

        // Controlla se la sessione esiste
        if let Some(session_data) = sent_fragments.get_mut(&session_id) {
            let buffer = &mut session_data.buffer;

            // Controlla se il frammento specificato esiste ed è ancora in sospeso
            if let Some(Some(_)) = buffer.get(ack.fragment_index as usize) {
                buffer[ack.fragment_index as usize] = None; // Marca il frammento come confermato
                session_data.last_activity = Instant::now(); // Aggiorna l'ultima attività della sessione

                self.log(format!(
                    "Host {}: Frammento {} confermato per la sessione {}.",
                    self.id, ack.fragment_index, session_id
                ));
            } else {
                self.log(format!(
                    "Host {}: Frammento {} già confermato o non trovato nella sessione {}.",
                    self.id, ack.fragment_index, session_id
                ));
            }
        } else {
            self.log(format!(
                "Host {}: Nessuna sessione trovata per l'ID {}.",
                self.id, session_id
            ));
        }
    }
    fn handle_nack(&self, nack: Nack, session_id: u64) {
        self.log(&format!(
            "Host {}: Received Nack for fragment index {} (session ID: {}). Reason: {:?}",
            self.id, nack.fragment_index, session_id, nack.nack_type
        ));

        // Blocca la mappa dei frammenti inviati
        let mut sent_fragments = self.sent_fragments.lock().unwrap();

        if let Some(session) = sent_fragments.get_mut(&session_id) {
            // Verifica se il frammento esiste
            if let Some(Some(fragment)) = session.buffer.get(nack.fragment_index as usize) {
                match nack.nack_type {
                    NackType::Dropped => {
                        self.log(&format!("Host {}: Retrying fragment due to drop.", self.id));
                        self.send_fragment(session_id, fragment.clone());
                    }
                    NackType::ErrorInRouting(node_id) => {
                        self.log(&format!(
                            "Host {}: Routing error encountered. Node ID: {}",
                            self.id, node_id
                        ));
                        // Cerca un percorso alternativo
                        if let Some(new_route) = self.network.find_alternate_route(self.id, session.destination,session.route.as_slice()){
                            self.log(format!(
                                "Percorso alternativo trovato. Ritrasmissione del frammento {}...",
                                nack.fragment_index
                            ));

                            // Crea un nuovo header di routing
                            let routing_header = SourceRoutingHeader::new(new_route, 0);

                            // Crea e invia il nuovo pacchetto
                            let packet = Packet::new_fragment(routing_header, session_id, fragment.clone());
                            self.send_packet(packet, None);
                        } else {
                            self.log(format!(
                                "Impossibile trovare un percorso alternativo per la sessione {}.",
                                session_id
                            ));
                        }
                    }
                    NackType::DestinationIsDrone => {
                        self.log(&format!(
                            "Host {}: Destination is drone.", self.id
                        ));
                        // Adotta azioni per correggere il percorso, se possibile
                    }
                    NackType::UnexpectedRecipient(node_id) => {
                        self.log(&format!(
                            "Host {}: Destination is drone.", self.id
                        ));
                        // non ho capito questo tipo di Nack a cosa serve
                    }
                }
            }
        }
    }
    fn handle_msg_fragment(&mut self, packet: Packet, fragment: Fragment) -> Option<Vec<u8>> {
        // Ottieni o inizializza il buffer di frammenti per questa sessione
        let mut received_fragments = self.received_fragments.lock().unwrap();

        let session_data = received_fragments
            .entry(packet.session_id.clone())
            .or_insert_with(|| RecSessionData {
                buffer: vec![None; fragment.total_n_fragments as usize],
                last_activity: Instant::now(),
            });

        let buffer = &mut session_data.buffer;
        session_data.last_activity = Instant::now(); // Aggiorna il timestamp

        let index = fragment.fragment_index as usize;

        // Verifica l'indice del frammento
        if index >= buffer.len() {
            self.log(format!(
                "Invalid fragment index: {} (session ID: {})",
                fragment.fragment_index, packet.session_id
            ));
            return None;
        }

        // Controlla se il frammento è duplicato
        if buffer[index].is_some() {
            self.log(format!(
                "Duplicate fragment received: {} (session ID: {})",
                fragment.fragment_index, packet.session_id
            ));
            return None;
        }

        // Memorizza il frammento
        buffer[index] = Some(fragment.clone());

        // Invia un ACK per il frammento ricevuto
        let ack_packet = self.build_packet_ack(packet.clone(), Some(fragment.fragment_index));
        self.send_packet(ack_packet, None);

        // Verifica se tutti i frammenti sono stati ricevuti
        if buffer.iter().all(|f| f.is_some()) {
            self.log(format!(
                "All fragments received for session ID: {}",
                packet.session_id
            ));

            // Ricostruisce il messaggio
            let content = buffer
                .iter()
                .filter_map(|f| f.as_ref())
                .flat_map(|frag| frag.data[..frag.length as usize].to_vec())
                .collect::<Vec<u8>>();

            // Rimuove il buffer per questa sessione
            received_fragments.remove(&packet.session_id);
            return Some(content)
        }

        None
    }
    fn cleanup_expired_sessions(&self, timeout: Duration) {
        self.log("Cleaning up expired sessions...");
        let now = Instant::now();

        // Pulizia delle sessioni ricevute
        {
            let mut received_fragments = self.received_fragments.lock().unwrap();
            received_fragments.retain(|session_id, session_data| {
                if now.duration_since(session_data.last_activity) > timeout {
                    self.log(format!(
                        "Session ID {} rimossa da received_fragments per inattività.",
                        session_id
                    ));
                    false // Rimuove la sessione
                } else {
                    true // Mantiene la sessione
                }
            });
        }

        // Pulizia delle sessioni inviate
        {
            let mut sent_fragments = self.sent_fragments.lock().unwrap();
            sent_fragments.retain(|session_id, session_data| {
                let session_expired = now.duration_since(session_data.last_activity) > timeout;
                let all_fragments_confirmed = session_data.buffer.iter().all(|frag| frag.is_none());

                if session_expired || all_fragments_confirmed {
                    if session_expired {
                        self.log(format!(
                            "Session ID {} rimossa da sent_fragments per inattività.",
                            session_id
                        ));
                    } else {
                        self.log(format!(
                            "Session ID {} rimossa da sent_fragments (tutti i frammenti confermati).",
                            session_id
                        ));
                    }
                    false // Rimuovi la sessione
                } else {
                    true // Mantieni la sessione
                }
            });
        }

        self.log("Pulizia delle sessioni completata.");
    }
    fn start_session_cleanup(self: Arc<Self>, timeout: Duration, interval: Duration) {
        let host_ref = Arc::clone(&self);

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            loop {
                interval_timer.tick().await;
                host_ref.cleanup_expired_sessions(timeout);
            }
        });
    }
    fn get_fragment(&self, session_id: u64, fragment_index: u64) -> Option<Fragment> {
        let sent_fragments = self.sent_fragments.lock().unwrap();
        sent_fragments
            .get(&session_id)
            .and_then(|session_data| session_data.buffer.get(fragment_index as usize).cloned().flatten())
    }
    fn send_fragment(&self, session_id: u64, fragment: Fragment) {
        let mut sent_fragments = self.sent_fragments.lock().unwrap();

        if let Some(session_data) = sent_fragments.get(&session_id) {
            let routing_header = SourceRoutingHeader::new(session_data.route.clone(), 0);

            let packet = Packet::new_fragment(routing_header, session_id, fragment);

            self.send_packet(packet, None);
        } else {
            self.log(format!(
                "Session ID {} non trovata durante l'invio del frammento.",
                session_id
            ));
        }
    }
    fn start_retransmission_task(self: Arc<Self>, retransmission_interval: Duration) {
        let sent_fragments = Arc::clone(&self.sent_fragments);

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(retransmission_interval);

            loop {
                interval_timer.tick().await;

                let now = Instant::now();
                let mut fragments = sent_fragments.lock().unwrap();

                for (session_id, session) in fragments.iter_mut() {
                    for (i, fragment) in session.buffer.iter_mut().enumerate() {
                        if let Some(fragment) = fragment {
                            if now.duration_since(session.last_activity) > retransmission_interval {
                                println!(
                                    "Session {}: Ritrasmissione del frammento {}",
                                    session_id, i
                                );
                                self.send_fragment(*session_id, fragment.clone());
                            }
                        }
                    }
                }
            }
        });
    }
    fn build_packet_ack(&self, packet: Packet, fragment_index: Option<u64>) -> Packet{
        let mut routing_header = packet.routing_header;
        routing_header.reverse();
        Packet::new_ack(routing_header, packet.session_id, fragment_index.unwrap_or(0))
    }
    fn start_tasks(self: Arc<Self>) {
        let cleanup_timeout = Duration::from_secs(300);
        let cleanup_interval = Duration::from_secs(60);
        let retransmission_interval = Duration::from_millis(200);

        self.clone().start_session_cleanup(cleanup_timeout, cleanup_interval);
        self.clone().start_retransmission_task(retransmission_interval);
    }
}


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
pub enum ServerToClient{
    ServerType(ServerTypes),

    FilesList(Vec<FileId>),
    File,

    MediaList(Vec<MediaId>),
    Media,

    ClientList(Vec<ClientId>),
    MessageFrom(ClientId, String),

    ErrNotFound,
    ErrClientId
}
#[derive(Hash, Eq, PartialEq)]
pub enum ServerTypes{
    Chat,
    Content
}
///MESSAGES WG_2024
#[derive(Clone)]
pub struct Message {
    pub session_id: u64,
    pub content: Vec<u8>,
}
pub struct Image{
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}
// pub trait DroneSend: Serialize + DeserializeOwned {
//     fn stringify(&self) -> String {
//         serde_json::to_string(self).unwrap()
//     }
//     fn from_string(raw: String) -> Result<Self, String> {
//         serde_json::from_str(raw.as_str()).map_err(|e| e.to_string())
//     }
// }
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
// pub trait Request: DroneSend {}
// pub trait Response: DroneSend {}
