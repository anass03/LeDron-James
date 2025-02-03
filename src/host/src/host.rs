use crossbeam_channel::{Receiver, Sender};
use wg_2024::controller::{DroneEvent};
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Packet, FloodRequest, FloodResponse, Ack, Nack, NackType, NodeType, Fragment, FRAGMENT_DSIZE};
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use petgraph::algo::dijkstra;
use petgraph::graph::{DiGraph, NodeIndex};
use rand::Rng;
use rand::rngs::ThreadRng;
use serde_json::to_string;
use wg_2024::controller::DroneEvent::{PacketSent};
pub type FileId = usize;
pub type MediaId = usize;
pub type ClientId = NodeId;
pub type ServerId = NodeId;

#[derive(Clone)]
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
impl Host{
    pub fn new(id:NodeId,
               type_host:NodeType,
               controller_send:Sender<DroneEvent>,
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
        };
        let host_arc = Arc::new(host.clone());
        host_arc.start_tasks();
        host
    }
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
#[derive(Clone)]
pub struct Network {
    pub graph: DiGraph<NodeId, f64>,
    pub topology: HashMap<NodeId, (NodeIndex, Vec<NodeId>)>, // NodeId -> (Graph NodeIndex, Vec of neighbors)
    pub servers: HashSet<NodeId>,
    pub drones_packet_sent: HashMap<NodeId,(i32,i32)>
}
impl Network {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            topology: HashMap::new(),
            servers: HashSet::new(),
            drones_packet_sent: HashMap::new()
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
    pub fn modify_pdr(&mut self){
        for i in self.topology.keys(){
            for j in self.topology.get(i).unwrap().1.iter(){
                if let Some(a) = self.graph.find_edge(self.topology.get(i).unwrap().0, self.topology.get(j).unwrap().0) {
                    if let Some(weight) = self.graph.edge_weight_mut(a) {
                        *weight = match self.drones_packet_sent.get(i).unwrap().1 {
                            0 => 0.0,
                            _ => {self.drones_packet_sent.get(i).unwrap().1.clone() as f64 /self.drones_packet_sent.get(i).unwrap().0 as f64}
                        };
                    }
                }
            }
        }
    }
    pub fn pdr_helper(&mut self,source_routing_header: SourceRoutingHeader){
        for i in source_routing_header.hops{
            self.drones_packet_sent.entry(i).and_modify(|s| s.0 += 1).or_insert((0,0));
        }
        self.modify_pdr();
    }
    pub fn pdr_from_nack(&mut self,id:NodeId){
        self.drones_packet_sent.entry(id).and_modify(|s| s.1 += 1).or_insert((0,0));
        self.modify_pdr();
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
    fn sendto_controller(&self, packet: Packet) -> Result<(),String>;
    fn handle_ack(&self, ack: Ack, session_id: u64);
    fn handle_nack(&mut self, nack: Nack,nack_generator: NodeId, session_id: u64);
    fn build_msg_fragment(&mut self, destination: NodeId, message: Message) -> Result<(), String>;
    fn cleanup_expired_sessions(&self, timeout: Duration);
    fn start_session_cleanup(self: Arc<Self>, timeout: Duration, interval: Duration);
    fn get_fragment(&self, session_id: u64, fragment_index: u64) -> Option<Fragment>;
    fn send_fragment(&self, session_id: u64, fragment: Fragment);
    fn start_retransmission_task(self: Arc<Self>, retransmission_interval: Duration);
    fn build_packet_ack(&self,packet: Packet, fragment_index: Option<u64>) -> Packet;
    fn start_tasks(self: Arc<Self>);
}
fn generate_unique_flood_id() -> u64 {
    let mut rng: ThreadRng = rand::rng();  // Usa il generatore di numeri casuali sicuro per il thread
    rng.random::<u64>()  // Genera un numero casuale di tipo u64
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
            self.log(&format!("Initiate flood result for {:?}: {:?}", neighbour_id, result));
        }
    }
    fn process_flood_request(&mut self, flood_request: FloodRequest, packet: Packet) {
        self.log("Handling FloodRequest...");

        // Verifica se la richiesta di flood è già stata processata
        if self.flood_cache.contains_key(&flood_request.initiator_id)
            && self.flood_cache[&flood_request.initiator_id] == flood_request.flood_id
        {
            self.log("Flood request already processed, sending response...");

            let result = self.send_packet(flood_request.generate_response(packet.session_id), None);
            self.log(&format!("Flood response result: {:?}", result));
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
            let result = self.send_packet(response_packet, None);
            self.log(&format!("Flood response result: {:?}", result));
        } else {
            // Inoltra il pacchetto aggiornato ai vicini
            self.log("Forwarding FloodRequest to neighbors...");
            let updated_packet = Packet::new_flood_request(packet.routing_header,packet.session_id,updated_flood_request);

            for (neighbor_id, sender) in neighbors_to_send {
                self.log(&format!("Forwarding FloodRequest to neighbor: {:?}", neighbor_id));
                let result = self.send_packet(updated_packet.clone(), Some(sender));
                self.log(&format!("Forwarding to neighbor result: {:?}", result));
            }
        }
    }
    fn process_flood_response(&mut self, response: FloodResponse) {
        self.log("Handling FloodResponse...");

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
                    if let Err(err) = self.sendto_controller(packet.clone()) {
                        self.log(&format!("Error notifying controller: {}", err));
                    }
                    SendingCodes::SuccessfullySent(packet.session_id)
                }
                Err(err) => {
                    if let Err(err) = self.sendto_controller(packet) {
                        self.log(&format!("Error notifying controller: {}", err));
                    }
                    SendingCodes::ErrorSending(err.to_string())
                }
            },
            None => {
                self.log("No valid channel found for sending packet.");
                if let Err(err) = self.sendto_controller(packet) {
                    self.log(&format!("Error notifying controller: {}", err));
                }
                SendingCodes::NoNextHop("Neighbour not found".to_string())
            }
        };

        self.log(&format!("{:?}", result));
        result
    }
    fn sendto_controller(&self, packet: Packet) -> Result<(),String>{
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
    fn handle_nack(&mut self, nack: Nack, nack_generator: NodeId, session_id: u64) {
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

                        //updated expected pdr for drone that dropped the packet
                        self.network.pdr_from_nack(nack_generator);
                    }
                    // NackType::DestinationIsDrone => {
                    //     self.log(&format!(
                    //         "Host {}: Destination is drone.", self.id
                    //     ));
                    //     // Adotta azioni per correggere il percorso, se possibile
                    // }
                    _ => {
                        self.log(&format!(
                            "Host {}: Routing error encountered or UnexpectedRecipient!",
                            self.id
                        ));
                        // Cerca un percorso alternativo
                        if let Some(new_route) = self.network.find_alternate_route(self.id, session.destination,session.route.as_slice()){
                            self.log(format!(
                                "Founded alternative route. Retransmission of fragment {}...",
                                nack.fragment_index
                            ));

                            // Crea un nuovo header di routing
                            let routing_header = SourceRoutingHeader::new(new_route, 0);

                            // Crea e invia il nuovo pacchetto
                            let packet = Packet::new_fragment(routing_header, session_id, fragment.clone());
                            let result = self.send_packet(packet, None);
                            self.log(&format!("Retransmission result: {:?}", result));
                        } else {
                            self.log(format!(
                                "Impossibile trovare un percorso alternativo per la sessione {}.",
                                session_id
                            ));
                        }
                    }
                }
            }
        }
    }
    fn build_msg_fragment(&mut self, destination: NodeId, message: Message) -> Result<(), String> {
        self.log(&format!(
            "Starting message fragmentation: session_id={}, destination={}",
            message.session_id, destination
        ));
        // Trova il miglior percorso per la destinazione
        let route = self.network.find_best_route(self.id, destination);
        if route.is_empty() {
            let err_msg = format!("No route found to destination: {}", destination);
            self.log(&err_msg);
            return Err(err_msg);
        }

        self.log(&format!("Route found: {:?}", route));

        // Serializza il contenuto del messaggio
        let content = to_string(&message.content)
            .map_err(|e| format!("Serialization error: {}", e))?;
        let content_bytes = content.as_bytes();
        let total_n_fragments = (content_bytes.len() + FRAGMENT_DSIZE - 1) / FRAGMENT_DSIZE;

        // Aggiungi o aggiorna l'entry nella mappa dei frammenti inviati
        let mut sent_fragments = self.sent_fragments.lock().unwrap();
        let session_data = sent_fragments.entry(message.session_id).or_insert_with(|| {
            self.log(&format!(
                "Creating new session entry: session_id={}, total_fragments={}",
                message.session_id, total_n_fragments
            ));
            SentSessionData {
                buffer: vec![None; total_n_fragments],
                last_activity: Instant::now(),
                route: route.clone(),
                destination,
            }
        });

        // Aggiorna `last_activity` e assicura che il buffer sia grande abbastanza
        session_data.last_activity = Instant::now();
        if session_data.buffer.len() < total_n_fragments {
            session_data.buffer.resize(total_n_fragments, None);
        }

        self.log(&format!(
            "Message will be fragmented into {} parts.",
            total_n_fragments
        ));

        // Invia i frammenti
        for (i, chunk) in content_bytes.chunks(FRAGMENT_DSIZE).enumerate() {
            let fragment = Fragment {
                fragment_index: i as u64,
                total_n_fragments: total_n_fragments as u64,
                length: chunk.len() as u8,
                data: {
                    let mut data = [0; FRAGMENT_DSIZE];
                    data[..chunk.len()].copy_from_slice(chunk);
                    data
                },
            };

            // Aggiungi il frammento al buffer
            session_data.buffer[i] = Some(fragment.clone());

            // Costruisci e invia il pacchetto
            let routing_header = SourceRoutingHeader::new(route.clone(), 0);
            let packet = Packet::new_fragment(routing_header.clone(), message.session_id, fragment);

            self.log(&format!(
                "Sending fragment {}/{} for session_id={} via route {:?}",
                i + 1,
                total_n_fragments,
                message.session_id,
                route
            ));

            let result= self.send_packet(packet, None);
            self.log(&format!("Sending fragment result: {}", result));


            let pdr_result = self.network.pdr_helper(routing_header);
            self.log(&format!(
                "PDR helper updated after fragment {}: {:?}",
                i, pdr_result
            ));
        }

        self.log(&format!(
            "Message fragmentation completed for session_id={}",
            message.session_id
        ));

        Ok(())
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
        let sent_fragments = self.sent_fragments.lock().unwrap();
        self.log(&format!(
            "Attempting to send fragment {} for session {}",
            fragment.fragment_index, session_id
        ));
        if let Some(session_data) = sent_fragments.get(&session_id) {
            let routing_header = SourceRoutingHeader::new(session_data.route.clone(), 0);

            let packet = Packet::new_fragment(routing_header, session_id, fragment);

            let result = self.send_packet(packet, None);
            self.log(&format!("Sending fragment result: {:?}", result));

        } else {
            self.log(format!(
                "Session ID {} non trovata durante l'invio del frammento.",
                session_id
            ));
        }
    }
    fn start_retransmission_task(self: Arc<Self>, retransmission_interval: Duration) {
        let sent_fragments = Arc::clone(&self.sent_fragments);

        self.log(&format!(
            "Starting retransmission task with interval {}ms",
            retransmission_interval.as_millis()
        ));

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(retransmission_interval);
            let mut retry_count = 0;

            loop {
                interval_timer.tick().await;
                retry_count += 1;

                self.log(&format!(
                    "Retransmission cycle #{} started",
                    retry_count
                ));
                let now = Instant::now();
                let mut fragments = sent_fragments.lock().unwrap();

                for (session_id, session) in fragments.iter_mut() {
                    let fragments_count = session.buffer.iter().filter(|f| f.is_some()).count();
                    self.log(&format!(
                        "Checking session {} ({} pending fragments)",
                        session_id, fragments_count
                    ));

                    for (i, fragment) in session.buffer.iter_mut().enumerate() {
                        if let Some(fragment) = fragment {
                            let elapsed = now.duration_since(session.last_activity);
                            if elapsed > retransmission_interval {
                                self.log(&format!(
                                    "Retransmitting fragment {} for session {} (elapsed: {:?})",
                                    i, session_id, elapsed
                                ));
                                self.send_fragment(*session_id, fragment.clone());
                                session.last_activity = Instant::now();
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

        self.log("Starting background tasks...");
        self.log(&format!(
            "Cleanup parameters: timeout={}s, interval={}s",
            cleanup_timeout.as_secs(),
            cleanup_interval.as_secs()
        ));
        self.log(&format!(
            "Retransmission interval: {}ms",
            retransmission_interval.as_millis()
        ));


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