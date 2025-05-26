use std::cmp::Reverse;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Packet, FloodRequest, FloodResponse, Ack, Nack, NackType, NodeType, Fragment, FRAGMENT_DSIZE, PacketType};
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use petgraph::algo::dijkstra;
use petgraph::graph::{DiGraph, NodeIndex};
use tracing::{info, warn, error, debug};
use rand::Rng;
use rand::rngs::ThreadRng;
use serde::{Deserialize, Serialize};
use ordered_float::OrderedFloat;


pub type FileId = usize;
pub type MediaId = usize;
pub type ClientId = NodeId;
pub type ServerId = NodeId;

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
        };
        host
    }
}

pub struct RecSessionData {
    pub buffer: Vec<Option<Fragment>>,
    pub last_activity: Instant, // Timestamp dell'ultima attività
    pub pending: usize,  // quanti frammenti ancora da ricevere
}
pub struct SentSessionData {
    pub buffer: Vec<Option<Fragment>>, // Buffer dei frammenti inviati
    pub last_activity: Instant,        // Timestamp dell'ultima attività
    pub destination: NodeId,        // Destinatario della sessione
    pub route: Vec<NodeId>,        // percorso pre-calcolato
    pub pending: usize,  // quanti frammenti ancora in attesa di ACK
}
#[derive(Clone)]
pub struct Edge {
    pub to:     NodeId,
    weight: f64,
}
#[derive(Clone)]
pub struct Network {
    // pub graph: DiGraph<NodeId, f64>,
    // pub topology: HashMap<NodeId, (NodeIndex, Vec<NodeId>)>, // NodeId -> (Graph NodeIndex, Vec of neighbors)
    pub servers: HashSet<NodeId>,
    /// Mappa nodo → (tipo, lista archi)
    pub nodes: HashMap<NodeId, (NodeType, Vec<Edge>)>,

    /// Per ogni drone: (pacchetti inviati, pacchetti persi)
    pub drones_packet_stats: HashMap<NodeId, (u32, u32)>,
}
impl Network {
    pub fn new() -> Self {
        Self {
            // graph: DiGraph::new(),
            // topology: HashMap::new(),
            servers: HashSet::new(),
            // drones_packet_sent: HashMap::new()
            nodes: HashMap::new(),
            drones_packet_stats: HashMap::new(),
        }
    }
    /// Chiama questa funzione **dopo** aver inserito/aggiornato
    /// i contatori in `drones_packet_stats` per ricalcolare tutti
    /// i PDR sugli archi uscenti da ciascun drone.
    pub fn recalc_pdr(&mut self) {
        for (&drone_id, &(sent, lost)) in &self.drones_packet_stats {
            // Evitiamo divisioni per zero
            let pdr = if sent == 0 {
                0.0
            } else {
                // PDR = (sent - lost) / sent
                (sent.saturating_sub(lost)) as f64 / sent as f64
            };

            // Aggiorna ogni arco uscente dal drone
            if let Some((node_type, edges)) = self.nodes.get_mut(&drone_id) {
                // Solo se è effettivamente un drone
                if *node_type == NodeType::Drone {
                    for edge in edges.iter_mut() {
                        edge.weight = pdr;
                    }
                }
            } else {
                log::warn!("Drone node {} non trovato in `nodes`!", drone_id);
            }
        }
    }

    /// Registra un tentativo di forwarding su ciascun hop
    /// (ad esempio quando mandi un pacchetto a percorso sorgente).
    pub fn pdr_helper(&mut self, hops: &[NodeId]) {
        // Ogni volta che "provi" a inoltrare via hop, incrementa sent
        for &node in hops {
            self.drones_packet_stats
                .entry(node)
                .and_modify(|stats| stats.0 += 1)
                .or_insert((1, 0));
        }
        self.recalc_pdr();
    }

    /// Registra un NACK (drop) ricevuto dal drone `node_id`
    pub fn pdr_from_nack(&mut self, node_id: NodeId) {
        self.drones_packet_stats
            .entry(node_id)
            .and_modify(|stats| stats.1 += 1)
            .or_insert((0, 1));
        self.recalc_pdr();
    }




