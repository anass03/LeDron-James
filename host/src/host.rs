use std::cmp::Reverse;
use crossbeam_channel::{after, select, Receiver, Sender};
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Packet, FloodRequest, FloodResponse, Ack, Nack, NackType, NodeType, Fragment, FRAGMENT_DSIZE, PacketType};
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use rand::Rng;
use rand::rngs::ThreadRng;
use serde::{Deserialize, Serialize};
use ordered_float::OrderedFloat;

pub type FileId = usize;
pub type MediaId = usize;
pub type ClientId = NodeId;
pub type ServerId = NodeId;
const ALPHA: f64 = 0.1; // fattore di smussamento (regolabile)

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
    pub log_sender: Sender<String>,

    // *** CAMPUS PER IL DEBOUNCE ***
    pub flood_scheduled: bool,
    pub flood_timer: Option<Receiver<Instant>>,

    // *** CONTATORE DI FLOOD PER SESSIONE ***
    max_flood_attempts: u8,
    flood_attempts: HashMap<u64, u8>,  // session_id -> tentativi
}
impl Host{
    pub fn new(id:NodeId,
               type_host:NodeType,
               controller_send:Sender<HostEvent>,
               packet_recv:Receiver<Packet>,
               packet_send:HashMap<NodeId, Sender<Packet>>,
               log_sender: Sender<String>) -> Host {
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
            log_sender,
            flood_scheduled: false,
            flood_timer: None,
            max_flood_attempts: 5,
            flood_attempts: HashMap::new(),
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

    /// Per ogni drone media mobile esponenziale: EWMA
    pub drone_pdr: HashMap<NodeId, f64>,
}
impl Network {
    pub fn new() -> Self {
        Self {
            // graph: DiGraph::new(),
            // topology: HashMap::new(),
            servers: HashSet::new(),
            // drones_packet_sent: HashMap::new()
            nodes: HashMap::new(),
            drone_pdr: HashMap::new(),
        }
    }
    /// Aggiorna edge.weight = 1.0 - drone_pdr[drone_id] per ogni arco uscente da `drone_id`.
    /// In questo modo, in `find_best_route` un arco avrà affidabilità = drone_pdr[&drone_id].
    pub fn recalc_pdr(&mut self) {
        for (&drone_id, &pdr) in &self.drone_pdr {
            if let Some((node_type, edges)) = self.nodes.get_mut(&drone_id) {
                if *node_type == NodeType::Drone {
                    let clamped = pdr.clamp(0.0, 1.0);
                    for edge in edges.iter_mut() {
                        edge.weight = clamped;
                    }
                }
            }
        }
    }
    /// Ogni volta che un drone “ha trasmesso” (forward), abbassiamo la sua stima di PDR:
    /// new_pdr = α * 0.0 + (1 − α) * old_pdr
    pub fn pdr_helper(&mut self, hops: &[NodeId]) {
        for &drone_id in hops {
            // Se non esiste ancora, lo inizializziamo a 1.0 (fiducia totale)
            let old = self.drone_pdr.entry(drone_id).or_insert(0.0);
            // EWMA verso 0.0 (abbassiamo la PDR perché ha trasmesso)
            let new_pdr = (1.0 - ALPHA) * *old;
            *old = new_pdr.clamp(0.0, 1.0);
        }
        self.recalc_pdr();
    }
    /// Ogni volta che un drone invia un NACK “Dropped”, abbassiamo ulteriormente la PDR:
    /// new_pdr = α * 0.0 + (1 − α) * old_pdr
    pub fn pdr_from_nack(&mut self, node_id: NodeId) {
        let old = self.drone_pdr.entry(node_id).or_insert(0.0);
        // EWMA verso 0.0 (penalizziamo il link quando effettivamente drop)
        let new_pdr = ALPHA * 1.0 + (1.0 - ALPHA) * *old;
        *old = new_pdr.clamp(0.0, 1.0);
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
                    // Qui estraggo direttamente la stima EWMA di PDR per il drone `u`
                    let reliability = 1.0 - edge.weight;
                    // Se la stima è zero o negativa, salto subito l'arco
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

                    // idem: leggo la stima EWMA per `u`
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
    fn send_logging(&self, message: String);
    fn schedule_flood(&mut self);
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
    /// Inoltra una stringa di log al simulation controller.
    fn send_logging(&self, message: String) {
        let role = if self.network.servers.contains(&self.id) { "Server" } else { "Client" };
        let log_msg = format!("[{}] ID {} - {}", role, self.id, message);

        if let Err(err) = self.log_sender.send(log_msg) {
            eprintln!("Failed to forward logging to controller: {}", err);
        }

    }
    /// Al posto di chiamare initiate_flood() direttamente, uso questo
    fn schedule_flood(&mut self) {
        // Se ho già schedulato un flood, ignoro
        if self.flood_scheduled {
            return;
        }
        self.flood_scheduled = true;
        // Il channel `after` manderà un messaggio dopo 100ms
        self.flood_timer = Some(after(Duration::from_millis(100)));
        self.send_logging("Flood scheduled (debounced)".to_string());
    }
    fn initiate_flood(&mut self) {
        if !self.network.nodes.is_empty() {
            self.network.nodes.clear();
        }
        if self.packet_send.is_empty() {
            self.send_logging(format!("No neighbors to flood to host={}",  self.id));
            // Siamo in errore di rete: MA siccome non ho inviato alcun FloodRequest,
            // devo _azzerare_ flood_scheduled e il flood_timer, altrimenti
            // il select li vede sempre pronti e richiama all’infinito questa funzione.
            self.flood_scheduled = false;
            self.flood_timer = None;
            return;
        }
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

        self.send_logging(format!("host: {}, flood_id: {},session_id: {}, Starting flood discovery", host_id, flood_id,session_id));

        if self.packet_send.is_empty() {
            self.send_logging(format!("host: {}, No neighbors to flood to",host_id));
            return;
        }

        // Inoltro la richiesta a tutti i vicini
        for (neighbour_id, sender) in &self.packet_send {
            self.send_logging(format!(
                "Sending FloodRequest host={} neighbor={}",
                host_id, neighbour_id,
            ));
            let result = self.send_packet(flood_request.clone(), Some(sender));
            let neighbour = neighbour_id;
            self.send_logging(format!(
                "FloodRequest send result: host={}, neishbor={}, result={}",
                host_id, neighbour, result,
            ));}
    }
    fn process_flood_request(&mut self, flood_request: FloodRequest, packet: Packet) {
        let host_id = self.id;
        let initiator = flood_request.initiator_id;
        let flood_id = flood_request.flood_id;
        let session_id = packet.session_id;
        let host = host_id;

        self.send_logging(format!(
            "Handling FloodRequest host={} initiator={} flood_id={} session={}",
            host,
            initiator,
            flood_id,
            session_id,
        ));
        let mut fr = flood_request.clone();

        // 1) Controllo se ho già visto questo flood_id dall'initiator
        let seen_flood_ids = self.flood_cache.entry(initiator).or_default();

        if seen_flood_ids.contains(&flood_id) {
            self.send_logging(format!("Path trace before adding myself: {:?}", fr.path_trace));

            fr.increment(host_id, self.type_host);
            let host = host_id;
            self.send_logging(format!(
                "FloodRequest already seen, sending response immediately host={} initiator={} flood_id={}",
                host,
                initiator,
                flood_id,
            ));
            self.send_logging(format!("Path trace after adding myself: {:?}", fr.path_trace));
            let mut response_pkt = fr.generate_response(session_id);
            response_pkt.routing_header.hop_index += 1;

            let next_hop = response_pkt.routing_header.hops[response_pkt.routing_header.hop_index];

            self.send_logging(format!(
                "Attempting to send FloodResponse to next hop host={} next_hop={} neighbors={:?}",
                host_id, next_hop, self.packet_send.keys().collect::<Vec<_>>(),
            ));
            let result = self.send_packet(response_pkt, None);

            self.send_logging(format!(
                "Sent FloodResponse for duplicate FloodRequest host={}, result={}",
                host_id, result,
            ));
            return;
        }

        // 2) Registriamo il flood_id come visto
        seen_flood_ids.push(flood_id);

        // 3) Verifico chi ha inviato il pacchetto
        let from = match flood_request.path_trace.last() {
            Some(&(node_id, _)) => node_id,
            None => {
                self.send_logging("Received FloodRequest with empty path-trace! Dropping packet.".to_string());
                return;
            }
        };
        let host = host_id;
        self.send_logging(format!(
            "FloodRequest arrived from neighbor host={} from={}",
            host,
            from
        ));
        self.send_logging(format!("Path trace before adding myself: {:?}", fr.path_trace));
        // 4) Aggiorno path_trace
        fr.increment(host_id, self.type_host);
        self.send_logging(format!("Path trace after adding myself: {:?}", fr.path_trace));
        // 5) Inoltro a tutti i vicini (escluso chi me l'ha mandato)
        let neighbors: Vec<_> = self
            .packet_send
            .iter()
            .filter(|(&nid, _)| nid != from)
            .collect();

        if neighbors.is_empty() {
            // 6a) Fine linea: genero immediatamente FloodResponse
            self.send_logging(format!("No other neighbors: replying flood immediately host={}", host_id));
            let mut resp_pkt = fr.generate_response(session_id);
            resp_pkt.routing_header.hop_index+=1;
            let next_hop = resp_pkt.routing_header.hops[resp_pkt.routing_header.hop_index];
            self.send_logging(format!(
                "Attempting to send FloodResponse to next hop host={} next_hop={} neighbors={:?}",
                host_id,
                next_hop,
                self.packet_send.keys().collect::<Vec<_>>(),
            ));
            let result = self.send_packet(resp_pkt, None);

            self.send_logging(format!(
                "FloodResponse sent from leaf node host={}, result={}",
                host_id, result,
            ));
        } else {
            // 6b) Inoltro ai vicini disponibili
            self.send_logging(format!(
                "Forwarding FloodRequest to neighbors host={} count={}",
                host_id,
                neighbors.len(),
            ));
            for (&nbr, ch) in neighbors {
                self.send_logging(format!("Forwarding to neighbor host={} neighbor={}",  host_id, nbr));
                let fwd_pkt = Packet::new_flood_request(
                    packet.routing_header.clone(),
                    session_id,
                    fr.clone(),
                );
                let result = self.send_packet(fwd_pkt, Some(ch));

                self.send_logging(format!(
                    "ForwardFloodRequest result host={} neighbor={} result={}",
                    host_id, nbr, result,
                ));
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

        self.send_logging(format!(
            "Handling FloodResponse with path host={} flood_id={}, path_str={}",
            host_id,
            response.flood_id,
            path_str,
        ));
        let path = &response.path_trace;
        if path.len() < 2 {
            self.send_logging(format!(
                "FloodResponse path too short, skipping host={} len={}",
                host_id,
                path.len(),
            ));
            return;
        }

        // 1) Assicura che tutti i nodi nel percorso siano nella adjacency-list
        for &(node_id, node_type) in path {
            // entry nuovo nodo, se non esiste
            self.network
                .nodes
                .entry(node_id)
                .or_insert_with(|| (node_type, Vec::new()));

            // per i droni, inizializza subito pdr
            if node_type == NodeType::Drone {
                self.network
                    .drone_pdr
                    .entry(node_id)
                    .or_insert(0.0);
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
                    self.send_logging(format!(
                        "Recorded adjacency edge host={} from={} to={}",
                        host_id,
                        curr_id,
                        next_id,
                    ));
                }
                // ——— Aggiungi anche l'arco inverso ———
                if let Some((_, inv_edges)) = self.network.nodes.get_mut(&next_id) {
                    // evita doppioni
                    if !inv_edges.iter().any(|e| e.to == curr_id) {
                        inv_edges.push(Edge { to: curr_id, weight: 0.0 });
                        self.send_logging(format!(
                            "Recorded adjacency edge host={} from={} to={}",
                            host_id,
                            next_id,
                            curr_id,
                        ));
                    }
                }

            } else {
                self.send_logging(format!(
                    "Current node missing from network.nodes host={} node={}",
                    host_id, curr_id,
                ));
            }
        }

        // (Opzionale) se vuoi ricalcolare subito i PDR:
        self.network.recalc_pdr();
    }
    fn send_packet(&self, packet: Packet, channel: Option<&Sender<Packet>>) -> SendingCodes {
        self.send_logging(format!("Sending packet... host={}", self.id));

        let next_channel = channel.or_else(|| {
            self.packet_send
                .get(&packet.routing_header.hops[packet.routing_header.hop_index])
        });
        let route = packet.routing_header.hops.clone();
        self.send_logging(format!("😂 Route={:?}", route));

        let result = match next_channel {
            Some(ch) => match ch.send(packet.clone()) {
                Ok(_) => {
                    if let Err(err) = self.sendto_controller(packet.clone()) {
                        self.send_logging(format!("Notifying controller gone wrong: {}", err));
                    }
                    SendingCodes::SuccessfullySent(packet.session_id)
                }
                Err(err) => {
                    if let Err(e) = self.sendto_controller(packet) {
                        self.send_logging(format!("Notifying controller, for error sending packet, gone wrong: {}", e));
                    }
                    SendingCodes::ErrorSending(err.to_string())
                }
            },
            None => {
                if let Err(e) = self.sendto_controller(packet) {
                    self.send_logging(format!("Error, notifying controller: {}", e));
                }
                SendingCodes::NoNextHop(format!("No valid channel found for sending packet. host={}", self.id))
            }
        };

        self.send_logging(format!("{:?}", result));
        result
    }
    fn sendto_controller(&self, packet: Packet) -> Result<(), String> {
        self.send_logging("Sending to controller...".to_string());

        match self.controller_send.send(HostEvent::PacketSent(packet)) {
            Ok(_) => {
                self.send_logging("Successfully sent to controller.".to_string());
                Ok(())
            }
            Err(er) => {
                self.send_logging(format!("Failed to send to controller: {}", er));
                Err(er.to_string())
            }
        }
    }
    fn handle_ack(&self, ack: Ack, session_id: u64) {

        self.send_logging("--- ✅ Drone packet stats ✅ ---".to_string());
        for (&drone_id, &pdr) in &self.network.drone_pdr {
            // pdr è la stima di “reliability” (0.0–1.0). Il drop_rate corrisponde a 1.0 – pdr.
            let reliability = 1.0 - pdr.clamp(0.0, 1.0);
            self.send_logging(format!(
                "Drone {}: pdr = {:.3}, reliability = {:.3}",
                drone_id, pdr, reliability
            ));
        }
        self.send_logging("--------------------------".to_string());

        let idx = ack.fragment_index as usize;
        self.send_logging(format!(
            "Received ACK host={} session={} fragment={}",
            self.id,
            session_id,
            ack.fragment_index,
        ));
        let mut sent = self.sent_fragments.lock().unwrap();
        match sent.get_mut(&session_id){
            None => { self.send_logging(format!(
                    "ACK for unknown session host={} session={}",
                    self.id, session_id,
                ));
            }
            Some(session) => {
                match session.buffer.get(idx){
                    None => {
                        self.send_logging(format!(
                            "received ACK for nonexistent fragment host={} session={} fragment={}",
                            self.id,
                            session_id,
                            ack.fragment_index,
                        ));
                    }
                    Some(None) => {
                        self.send_logging(format!(
                            "received ACK for already confirmed fragment host={} session={} fragment={}",
                            self.id,
                            session_id,
                            ack.fragment_index,
                        ));
                    }
                    Some(Some(_)) => {
                        session.buffer[idx] = None;
                        session.pending = session.pending.saturating_sub(1);
                        session.last_activity = Instant::now();
                        self.send_logging(format!(
                            "ACK processed, host={} session={} pending={}",
                            self.id,
                            session_id,
                            session.pending,
                        ));
                    }
                }
            }
        }
    }
    fn handle_nack(&mut self, nack: Nack, nack_generator: NodeId, session_id: u64) {
        let host = self.id;
        let frag_idx = nack.fragment_index;

        // 1) Apro il lock TANTO PER PRENDERE fragment, destination e route correnti,
        //    quindi droppo subito il lock.
        let (fragment, mut current_route, destination) = {
            let mut sent = self.sent_fragments.lock().unwrap();
            if let Some(session) = sent.get_mut(&session_id) {
                match session.buffer.get(frag_idx as usize) {
                    Some(Some(f)) => {
                        let frag_clone = f.clone();
                        let route_clone = session.route.clone();
                        let dest = session.destination;
                        (frag_clone, route_clone, dest)
                    }
                    Some(None) => {
                        // frammento già confermato o non esistente
                        self.send_logging(format!(
                            "NACK per frammento inesistente/già confermato host={} session={} fragment={}",
                            host, session_id, frag_idx
                        ));
                        return;
                    }
                    None => {
                        // indice fuori range
                        self.send_logging(format!(
                            "NACK index out of range host={} session={} fragment={}",
                            host, session_id, frag_idx
                        ));
                        return;
                    }
                }
            } else {
                // sessione sconosciuta
                self.send_logging(format!(
                    "NACK per sessione sconosciuta host={} session={}",
                    host, session_id
                ));
                return;
            }
        };
        // Qui il MutexGuard su sent_fragments è già stato droppato.

        match nack.nack_type {
            NackType::Dropped => {
                // 2) NACK “Dropped”: ritrasmetto subito il frammento
                self.send_logging(format!(
                    "Retransmitting dropped fragment host={} session={} fragment={}",
                    host, session_id, frag_idx
                ));
                self.send_fragment(session_id, fragment.clone());
                self.network.pdr_from_nack(nack_generator);
            }

            NackType::ErrorInRouting(bad) | NackType::UnexpectedRecipient(bad) => {
                self.send_logging(format!(
                    "Routing error, attempting alternate route host={} session={} fragment={} bad_hop={}",
                    host, session_id, frag_idx, bad
                ));

                // 3) Provo subito una rotta alternativa sulla route corrente
                if let Some(new_route) =
                    self.network.find_alternate_route(host, destination, &current_route)
                {
                    self.send_logging(format!(
                        "Alternate route found, updating and retransmitting host={} session={} fragment={} alt_route={:?}",
                        host, session_id, frag_idx, new_route
                    ));

                    // 3.a) Apro il lock solo per aggiornare session.route
                    {
                        let mut sent = self.sent_fragments.lock().unwrap();
                        if let Some(session) = sent.get_mut(&session_id) {
                            session.route = new_route.clone();
                            session.last_activity = Instant::now();
                        } else {
                            // La sessione è già stata rimossa nel frattempo: esco
                            self.send_logging(format!(
                                "Session {} già rimossa, non ritrasmetto",
                                session_id
                            ));
                            return;
                        }
                    } // il lock viene droppato qui

                    // 3.b) Invio il frammento sulla nuova rotta
                    current_route = new_route.clone();
                    let header = SourceRoutingHeader::new(new_route.clone(), 0);
                    let pkt = Packet::new_fragment(header, session_id, fragment.clone());
                    let res = self.send_packet(pkt, None);
                    self.send_logging(format!(
                        "Retransmission result host={} session={} fragment={} result={:?}",
                        host, session_id, frag_idx, res
                    ));
                } else {
                    // 3.c) Non c'è rotta alternativa: incremento flood_attempts e valuto se abortire o floodare
                    let cnt = {
                        let entry = self.flood_attempts.entry(session_id).or_insert(0);
                        *entry += 1;
                        *entry
                    };

                    if cnt > self.max_flood_attempts {
                        // 3.d) Superata la soglia: rete scollegata, rimuovo la sessione
                        self.send_logging(format!(
                            "❌ NET DISCONNECTED, session {} fallita dopo {} flood → aborting",
                            session_id,
                            cnt - 1 // cnt-1 perché cnt è già stato incrementato oltre la soglia
                        ));
                        let mut sent = self.sent_fragments.lock().unwrap();
                        sent.remove(&session_id);
                    } else {
                        // 3.e) Pianifico un solo flood, poi esco
                        self.send_logging(format!(
                            "No alternate route per session {}, scheduling flood attempt {}/{}",
                            session_id, cnt, self.max_flood_attempts
                        ));
                        self.initiate_flood();
                    }
                }
            }

            NackType::DestinationIsDrone => {
                self.send_logging(format!(
                    "DestinationIsDrone NACK — dropping fragment host={} session={} fragment={}",
                    host, session_id, frag_idx
                ));
            }
        }
    }

    fn build_msg_fragment(&mut self, destination: NodeId, message: Message) -> Result<(), String> {
        self.send_logging(format!(
            "Starting fragmentation host={} session={} dest={}",
            self.id,
            message.session_id,
            destination,
        ));
        // Trova percorso
        let route = match self.network.find_best_route(self.id, destination) {
            Some(r) => r,
            None => {
                let err = format!("No route to {}", destination);
                self.send_logging(format!("{} host={} session={}", err, self.id, message.session_id));
                return Err(err);
            }
        };
        self.send_logging(format!("✅💥💥 Route found for fragmentation: {:?}", route.clone()));

        // Calcola numero totale di frammenti
        let bytes = &message.content;
        self.send_logging(format!("❤️❤️ vettore di byte: {:?}", bytes.clone()));
        let total = (bytes.len() + FRAGMENT_DSIZE - 1) / FRAGMENT_DSIZE;

        // Prepara o aggiorna SentSessionData
        {
            let mut sent = self.sent_fragments.lock().map_err(|_| "Mutex poisoned")?;
            let session = sent.entry(message.session_id).or_insert_with(|| {
                let host = self.id;
                let session = message.session_id;
                self.send_logging(format!(
                    "New SentSessionData host={} session={} total={}",
                    host,
                    session,
                    total,
                ));
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
            let header = SourceRoutingHeader::new(route.clone(), 1);
            let packet = Packet::new_fragment(header.clone(), message.session_id, frag.clone());

            self.send_logging(format!(
                "Sending fragment host={} session={} fragment={}",
                self.id,
                message.session_id,
                i,
            ));
            let result = self.send_packet(packet, None);

            self.send_logging(format!(
                "Send result host={} session={} fragment={} result={}",
                self.id, message.session_id, i, result
            ));

        }

        self.send_logging(format!(
            "Fragmentation completed host={} session={}",
            self.id,
            message.session_id,
        ));
        Ok(())
    }
    fn cleanup_expired_sessions(&self, timeout: Duration) {
        let host = self.id;
        self.send_logging(format!("Cleanup start host={} timeout_secs={}", host, timeout.as_secs()));
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
                    self.send_logging(format!("Removed expired sent session host={} session={}", host, sid));
                } else {
                    self.send_logging(format!("Removed completed sent session host={} session={}", host, sid));
                }
            }
        }

        self.send_logging(format!("Cleanup completed host={}", host));
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
                    self.send_logging(format!(
                        "send_fragment: session not found host={} session={} fragment={}",
                        host, session_id, frag_idx,
                    ));
                    return;
                }
            }
        };

