use crossbeam_channel::{Receiver, Sender};
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Packet, FloodRequest, FloodResponse, Ack, Nack, NackType, NodeType, Fragment, FRAGMENT_DSIZE, PacketType};
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use petgraph::algo::dijkstra;
use petgraph::graph::{DiGraph, NodeIndex};
use tracing::{info, warn, error, debug};
use rand::Rng;
use rand::rngs::ThreadRng;
use serde::{Deserialize, Serialize};

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
        // Rimuove tutti gli archi esistenti
        self.graph.clear_edges();

        for (node_id, (node_index, neighbors)) in &self.topology {
            for neighbor_id in neighbors {
                if let Some((neighbor_index, _)) = self.topology.get(neighbor_id) {
                    if !self.graph.contains_edge(*node_index, *neighbor_index) {
                        self.graph.add_edge(*node_index, *neighbor_index, 0.0);
                    }
                    if !self.graph.contains_edge(*neighbor_index, *node_index) {
                        self.graph.add_edge(*neighbor_index, *node_index, 0.0);
                    }
                } else {
                    warn!(
                    "Neighbor {} of node {} not found in topology. Skipping edge.",
                    neighbor_id,
                    node_id
                );
                }
            }
        }

    }

    pub fn modify_pdr(&mut self) {
        for i in self.topology.keys() {
            if let Some((index_i, neighbors)) = self.topology.get(i) {
                if let Some((sent, total)) = self.drones_packet_sent.get(i) {
                    let pdr = if *total == 0 {
                        0.0
                    } else {
                        *total as f64 / *sent as f64
                    };

                    for j in neighbors {
                        if let Some((index_j, _)) = self.topology.get(j) {
                            if let Some(edge) = self.graph.find_edge(*index_i, *index_j) {
                                if let Some(weight) = self.graph.edge_weight_mut(edge) {
                                    *weight = pdr;
                                }
                            }
                        } else {
                            warn!("Neighbour node {} not found in topology.", j);
                        }
                    }
                } else {
                    warn!("Packet stats not found for node {} in drones_packet_sent.", i);
                }
            } else {
                warn!("Node {} not found in topology.", i);
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
    pub fn find_best_route(&self, start: NodeId, destination: NodeId) -> Option<Vec<NodeId>> {
        let start_index = self.topology.get(&start)?.0;
        let destination_index = self.topology.get(&destination)?.0;

        let shortest_paths = dijkstra(&self.graph, start_index, Some(destination_index), |edge| *edge.weight());

        if !shortest_paths.contains_key(&destination_index) {
            return None; // Nessun percorso trovato
        }

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
        Some(path.iter().filter_map(|&n| self.graph.node_weight(n).copied()).collect())
    }

    /// Trova un percorso alternativo, cercando comunque di soddisfare le condizioni di efficienza
    pub fn find_alternate_route(&self, start: NodeId, destination: NodeId, used_route: &[NodeId]) -> Option<Vec<NodeId>> {
        let start_index = self.topology.get(&start)?.0;
        let destination_index = self.topology.get(&destination)?.0;

        let mut graph = self.graph.clone();

        // Rimuove gli archi del percorso già usato
        for window in used_route.windows(2) {
            if let [prev, next] = window {
                if let (Some(&(prev_index, _)), Some(&(next_index, _))) =
                    (self.topology.get(prev), self.topology.get(next))
                {
                    if let Some(edge_index) = graph.find_edge(prev_index, next_index) {
                        graph.remove_edge(edge_index);
                    }
                }
            }
        }

        let shortest_paths = dijkstra(&graph, start_index, Some(destination_index), |edge| *edge.weight());

        if !shortest_paths.contains_key(&destination_index) {
            return None; // Nessun percorso alternativo trovato
        }

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
        Some(path.iter().filter_map(|&n| self.graph.node_weight(n).copied()).collect())
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

        // 1) Assicura che tutti i nodi nel percorso siano aggiunti alla topologia
        for &(node_id, node_type) in path {
            self.network.topology.entry(node_id).or_insert_with(|| {
                let idx = self.network.graph.add_node(node_id);
                (idx, Vec::new())
            });

            if node_type == NodeType::Server {
                self.network.servers.insert(node_id);
                tracing::debug!(host = host_id, server = node_id, "Marked node as server");
            }
        }

        // 2) Registra connessioni tra nodi adiacenti
        for window in path.windows(2) {
            let (curr_id, _) = window[0];
            let (next_id, _) = window[1];

            let entry = self.network.topology.get_mut(&curr_id).unwrap();
            if !entry.1.contains(&next_id) {
                entry.1.push(next_id);
                tracing::debug!(
                host = host_id,
                from = curr_id,
                to = next_id,
                "Recorded topology edge"
            );
            }
        }

        // 3) Aggiorna la rappresentazione del grafo
        self.network.update_graph();

        tracing::info!(
        host = host_id,
        nodes = self.network.topology.len(),
        "Topology graph updated"
    );

        // Debug opzionale per nodo specifico
        if host_id == 6 {
            tracing::debug!("🔍 Topology at node 6: {:?}", self.network.topology.keys());
            tracing::debug!("🔍 Servers known to node 6: {:?}", self.network.servers);
            if let Some(route) = self.network.find_best_route(6, 7) {
                tracing::debug!("✅ Route from 6 to 7 found: {:?}", route);
            } else {
                tracing::debug!("❌ No route yet from 6 to 7");
            }
        }
    }


    fn send_packet(&self, packet: Packet, channel: Option<&Sender<Packet>>) -> SendingCodes {
        info!("Sending packet...");

        match packet.pack_type {
            PacketType::FloodRequest(_) => {
                if channel.is_none() { tracing::info!("TRALLALERO TRALLALA"); }
            }
            _ => {}
        }

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
                self.network.pdr_helper(header.clone());
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