    /// Restituisce Some(path) che massimizza la probabilità di successo (PDR),
    /// ovvero minimizza la somma dei costi c = -ln(pdr) lungo gli archi.
    pub fn find_best_route(
        &self,
        start: NodeId,
        destination: NodeId,
    ) -> Option<Vec<NodeId>> {
        let mut dist: HashMap<NodeId, f64> = HashMap::new();
        let mut prev: HashMap<NodeId, NodeId> = HashMap::new();
        let mut pq: BinaryHeap<Reverse<(OrderedFloat<f64>, NodeId)>> = BinaryHeap::new();

        dist.insert(start, 0.0);
        pq.push(Reverse((OrderedFloat(0.0), start)));

        while let Some(Reverse((OrderedFloat(cost_u), u))) = pq.pop() {
            // se troviamo un costo più grande di quello già noto, skip
            if cost_u > dist.get(&u).copied().unwrap_or(f64::INFINITY) {
                continue;
            }
            if u == destination {
                break;
            }
            if let Some((_, edges)) = self.nodes.get(&u) {
                for edge in edges {
                    let v = edge.to;
                    let reliability = 1.0 - edge.weight;
                    // se affidabilità nulla, ignoro l'arco
                    if reliability <= 0.0 {
                        continue;
                    }
                    // costo additivo
                    let c = -reliability.ln();
                    let next_cost = cost_u + c;
                    if dist.get(&v).map_or(true, |&old| next_cost < old) {
                        dist.insert(v, next_cost);
                        prev.insert(v, u);
                        pq.push(Reverse((OrderedFloat(next_cost), v)));
                    }
                }
            }
        }

        if !dist.contains_key(&destination) {
            return None;
        }
        // ricostruzione del percorso
        let mut path = Vec::new();
        let mut cur = destination;
        while cur != start {
            path.push(cur);
            cur = prev[&cur];
        }
        path.push(start);
        path.reverse();
        Some(path)
    }

