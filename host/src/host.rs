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
pub type Extension = String;
const ALPHA: f64 = 0.01; // fattore di smussamento (regolabile)


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

    /// quanto vogliamo aspettare tra un flood e il successivo
    nack_flood_backoff: Duration,
    /// timestamp dell’ultimo flood scatenato da un NACK
    last_nack_flood: Instant,

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
            nack_flood_backoff: Duration::from_secs(5),
            last_nack_flood: Instant::now() - Duration::from_secs(5),
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
    banned_nodes: HashSet<NodeId>,
}
#[derive(Clone, Debug)]
pub struct Edge {
    pub to:     NodeId,
    pub weight: f64,
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
    /// Ogni volta che un drone “ha trasmesso con successo!” (forward), abbassiamo la sua stima di PDR:
    /// new_pdr = α * 0.0 + (1 − α) * old_pdr
    pub fn pdr_helper(&mut self, hops: &[NodeId]) {
        for &drone_id in hops {
            // Se non esiste ancora, lo inizializziamo a 1.0 (fiducia totale)
            let old = self.drone_pdr.entry(drone_id).or_insert(0.0);
            // EWMA verso 0.0 (abbassiamo la PDR perché ha trasmesso)
            let new_pdr = ALPHA * 0.0 + (1.0 - ALPHA) * *old;
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
                    // se v NON è né start né destination, allora deve essere un drone
                    if v != start && v != destination {
                        match self.nodes.get(&v) {
                            Some((NodeType::Drone, pdr)) => {  }
                            _ => continue,  // salta host o altri tipi intermedi
                        }
                    }
                    // Estrai la PDR del drone di ARRIVO (v), non di u
                    let pdr_v = self.drone_pdr
                        .get(&v)
                        .copied()
                        .unwrap_or(0.0);
                    let reliability = 1.0 - pdr_v;
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
    pub fn find_best_route_excluding(
        &self,
        start: NodeId,
        destination: NodeId,
        banned_nodes: &HashSet<NodeId>,
    ) -> Option<Vec<NodeId>> {

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
            if let Some((_, edges)) = self.nodes.get(&u) {
                for edge in edges {
                    let v = edge.to;

                    // Se il nodo è bannato, lo skippiamo subito
                    if banned_nodes.contains(&v) {
                        continue;
                    }

                    // Se v NON è né start né destination, allora deve essere un drone
                    if v != start && v != destination {
                        match self.nodes.get(&v) {
                            Some((NodeType::Drone, _)) => { /* ok */ }
                            _ => continue,  // salta host o altri tipi intermedi
                        }
                    }

                    let pdr_v = self.drone_pdr
                        .get(&v)
                        .copied()
                        .unwrap_or(0.0);
                    let reliability = 1.0 - pdr_v;
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
    fn initiate_flood(&mut self);
    fn process_flood_request(&mut self, flood_request: FloodRequest,packet: Packet);
    fn process_flood_response(&mut self, packet: Packet);
    fn send_packet(&self, packet: Packet, channel: Option<&Sender<Packet>>) -> SendingCodes;
    fn sendto_controller(&self, packet: Packet) -> Result<(),String>;
    fn handle_ack(&mut self, ack: Ack, hops: &[NodeId], session_id: u64);
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
        let role = self.type_host;
        let log_msg = format!("[{:?}] ID {} - {}", role, self.id, message);

        if let Err(err) = self.log_sender.send(log_msg) {
            eprintln!("Failed to forward logging to controller: {}", err);
        }
    }
    /// Al posto di chiamare initiate_flood() direttamente, uso questo
    fn initiate_flood(&mut self) {
        if !self.network.nodes.is_empty() {
            self.network.nodes.clear();
        }
        if self.packet_send.is_empty() {
            self.send_logging(format!("No neighbors to flood to host={}",  self.id));
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
    fn process_flood_response(&mut self, mut pkt: Packet) {
        // 1) Estraggo il FloodResponse e l’header di routing
        let (response, routing_header) = match &mut pkt.pack_type {
            PacketType::FloodResponse(resp) => (resp, &mut pkt.routing_header),
            _ => {
                // Non dovrebbe succedere: sto chiamando questa funzione
                // solo per FloodResponse. Se capita, esco subito.
                return;
            }
        };

        let host_id = self.id;
        let flood_id = response.flood_id;

        // 2) Controllo che path_trace non sia vuoto
        if response.path_trace.is_empty() {
            self.send_logging(format!("FloodResponse senza path_trace – host={}", host_id));
            return;
        }
        self.send_logging(format!("👌👌👌 path trace da controllare, {:?}", response.path_trace));
        // 3) The last element in path_trace is the initiator
        let (initiator, _) = response.path_trace.first().unwrap();

        // 4) If I'm not the initiator, forward the response back along the path
        if *initiator != host_id {
            routing_header.hop_index += 1;
            if routing_header.hop_index < routing_header.hops.len() {
                let next_hop = routing_header.hops[routing_header.hop_index];
                self.send_logging(format!(
                    "Forwarding FloodResponse: host={} flood_id={} -> next_hop={}",
                    host_id,
                    response.flood_id,
                    next_hop
                ));
                let _ = self.send_packet(pkt.clone(), None);
            } else {
                self.send_logging(format!(
                    "Invalid hop_index in FloodResponse: host={} hop_index={} len={}",
                    host_id,
                    routing_header.hop_index,
                    routing_header.hops.len()
                ));
            }
            return;
        }

        // ◁ Se arrivo qui, “sono” l’iniziatore (e flood_id ∈ flood_cache[self.id]):
        //    posso far partire la logica che registra i nodi e gli archi
        self.send_logging(format!(
            "Handling FloodResponse per me host={} flood_id={} path={:?}",
            host_id,
            flood_id,
            response
                .path_trace
                .iter()
                .map(|(nid, ty)| format!("{:?}({})", ty, nid))
                .collect::<Vec<_>>()
                .join(" -> ")
        ));

        let path = &response.path_trace;
        if path.len() < 2 {
            self.send_logging(format!(
                "FloodResponse path troppo corto, host={} len={}",
                host_id,
                path.len()
            ));
            return;
        }

        // 5) Registro tutti i nodi nella mia adjacency-list
        for &(node_id, node_type) in path {
            let entry = self
                .network
                .nodes
                .entry(node_id)
                .or_insert_with(|| (node_type, Vec::new()));
            // inizializzo pdr se è drone
            if entry.0 == NodeType::Drone {
                self.network.drone_pdr.entry(node_id).or_insert(0.0);
            }
            // se è server, lo aggiungo all’insieme
            if entry.0 == NodeType::Server {
                self.network.servers.insert(node_id);
            }
        }

        // 6) Registro gli archi tra nodi consecutivi
        for window in path.windows(2) {
            let (curr_id, _) = window[0];
            let (next_id, _) = window[1];
            // aggiungo arco curr_id → next_id
            if let Some((_, edges)) = self.network.nodes.get_mut(&curr_id) {
                if !edges.iter().any(|e| e.to == next_id) {
                    edges.push(Edge { to: next_id, weight: 0.0 });
                    self.send_logging(format!(
                        "Recorded adjacency edge host={} da {} a {}",
                        host_id, curr_id, next_id
                    ));
                }
            }
            // aggiungo anche l’arco inverso next_id → curr_id
            if let Some((_, inv_edges)) = self.network.nodes.get_mut(&next_id) {
                if !inv_edges.iter().any(|e| e.to == curr_id) {
                    inv_edges.push(Edge { to: curr_id, weight: 0.0 });
                    self.send_logging(format!(
                        "Recorded adjacency edge host={} da {} a {}",
                        host_id, next_id, curr_id
                    ));
                }
            }
        }

        // 7) (Opzionale) Ricalcolo subito i PDR
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
    fn handle_ack(&mut self, ack: Ack, hops: &[NodeId], session_id: u64) {
        let drone_hops: Vec<NodeId> = hops.iter()
            .filter(|&&nid| {
                matches!(
                        self.network.nodes.get(&nid).map(|(ty, _)| ty),
                        Some(NodeType::Drone))}
            )
            .copied()
            .collect();
        self.network.pdr_helper(&drone_hops);

        self.send_logging(format!(
            "PDR updated host={} header hops={:?}",
            self.id,
            hops,
        ));

        // 2) *RICALCOLO* la rotta per la sessione, dato che i PDR sono cambiati
        {
            let mut sent = self.sent_fragments.lock().unwrap();
            if let Some(session) = sent.get_mut(&session_id) {
                let dest = session.destination;
                if let Some(new_route) = self.network.find_best_route(self.id, dest) {
                    session.route = new_route;
                    session.last_activity = Instant::now();
                    self.send_logging(format!(
                        "🔄 Route aggiornata host={} session={} route={:?}",
                        self.id, session_id, session.route
                    ));
                }
            }
        }
        self.send_logging("--- ✅ Drone packet stats ✅ ---".to_string());
        for (&drone_id, &pdr) in &self.network.drone_pdr {
            // pdr è la stima di “reliability” (0.0–1.0). Il drop_rate corrisponde a 1.0 – pdr.
            let reliability = 1.0 - pdr.clamp(0.0, 1.0);
            self.send_logging(format!(
                "Drone {}: expected pdr = {:.3}, reliability = {:.3}",
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

        // 1) Recupero i dati di sessione (fragment, route, banned)
        let (fragment, current_route, destination, mut banned_nodes) = {
            let sent = self.sent_fragments.lock().unwrap();
            if let Some(session) = sent.get(&session_id) {
                match session.buffer.get(frag_idx as usize) {
                    Some(Some(f)) => (f.clone(), session.route.clone(), session.destination, session.banned_nodes.clone()),
                    Some(None) | None => {
                        self.send_logging(format!("NACK su frammento già confermato o out-of-range: session={} frag={}", session_id, frag_idx));
                        return;
                    }
                }
            } else {
                self.send_logging(format!("NACK su sessione sconosciuta: {}", session_id));
                return;
            }
        };

        match nack.nack_type {
            NackType::Dropped => {
                self.send_logging(format!("Retransmitting dropped fragment session={} frag={}", session_id, frag_idx));
                self.send_fragment(session_id, fragment.clone());
                self.network.pdr_from_nack(nack_generator);
            }

            NackType::ErrorInRouting(bad) | NackType::UnexpectedRecipient(bad) => {
                self.send_logging(format!("Routing error on bad_hop={} for session={} frag={}", bad, session_id, frag_idx));

                // Se il bad node NON è già nella route corrente, allora la route corrente è già aggiornata
                if !current_route.contains(&bad) {
                    self.send_logging(format!("Bad node {} not in current route — keep current route.", bad));
                    self.send_fragment(session_id, fragment.clone());
                    return;
                }

                // Aggiungo il nodo problematico nella blacklist
                banned_nodes.insert(bad);

                // Provo una route alternativa escludendo i banned_nodes
                if let Some(new_route) = self.network.find_best_route_excluding(host, destination, &banned_nodes) {
                    self.send_logging(format!("Found alternative route: {:?}", new_route));

                    {
                        let mut sent = self.sent_fragments.lock().unwrap();
                        if let Some(session) = sent.get_mut(&session_id) {
                            session.route = new_route.clone();
                            session.banned_nodes = banned_nodes.clone();
                            session.last_activity = Instant::now();
                        } else { return; }
                    }

                    let header = SourceRoutingHeader::new(new_route, 1);
                    let pkt = Packet::new_fragment(header, session_id, fragment.clone());
                    let res = self.send_packet(pkt, None);
                    self.send_logging(format!("Retransmission result: {:?}", res));

                } else {
                    self.send_logging(format!("No alternative route found, flooding..."));
                    let now = Instant::now();

                    if now.duration_since(self.last_nack_flood) >= self.nack_flood_backoff {
                        self.initiate_flood();
                        self.last_nack_flood = now;
                    } else {
                        self.send_logging(format!("Flood backoff not expired"));
                    }
                }
            }

            NackType::DestinationIsDrone => {
                self.send_logging(format!("Destination is drone, dropping fragment."));
            }
        }
    }

    // fn handle_nack(&mut self, nack: Nack, nack_generator: NodeId, session_id: u64) {
    //     let host = self.id;
    //     let frag_idx = nack.fragment_index;
    //
    //     // 1) Apro il lock TANTO PER PRENDERE fragment, destination e route correnti,
    //     //    quindi droppo subito il lock.
    //     let (fragment, current_route, destination) = {
    //         let mut sent = self.sent_fragments.lock().unwrap();
    //         if let Some(session) = sent.get_mut(&session_id) {
    //             match session.buffer.get(frag_idx as usize) {
    //                 Some(Some(f)) => {
    //                     let frag_clone = f.clone();
    //                     let route_clone = session.route.clone();
    //                     let dest = session.destination;
    //                     (frag_clone, route_clone, dest)
    //                 }
    //                 Some(None) => {
    //                     // frammento già confermato o non esistente
    //                     self.send_logging(format!(
    //                         "NACK per frammento inesistente/già confermato host={} session={} fragment={}",
    //                         host, session_id, frag_idx
    //                     ));
    //                     return;
    //                 }
    //                 None => {
    //                     // indice fuori range
    //                     self.send_logging(format!(
    //                         "NACK index out of range host={} session={} fragment={}",
    //                         host, session_id, frag_idx
    //                     ));
    //                     return;
    //                 }
    //             }
    //         } else {
    //             // sessione sconosciuta
    //             self.send_logging(format!(
    //                 "NACK per sessione sconosciuta host={} session={}",
    //                 host, session_id
    //             ));
    //             return;
    //         }
    //     };
    //     // Qui il MutexGuard su sent_fragments è già stato droppato.
    //
    //     match nack.nack_type {
    //         NackType::Dropped => {
    //             // 2) NACK “Dropped”: ritrasmetto subito il frammento
    //             self.send_logging(format!(
    //                 "Retransmitting dropped fragment host={} session={} fragment={}",
    //                 host, session_id, frag_idx
    //             ));
    //             self.send_fragment(session_id, fragment.clone());
    //             self.network.pdr_from_nack(nack_generator);
    //         }
    //
    //         NackType::ErrorInRouting(bad) | NackType::UnexpectedRecipient(bad) => {
    //             self.send_logging(format!(
    //                 "Routing error, attempting alternate route host={} session={} fragment={} bad_hop={}",
    //                 host, session_id, frag_idx, bad
    //             ));
    //
    //             // 3) Provo subito una rotta alternativa sulla route corrente
    //             if let Some(new_route) = self.network.find_alternate_route(host, destination, &current_route) {
    //                 self.send_logging(format!(
    //                     "Alternate route found, updating and retransmitting host={} session={} fragment={} alt_route={:?}",
    //                     host, session_id, frag_idx, new_route
    //                 ));
    //
    //                 // 3.a) Apro il lock solo per aggiornare session.route
    //                 {
    //                     let mut sent = self.sent_fragments.lock().unwrap();
    //                     if let Some(session) = sent.get_mut(&session_id) {
    //                         session.route = new_route.clone();
    //                         session.last_activity = Instant::now();
    //                     } else {
    //                         // La sessione è già stata rimossa nel frattempo: esco
    //                         self.send_logging(format!(
    //                             "Session {} già rimossa, non ritrasmetto",
    //                             session_id
    //                         ));
    //                         return;
    //                     }
    //                 } // il lock viene droppato qui
    //
    //                 // 3.b) Invio il frammento sulla nuova rotta
    //                 //current_route = new_route.clone();
    //                 let header = SourceRoutingHeader::new(new_route, 1);
    //                 let pkt = Packet::new_fragment(header, session_id, fragment.clone());
    //                 let res = self.send_packet(pkt, None);
    //                 self.send_logging(format!(
    //                     "Retransmission result host={} session={} fragment={} result={:?}",
    //                     host, session_id, frag_idx, res
    //                 ));
    //             } else {
    //                 let now = Instant::now();
    //                 if now.duration_since(self.last_nack_flood) >= self.nack_flood_backoff {
    //                     self.send_logging("NACK backoff scaduto: avvio flood".to_string());
    //                     self.initiate_flood();
    //                     if let Some(new_route) = self.network.find_best_route(host, destination) {
    //                         self.send_logging(format!(
    //                             "route found after flooding, updating and retransmitting host={} session={} fragment={} route={:?}",
    //                             host, session_id, frag_idx, new_route
    //                         ));
    //
    //                         // 3.a) Apro il lock solo per aggiornare session.route
    //                         {
    //                             let mut sent = self.sent_fragments.lock().unwrap();
    //                             if let Some(session) = sent.get_mut(&session_id) {
    //                                 session.route = new_route.clone();
    //                                 session.last_activity = Instant::now();
    //                             } else {
    //                                 // La sessione è già stata rimossa nel frattempo: esco
    //                                 self.send_logging(format!(
    //                                     "Session {} già rimossa, non ritrasmetto",
    //                                     session_id
    //                                 ));
    //                                 return;
    //                             }
    //                         } // il lock viene droppato qui
    //
    //                         // 3.b) Invio il frammento sulla nuova rotta
    //                         //current_route = new_route.clone();
    //                         let header = SourceRoutingHeader::new(new_route, 0);
    //                         let pkt = Packet::new_fragment(header, session_id, fragment.clone());
    //                         let res = self.send_packet(pkt, None);
    //                         self.send_logging(format!(
    //                             "Retransmission result host={} session={} fragment={} result={:?}",
    //                             host, session_id, frag_idx, res
    //                         ));
    //                     }else {
    //                         self.send_logging(format!(
    //                             "❌ NET DISCONNECTED, session {} failed --> aborting flood!",
    //                             session_id,
    //
    //                         ));
    //                     }
    //                     self.last_nack_flood = now;
    //                 } else {
    //                     self.send_logging(format!(
    //                         "NACK ricevuto ma backoff non scaduto ({:.1}s rimanenti)",
    //                         (self.nack_flood_backoff - now.duration_since(self.last_nack_flood)).as_secs_f32()
    //                     ));
    //                 }
    //             }
    //         }
    //
    //         NackType::DestinationIsDrone => {
    //             self.send_logging(format!(
    //                 "DestinationIsDrone NACK — dropping fragment host={} session={} fragment={}",
    //                 host, session_id, frag_idx
    //             ));
    //         }
    //     }
    // }

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
                    banned_nodes: HashSet::new()
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
    FilesList(ServerId),
    File(ServerId, FileId),

    MediaList(ServerId),
    Media(ServerId,MediaId),

    //chat-server
    RegistrationToChat(ServerId),
    ClientList(ServerId),
    MessageFor(ClientId,ServerId, String),
    FileFor(ClientId,ServerId, FileId, Extension, Vec<u8>),
    MediaFor(ClientId,ServerId, MediaId, Extension, Image),
}
pub enum HostCommand {
    AddSender(NodeId, Sender<Packet>),
    RemoveSender(NodeId),
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerToClient{
    ServerType(ServerTypes),

    //content server
    FilesList(ServerId, Vec<FileId>),
    File(ServerId, FileId, Extension, Vec<u8>),

    MediaList(ServerId, Vec<MediaId>),
    Media(ServerId, MediaId, Extension, Image),

    //chat server
    ClientList(ServerId, Vec<ClientId>),
    MessageFrom(ServerId, ClientId, String),
    FileFrom(ServerId, ClientId, FileId, Extension, Vec<u8>),
    MediaFrom(ServerId, ClientId, MediaId, Extension, Image),

    ErrNotFound(ServerId),
    ErrClientId(ServerId),
    WrongServer(ServerId)
}
/// Application events that comes to the SimulationController
pub enum HostEvent {
    /// Rebuilt entire high level message(es. from server to client)
    PacketReconstructed(ServerToClient),
    /// I report sending a low level packet (to be wrapped in the controller if needed)
    PacketSent(Packet),
    ///Servers discovered, response at ServerType
    ServersDiscovered(HashMap<ServerTypes,Vec<ServerId>>)
}

#[derive(Hash, Eq,Clone, PartialEq, Debug,Deserialize, Serialize)]
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