        // 2) Costruisci header e packet
        let header = SourceRoutingHeader::new(route.clone(), 0);
        let mut packet = Packet::new_fragment(header.clone(), session_id, fragment.clone());

        // 3) Log e invio
        self.send_logging(format!(
            "sending fragment host={}, session={} fragment={}, route={:?}",
            host, session_id, frag_idx, route
        ));
        packet.routing_header.hop_index +=1;
        let result = self.send_packet(packet, None);
        self.send_logging(format!(
            "send result host={}, session={} fragment={}, result={}",
            host, session_id, frag_idx, result
        ));
    }
    fn build_packet_ack(&self, packet: Packet, fragment_index: Option<u64>) -> Packet{
        let mut routing_header = packet.routing_header;
        routing_header.reverse();
        Packet::new_ack(routing_header, packet.session_id, fragment_index.unwrap_or(0))
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub enum ClientNetCommand {
    //client command
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
}
pub enum HostCommand {
    AddSender(NodeId, Sender<Packet>),
    RemoveSender(NodeId),
}
#[derive(Serialize, Deserialize, Debug)]
pub enum ServerToClient{
    ServerType(ServerTypes),

    FilesList(Vec<FileId>),
    File(FileId, Vec<u8>),

    MediaList(Vec<MediaId>),
    Media(MediaId, Image),

    ClientList(Vec<ClientId>),
    MessageFrom(ClientId, String),

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