    /// Come `find_best_route`, ma esclude tutti gli archi consecutivi già
    /// presenti in `used_route`.
    pub fn find_alternate_route(
        &self,
        start: NodeId,
        destination: NodeId,
        used_route: &[NodeId],
    ) -> Option<Vec<NodeId>> {
        // clono la topologia e rimuovo gli archi usati
        let mut temp = self.nodes.clone();
        for window in used_route.windows(2) {
            if let [u, v] = window {
                if let Some((_, edges)) = temp.get_mut(u) {
                    edges.retain(|e| e.to != *v);
                }
                if let Some((_, edges)) = temp.get_mut(v) {
                    edges.retain(|e| e.to != *u);
                }
            }
        }

        let mut dist: HashMap<NodeId, f64> = HashMap::new();
        let mut prev: HashMap<NodeId, NodeId> = HashMap::new();
        let mut pq: BinaryHeap<Reverse<(OrderedFloat<f64>, NodeId)>> = BinaryHeap::new();

        dist.insert(start, 0.0);
        pq.push(Reverse((OrderedFloat(0.0), start)));

        while let Some(Reverse((OrderedFloat(cost_u), u))) = pq.pop() {
            if cost_u > dist.get(&u).copied().unwrap_or(f64::INFINITY) {
                continue;
            }
            if u == destination {
                break;
            }
            if let Some((_, edges)) = temp.get(&u) {
                for edge in edges {
                    let v = edge.to;
                    let reliability = 1.0 - edge.weight;
                    if reliability <= 0.0 {
                        continue;
                    }
                    let c = -reliability.ln();
                    let next_cost = cost_u + c;
                    if dist.get(&v).map_or(true, |&old| next_cost < old) {
                        dist.insert(v, next_cost);
                        prev.insert(v, u);
                        pq.push(Reverse((OrderedFloat(next_cost), v)));
                    }
                }
            }
        }

        if !dist.contains_key(&destination) {
            return None;
        }
        let mut path = Vec::new();
        let mut cur = destination;
        while cur != start {
            path.push(cur);
            cur = prev[&cur];
        }
        path.push(start);
        path.reverse();
        Some(path)
    }
}
pub trait Functionality{
    fn initiate_flood(&mut self);
    fn process_flood_request(&mut self, flood_request: FloodRequest,packet: Packet);
    fn process_flood_response(&mut self, response: FloodResponse);
    fn send_packet(&self, packet: Packet, channel: Option<&Sender<Packet>>) -> SendingCodes;
    fn sendto_controller(&self, packet: Packet) -> Result<(),String>;
    fn handle_ack(&self, ack: Ack, session_id: u64);
    fn handle_nack(&mut self, nack: Nack,nack_generator: NodeId, session_id: u64);
    fn build_msg_fragment(&mut self, destination: NodeId, message: Message) -> Result<(), String>;
    fn cleanup_expired_sessions(&self, timeout: Duration);
    fn get_fragment(&self, session_id: u64, fragment_index: u64) -> Option<Fragment>;
    fn send_fragment(&self, session_id: u64, fragment: Fragment);
    fn build_packet_ack(&self,packet: Packet, fragment_index: Option<u64>) -> Packet;
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
    fn initiate_flood(&mut self) {
        // Genero subito gli ID, così li posso loggare
        let session_id = generate_unique_session_id();
        let flood_id = generate_unique_flood_id();
        let host_id = self.id;

        self.flood_cache.insert(host_id.clone(), vec![flood_id.clone()]);

        // Costruisco header e packet (attenzione: hop_index iniziale = 0)
        let routing_header = SourceRoutingHeader::new(vec![], 0); //rimosso host_id dal vec
        let flood_request = Packet::new_flood_request(
            routing_header,
            session_id,
            FloodRequest::initialize(flood_id, host_id, self.type_host),
        );

        tracing::info!(host = host_id,flood_id,session_id,"Starting flood discovery");

        if self.packet_send.is_empty() {
            tracing::warn!(host = host_id, "No neighbors to flood to");
            return;
        }

        // Inoltro la richiesta a tutti i vicini
        for (neighbour_id, sender) in &self.packet_send {
            tracing::debug!(
                host = host_id,
                neighbour = neighbour_id,
                "Sending FloodRequest"
            );
            let result = self.send_packet(flood_request.clone(), Some(sender));
            tracing::info!(
                host = host_id,
                neighbour = neighbour_id,
                ?result,
                "FloodRequest send result: "
            );
        }
    }
    fn process_flood_request(&mut self, flood_request: FloodRequest, packet: Packet) {
        let host_id = self.id;
        let initiator = flood_request.initiator_id;
        let flood_id = flood_request.flood_id;
        let session_id = packet.session_id;

        tracing::info!(
            host = host_id,
            initiator,
            flood_id,
            session = session_id,
            "Handling FloodRequest"
        );

        let mut fr = flood_request.clone();

        // 1) Controllo se ho già visto questo flood_id dall'initiator
        let seen_flood_ids = self.flood_cache.entry(initiator).or_default();

        if seen_flood_ids.contains(&flood_id) {
            tracing::info!("Path trace before adding myself: {:?}", fr.path_trace);

            fr.increment(host_id, self.type_host);

            tracing::debug!(
                host = host_id,
                initiator,
                flood_id,
                "FloodRequest already seen, sending response immediately"
            );

            tracing::info!("Path trace after adding myself: {:?}", fr.path_trace);
            let mut response_pkt = fr.generate_response(session_id);
            response_pkt.routing_header.hop_index += 1;

            let next_hop = response_pkt.routing_header.hops[response_pkt.routing_header.hop_index];
            tracing::debug!(
                host = host_id,
                ?next_hop,
                neighbors = ?self.packet_send.keys().collect::<Vec<_>>(),
                "Attempting to send FloodResponse to next hop"
            );

            let result = self.send_packet(response_pkt, None);

            tracing::info!(
                host = host_id,
                ?result,
                "Sent FloodResponse for duplicate FloodRequest"
            );

            return;
        }

        // 2) Registriamo il flood_id come visto
        seen_flood_ids.push(flood_id);

        // 3) Verifico chi ha inviato il pacchetto
        let from = match flood_request.path_trace.last() {
            Some(&(node_id, _)) => node_id,
            None => {
                warn!("Received FloodRequest with empty path-trace! Dropping packet.");
                return;
            }
        };

        tracing::debug!(
            host = host_id,
            from,
            "FloodRequest arrived from neighbor"
        );
        tracing::info!("Path trace before adding myself: {:?}", fr.path_trace);
        // 4) Aggiorno path_trace
        fr.increment(host_id, self.type_host);
        tracing::info!("Path trace after adding myself: {:?}", fr.path_trace);
        // 5) Inoltro a tutti i vicini (escluso chi me l'ha mandato)
        let neighbors: Vec<_> = self
            .packet_send
            .iter()
            .filter(|(&nid, _)| nid != from)
            .collect();

        if neighbors.is_empty() {
            // 6a) Fine linea: genero immediatamente FloodResponse
            tracing::info!(host = host_id, "No other neighbors: replying flood immediately");
            let mut resp_pkt = fr.generate_response(session_id);
            resp_pkt.routing_header.hop_index+=1;
            let next_hop = resp_pkt.routing_header.hops[resp_pkt.routing_header.hop_index];
            tracing::debug!(
                host = host_id,
                ?next_hop,
                neighbors = ?self.packet_send.keys().collect::<Vec<_>>(),
                "Attempting to send FloodResponse to next hop"
            );

            let result = self.send_packet(resp_pkt, None);

            tracing::info!(
            host = host_id,
            ?result,
            "FloodResponse sent from leaf node"
        );
        } else {
            // 6b) Inoltro ai vicini disponibili
            tracing::info!(
                host = host_id,
                count = neighbors.len(),
                "Forwarding FloodRequest to neighbors"
            );

            for (&nbr, ch) in neighbors {
                tracing::debug!(host = host_id, neighbor = nbr, "Forwarding to neighbor");
                let fwd_pkt = Packet::new_flood_request(
                    packet.routing_header.clone(),
                    session_id,
                    fr.clone(),
                );
                let result = self.send_packet(fwd_pkt, Some(ch));

                tracing::debug!(
                    host = host_id,
                    neighbor = nbr,
                    ?result,
                    "ForwardFloodRequest result"
                );
            }
        }
    }

    // fn process_flood_response(&mut self, response: FloodResponse) {
    //     let host_id = self.id;
    //
    //     // 🔍 LOG: path completo in arrivo
    //     let path_str = response
    //         .path_trace
    //         .iter()
    //         .map(|(id, ty)| format!("{:?}({})", ty, id))
    //         .collect::<Vec<_>>()
    //         .join(" -> ");
    //
    //     tracing::info!(
    //         host = host_id,
    //         flood_id = response.flood_id,
    //         path = %path_str,
    //         "Handling FloodResponse with path"
    //     );
    //
    //     let path = &response.path_trace;
    //     if path.len() < 2 {
    //         tracing::warn!(
    //             host = host_id,
    //             len = path.len(),
    //             "FloodResponse path too short, skipping"
    //         );
    //         return;
    //     }
    //
    //     // 1) Assicura che tutti i nodi nel percorso siano aggiunti alla topologia
    //     for &(node_id, node_type) in path {
    //         self.network.topology.entry(node_id).or_insert_with(|| {
    //             let idx = self.network.graph.add_node(node_id);
    //             (idx, Vec::new())
    //         });
    //
    //         if node_type == NodeType::Server {
    //             self.network.servers.insert(node_id);
    //             tracing::debug!(host = host_id, server = node_id, "Marked node as server");
    //         }
    //     }
    //
    //     // 2) Registra connessioni tra nodi adiacenti
    //     for window in path.windows(2) {
    //         let (curr_id, _) = window[0];
    //         let (next_id, _) = window[1];
    //
    //         let entry = self.network.topology.get_mut(&curr_id).unwrap(); //unwrap() è sicuro qui perché tutti i nodi della path sono stati già inseriti nel blocco precedente
    //         if !entry.1.contains(&next_id) {
    //             entry.1.push(next_id);
    //             tracing::debug!(
    //                 host = host_id,
    //                 from = curr_id,
    //                 to = next_id,
    //                 "Recorded topology edge"
    //             );
    //         }
    //     }
    //
    //     // 3) Aggiorna la rappresentazione del grafo
    //     self.network.update_graph();
    //
    //     tracing::info!(
    //         host = host_id,
    //         nodes = self.network.topology.len(),
    //         "Topology graph updated"
    //     );
    //
    //     // Debug opzionale per nodo specifico
    //     // if host_id == 6 {
    //     //     tracing::debug!("🔍 Topology at node 6: {:?}", self.network.topology.keys());
    //     //     tracing::debug!("🔍 Servers known to node 6: {:?}", self.network.servers);
    //     //     // if let Some(route) = self.network.find_best_route(6, 7) {
    //     //     //     tracing::debug!("✅ Route from 6 to 7 found: {:?}", route);
    //     //     // } else {
    //     //     //     tracing::debug!("❌ No route yet from 6 to 7");
    //     //     // }
    //     // }else {
    //     //     tracing::debug!("🔍 Topology at node 7: {:?}", self.network.topology.keys());
    //     //     tracing::debug!("🔍 Servers known to node 7: {:?}", self.network.servers);
    //     //     // if let Some(route) = self.network.find_best_route(7, 6) {
    //     //     //     tracing::debug!("✅ Route from 7 to 6 found: {:?}", route);
    //     //     // } else {
    //     //     //     tracing::debug!("❌ No route yet from 7 to 6");
    //     //     // }
    //     // }
    // }

    fn process_flood_response(&mut self, response: FloodResponse) {
        let host_id = self.id;

        // 🔍 LOG: path completo in arrivo
        let path_str = response
            .path_trace
            .iter()
            .map(|(id, ty)| format!("{:?}({})", ty, id))
            .collect::<Vec<_>>()
            .join(" -> ");

        tracing::info!(
            host = host_id,
            flood_id = response.flood_id,
            path = %path_str,
            "Handling FloodResponse with path"
        );

        let path = &response.path_trace;
        if path.len() < 2 {
            tracing::warn!(
                host = host_id,
                len = path.len(),
                "FloodResponse path too short, skipping"
            );
            return;
        }

        // 1) Assicura che tutti i nodi nel percorso siano nella adjacency-list
        for &(node_id, node_type) in path {
            // entry nuovo nodo, se non esiste
            self.network
                .nodes
                .entry(node_id)
                .or_insert_with(|| (node_type, Vec::new()));

            // per i droni, inizializza subito i contatori (0 inviati, 0 persi)
            if node_type == NodeType::Drone {
                self.network
                    .drones_packet_stats
                    .entry(node_id)
                    .or_insert((0, 0));
            }
            // per i server, inserisci nell’HashSet dei server
            if node_type == NodeType::Server {
                self.network
                    .servers
                    .insert(node_id);
            }
        }

        // 2) Registra connessioni tra nodi adiacenti
        for window in path.windows(2) {
            let (curr_id, _) = window[0];
            let (next_id, _) = window[1];

            if let Some((_, edges)) = self.network.nodes.get_mut(&curr_id) {
                // evita doppioni
                if !edges.iter().any(|e| e.to == next_id) {
                    edges.push(Edge { to: next_id, weight: 0.0 });
                    tracing::debug!(
                        host = host_id,
                        from = curr_id,
                        to = next_id,
                        "Recorded adjacency edge"
                    );
                }
            } else {
                tracing::warn!(
                    host = host_id,
                    node = curr_id,
                    "Current node missing from network.nodes"
                );
            }
        }

        // (Opzionale) se vuoi ricalcolare subito i PDR:
        // self.network.recalc_pdr();
    }
    fn send_packet(&self, packet: Packet, channel: Option<&Sender<Packet>>) -> SendingCodes {
        info!("Sending packet...");

        let next_channel = channel.or_else(|| {
            self.packet_send
                .get(&packet.routing_header.hops[packet.routing_header.hop_index])
        });

        let result = match next_channel {
            Some(ch) => match ch.send(packet.clone()) {
                Ok(_) => {
                    if let Err(err) = self.sendto_controller(packet.clone()) {
                        warn!("Notifying controller gone wrong: {}", err);
                    }
                    SendingCodes::SuccessfullySent(packet.session_id)
                }
                Err(err) => {
                    if let Err(e) = self.sendto_controller(packet) {
                        warn!("Notifying controller, for error sending packet, gone wrong: {}", e);
                    }
                    SendingCodes::ErrorSending(err.to_string())
                }
            },
            None => {
                warn!("No valid channel found for sending packet.");
                if let Err(e) = self.sendto_controller(packet) {
                    warn!("Error, notifying controller: {}", e);
                }
                SendingCodes::NoNextHop("Neighbour not found".to_string())
            }
        };

        debug!("{:?}", result);
        result
    }
    fn sendto_controller(&self, packet: Packet) -> Result<(), String> {
        info!("Sending to controller...");

        match self.controller_send.send(HostEvent::PacketSent(packet)) {
            Ok(_) => {
                info!("Successfully sent to controller.");
                Ok(())
            }
            Err(er) => {
                warn!("Failed to send to controller: {}", er);
                Err(er.to_string())
            }
        }
    }
    fn handle_ack(&self, ack: Ack, session_id: u64) {
        let idx = ack.fragment_index as usize;
        tracing::info!(
        host = self.id,
        session = session_id,
        fragment = ack.fragment_index,
        "Received ACK"
    );

        let mut sent = self.sent_fragments.lock().unwrap();
        if let Some(session) = sent.get_mut(&session_id) {
            if idx < session.buffer.len() && session.buffer[idx].is_some() {
                session.buffer[idx] = None;
                session.pending = session.pending.saturating_sub(1);
                session.last_activity = Instant::now();
                tracing::debug!(
                host = self.id,
                session = session_id,
                pending = session.pending,
                "ACK processed, pending--"
            );
            } else {
                tracing::debug!(
                host = self.id,
                session = session_id,
                fragment = ack.fragment_index,
                "ACK for nonexistent or already confirmed fragment"
            );
            }
        } else {
            tracing::warn!(
            host = self.id,
            session = session_id,
            "ACK for unknown session"
        );
        }
    }
    fn handle_nack(&mut self, nack: Nack, nack_generator: NodeId, session_id: u64) {
        let host = self.id;
        let frag_idx = nack.fragment_index;
        tracing::warn!(
            host = host,
            session = session_id,
            fragment = frag_idx,
            reason = ?nack.nack_type,
            "Received NACK"
        );

        let mut sent = self.sent_fragments.lock().unwrap();
        if let Some(session) = sent.get_mut(&session_id) {
            match session.buffer.get(frag_idx as usize) {
                Some(Some(fragment)) => {
                    match nack.nack_type {
                        NackType::Dropped => {
                            tracing::info!(
                                host = host,
                                session = session_id,
                                fragment = frag_idx,
                                "Retransmitting dropped fragment"
                            );
                            self.send_fragment(session_id, fragment.clone());
                            self.network.pdr_from_nack(nack_generator);
                        }
                        NackType::ErrorInRouting(bad) | NackType::UnexpectedRecipient(bad) => {
                            tracing::info!(
                                host = host,
                                session = session_id,
                                fragment = frag_idx,
                                bad_hop = bad,
                                "Routing error, attempting alternate route"
                            );
                            if let Some(new_route) =
                                self.network.find_alternate_route(host, session.destination, &session.route)
                            {
                                tracing::debug!(
                                    host = host,
                                    session = session_id,
                                    fragment = frag_idx,
                                    alt_route = ?new_route,
                                    "Alternate route found, updating and retransmitting"
                                );
                                session.route = new_route.clone();
                                let header = SourceRoutingHeader::new(new_route, 0);
                                let pkt = Packet::new_fragment(header, session_id, fragment.clone());
                                let res = self.send_packet(pkt, None);
                                tracing::debug!(
                                    host = host,
                                    session = session_id,
                                    fragment = frag_idx,
                                    ?res,
                                    "Retransmission result"
                                );
                            } else {
                                drop(sent);
                                tracing::warn!(
                                    host = host,
                                    session = session_id,
                                    "No alternate route, re-initiating flood"
                                );
                                self.initiate_flood();
                            }
                        }
                        NackType::DestinationIsDrone => {
                            tracing::error!(
                                host = host,
                                session = session_id,
                                fragment = frag_idx,
                                "DestinationIsDrone NACK—dropping fragment"
                            );
                        }
                    }
                }
                Some(None) => {
                    tracing::debug!(
                        host = host,
                        session = session_id,
                        fragment = frag_idx,
                        "NACK for fragment already confirmed"
                    );
                }
                None => {
                    tracing::warn!(
                        host = host,
                        session = session_id,
                        fragment = frag_idx,
                        "NACK index out of range"
                    );
                }
            }
        } else {
            tracing::warn!(
                host = host,
                session = session_id,
                "NACK for unknown session"
            );
        }
    }
    fn build_msg_fragment(&mut self, destination: NodeId, message: Message) -> Result<(), String> {
        tracing::info!(
        host = self.id,
        session = message.session_id,
        dest = destination,
        "Starting fragmentation"
    );

        // Trova percorso
        let route = match self.network.find_best_route(self.id, destination) {
            Some(r) => r,
            None => {
                let err = format!("No route to {}", destination);
                error!(host = self.id, session = message.session_id, "{}", err);
                return Err(err);
            }
        };
        println!("Route found for fragmentation: {:?}", route.clone());

        // Calcola numero totale di frammenti
        let bytes = &message.content;
        let total = (bytes.len() + FRAGMENT_DSIZE - 1) / FRAGMENT_DSIZE;

        // Prepara o aggiorna SentSessionData
        {
            let mut sent = self.sent_fragments.lock().map_err(|_| "Mutex poisoned")?;
            let session = sent.entry(message.session_id).or_insert_with(|| {
                tracing::debug!(
                host = self.id,
                session = message.session_id,
                total,
                "New SentSessionData"
            );
                SentSessionData {
                    buffer: vec![None; total],
                    last_activity: Instant::now(),
                    route: route.clone(),
                    destination,
                    pending: total,
                }
            });

            session.pending = total;
            session.last_activity = Instant::now();
            if session.buffer.len() < total {
                session.buffer.resize(total, None);
            }
        }

        // Invio frammenti
        for (i, chunk) in bytes.chunks(FRAGMENT_DSIZE).enumerate() {
            let frag = Fragment {
                fragment_index: i as u64,
                total_n_fragments: total as u64,
                length: chunk.len() as u8,
                data: {
                    let mut d = [0; FRAGMENT_DSIZE];
                    d[..chunk.len()].copy_from_slice(chunk);
                    d
                },
            };

            // Aggiorna buffer
            {
                let mut sent = self.sent_fragments.lock().map_err(|_| "Mutex poisoned")?;
                if let Some(session) = sent.get_mut(&message.session_id) {
                    session.buffer[i] = Some(frag.clone());
                } else {
                    return Err("Missing session in sent_fragments".to_string());
                }
            }

            // Costruisci e invia pacchetto
            let header = SourceRoutingHeader::new(route.clone(), 0);
            let packet = Packet::new_fragment(header.clone(), message.session_id, frag.clone());

            tracing::info!(
                host = self.id,
                session = message.session_id,
                fragment = i,
                "Sending fragment"
            );

            let result = self.send_packet(packet, None);

            tracing::info!(
            host = self.id,
            session = message.session_id,
            fragment = i,
            ?result,
            "Send result"
        );

            if let SendingCodes::SuccessfullySent(_) = result {
                self.network.pdr_helper(header.hops.as_slice());
                tracing::debug!(
                host = self.id,
                hops = ?header.hops,
                fragment = i,
                "PDR updated"
            );
            }
        }

        tracing::info!(
        host = self.id,
        session = message.session_id,
        "Fragmentation completed"
    );

        Ok(())
    }

    fn cleanup_expired_sessions(&self, timeout: Duration) {
        let host = self.id;
        tracing::info!(host, timeout_secs = timeout.as_secs(), "Cleanup start");
        let now = Instant::now();

        // 1) raccogli ID to remove
        let mut to_remove_sent = Vec::new();
        {
            let sent = self.sent_fragments.lock().unwrap();
            for (&sid, sess) in sent.iter() {
                let expired = now.duration_since(sess.last_activity) > timeout;
                let all_conf = sess.buffer.iter().all(|f| f.is_none());
                if expired || all_conf {
                    to_remove_sent.push((sid, expired));
                }
            }
        }

        // 2) rimuovi con lock
        if !to_remove_sent.is_empty() {
            let mut sent = self.sent_fragments.lock().unwrap();
            for (sid, expired) in to_remove_sent {
                sent.remove(&sid);
                if expired {
                    tracing::warn!(host, session = sid, "Removed expired sent session");
                } else {
                    tracing::debug!(host, session = sid, "Removed completed sent session");
                }
            }
        }

        tracing::info!(host, "Cleanup completed");
    }
    fn get_fragment(&self, session_id: u64, fragment_index: u64) -> Option<Fragment> {
        let sent_fragments = self.sent_fragments.lock().unwrap();
        sent_fragments
            .get(&session_id)
            .and_then(|session_data| session_data.buffer.get(fragment_index as usize).cloned().flatten())
    }
    fn send_fragment(&self, session_id: u64, fragment: Fragment) {
        let host = self.id;
        let frag_idx = fragment.fragment_index;

        // 1) Preleva in modo rapido la rotta senza tenere il lock durante l'invio
        let route = {
            let mut sent = self.sent_fragments.lock().unwrap();
            match sent.get_mut(&session_id) {
                Some(session) => {
                    // aggiorno last_activity per questa ritrasmissione
                    session.last_activity = Instant::now();
                    session.route.clone()
                }
                None => {
                    tracing::warn!(
                        host = host,
                        session = session_id,
                        fragment = frag_idx,
                        "send_fragment: session not found"
                    );
                    return;
                }
            }
        };

        // 2) Costruisci header e packet
        let header = SourceRoutingHeader::new(route.clone(), 0);
        let packet = Packet::new_fragment(header.clone(), session_id, fragment.clone());

        // 3) Log e invio
        tracing::info!(
            host = host,
            session = session_id,
            fragment = frag_idx,
            route = ?route,
            "sending fragment"
        );
        let result = self.send_packet(packet, None);
        tracing::info!(
            host = host,
            session = session_id,
            fragment = frag_idx,
            ?result,
            "send result"
        );
    }
    fn build_packet_ack(&self, packet: Packet, fragment_index: Option<u64>) -> Packet{
        let mut routing_header = packet.routing_header;
        routing_header.reverse();
        Packet::new_ack(routing_header, packet.session_id, fragment_index.unwrap_or(0))
    }
}

#[derive(Serialize, Deserialize, Debug)]
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
    MessageFor(ClientId, Message),

}
pub enum NetCommand {
    Net(ClientCommand),
    ControllerShortcut(Packet),
}
#[derive(Serialize, Deserialize, Debug)]
pub enum ServerToClient{
    ServerType(ServerTypes),

    FilesList(Vec<FileId>),
    File(FileId, Vec<u8>),

    MediaList(Vec<MediaId>),
    Media(MediaId, Image),

    ClientList(Vec<ClientId>),
    MessageFrom(ClientId, Message),

    ErrNotFound,
    ErrClientId
}
/// Application events that comes to the SimulationController
pub enum HostEvent {
    /// Rebuilt entire high level message(es. from server to client)
    PacketReconstructed(ServerToClient),
    /// I report sending a low level packet (to be wrapped in the controller if needed)
    PacketSent(Packet),
    // Timeout(session_id),
    // ClientDisconnected(client_id),
    // ecc.
}

#[derive(Hash, Eq, PartialEq, Debug,Deserialize, Serialize)]
pub enum ServerTypes{
    Chat,
    Content
}
///MESSAGES WG_2024
#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub session_id: u64,
    pub content: Vec<u8>,
}
#[derive(Serialize, Deserialize, Debug)]
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