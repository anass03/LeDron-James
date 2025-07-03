use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use ordered_float::OrderedFloat;
use wg_2024::network::NodeId;
use wg_2024::packet::NodeType;
use crate::logging::{LogControl, LogType, LogModule};

const ALPHA: f64 = 0.01; // smoothing factor for EWMA

/// Represents the network topology and maintains PDR estimates for drone links.
#[derive(Clone)]
pub struct Network {
    /// Set of content/chat servers discovered via flooding
    pub servers: HashSet<NodeId>,

    /// Map of each node → (its type, list of outgoing edges)
    pub nodes: HashMap<NodeId, (NodeType, Vec<Edge>)>,

    /// Exponential moving average of PDR for each drone node
    pub drone_pdr: HashMap<NodeId, f64>,
}

/// An outgoing network edge with an associated routing cost.
#[derive(Clone, Debug)]
pub struct Edge {
    pub to: NodeId,
    pub weight: f64,
}

impl Network {
    /// Constructs a new, empty network.
    pub fn new() -> Self {
        Self {
            servers: HashSet::new(),
            nodes: HashMap::new(),
            drone_pdr: HashMap::new(),
        }
    }

    /// Recalculates the weight of every outgoing edge from each drone
    /// based on the latest PDR estimate.
    ///
    /// # Logging
    /// - DEBUG at start and end of recalculation.
    pub fn recalc_pdr(&mut self) {
        for (&drone_id, &pdr) in &self.drone_pdr {
            if let Some((ty, edges)) = self.nodes.get_mut(&drone_id) {
                if *ty == NodeType::Drone {
                    let clamped = pdr.clamp(0.0, 1.0);
                    for edge in edges.iter_mut() {
                        edge.weight = clamped;
                    }
                }
            }
        }
    }

    /// Updates the PDR estimate for each drone in `hops` after a successful forward.
    ///
    /// EWMA formula: new_pdr = α * 0.0 + (1 − α) * old_pdr
    ///
    /// # Logging
    /// - DEBUG for each updated drone.
    pub fn pdr_helper(&mut self, hops: &[NodeId]) {
        for &drone_id in hops {
            let old = self.drone_pdr.entry(drone_id).or_insert(0.0);
            let new_pdr = ALPHA * 0.0 + (1.0 - ALPHA) * *old;
            *old = new_pdr.clamp(0.0, 1.0);
        }
        // Recompute edge weights with updated PDRs
        self.recalc_pdr();
    }

    /// Penalizes the PDR estimate for `node_id` after receiving a NACK drop.
    ///
    /// EWMA formula: new_pdr = α * 1.0 + (1 − α) * old_pdr
    ///
    /// # Logging
    /// - INFO when penalizing a drone link.
    // fn process_flood_response(&mut self, mut pkt: Packet) {
    //         // 1) Estraggo il FloodResponse e l’header di routing
    //         let (response, routing_header) = match &mut pkt.pack_type {
    //             PacketType::FloodResponse(resp) => (resp, &mut pkt.routing_header),
    //             _ => {
    //                 // Non dovrebbe succedere: sto chiamando questa funzione
    //                 // solo per FloodResponse. Se capita, esco subito.
    //                 return;
    //             }
    //         };
    //
    //         let host_id = self.id;
    //         let flood_id = response.flood_id;
    //
    //         // 2) Controllo che path_trace non sia vuoto
    //         if response.path_trace.is_empty() {
    //             self.send_logging(format!("FloodResponse senza path_trace – host={}", host_id));
    //             return;
    //         }
    //         self.send_logging(format!("👌👌👌 path trace da controllare, {:?}", response.path_trace));
    //         // 3) The last element in path_trace is the initiator
    //         let (initiator, _) = response.path_trace.first().unwrap();
    //
    //         // 4) If I'm not the initiator, forward the response back along the path
    //         if *initiator != host_id {
    //             routing_header.hop_index += 1;
    //             if routing_header.hop_index < routing_header.hops.len() {
    //                 let next_hop = routing_header.hops[routing_header.hop_index];
    //                 self.send_logging(format!(
    //                     "Forwarding FloodResponse: host={} flood_id={} -> next_hop={}",
    //                     host_id,
    //                     response.flood_id,
    //                     next_hop
    //                 ));
    //                 let _ = self.send_packet(pkt.clone(), None);
    //             } else {
    //                 self.send_logging(format!(
    //                     "Invalid hop_index in FloodResponse: host={} hop_index={} len={}",
    //                     host_id,
    //                     routing_header.hop_index,
    //                     routing_header.hops.len()
    //                 ));
    //             }
    //             return;
    //         }
    //
    //         // ◁ Se arrivo qui, “sono” l’iniziatore (e flood_id ∈ flood_cache[self.id]):
    //         //    posso far partire la logica che registra i nodi e gli archi
    //         self.send_logging(format!(
    //             "Handling FloodResponse per me host={} flood_id={} path={:?}",
    //             host_id,
    //             flood_id,
    //             response
    //                 .path_trace
    //                 .iter()
    //                 .map(|(nid, ty)| format!("{:?}({})", ty, nid))
    //                 .collect::<Vec<_>>()
    //                 .join(" -> ")
    //         ));
    //
    //         let path = &response.path_trace;
    //         if path.len() < 2 {
    //             self.send_logging(format!(
    //                 "FloodResponse path troppo corto, host={} len={}",
    //                 host_id,
    //                 path.len()
    //             ));
    //             return;
    //         }
    //
    //         // 5) Registro tutti i nodi nella mia adjacency-list
    //         for &(node_id, node_type) in path {
    //             let entry = self
    //                 .network
    //                 .nodes
    //                 .entry(node_id)
    //                 .or_insert_with(|| (node_type, Vec::new()));
    //             // inizializzo pdr se è drone
    //             if entry.0 == NodeType::Drone {
    //                 self.network.drone_pdr.entry(node_id).or_insert(0.0);
    //             }
    //             // se è server, lo aggiungo all’insieme
    //             if entry.0 == NodeType::Server {
    //                 self.network.servers.insert(node_id);
    //             }
    //         }
    //
    //         // 6) Registro gli archi tra nodi consecutivi
    //         for window in path.windows(2) {
    //             let (curr_id, _) = window[0];
    //             let (next_id, _) = window[1];
    //             // aggiungo arco curr_id → next_id
    //             if let Some((_, edges)) = self.network.nodes.get_mut(&curr_id) {
    //                 if !edges.iter().any(|e| e.to == next_id) {
    //                     edges.push(Edge { to: next_id, weight: 0.0 });
    //                     self.send_logging(format!(
    //                         "Recorded adjacency edge host={} da {} a {}",
    //                         host_id, curr_id, next_id
    //                     ));
    //                 }
    //             }
    //             // aggiungo anche l’arco inverso next_id → curr_id
    //             if let Some((_, inv_edges)) = self.network.nodes.get_mut(&next_id) {
    //                 if !inv_edges.iter().any(|e| e.to == curr_id) {
    //                     inv_edges.push(Edge { to: curr_id, weight: 0.0 });
    //                     self.send_logging(format!(
    //                         "Recorded adjacency edge host={} da {} a {}",
    //                         host_id, next_id, curr_id
    //                     ));
    //                 }
    //             }
    //         }
    //
    //         // 7) (Opzionale) Ricalcolo subito i PDR
    //         self.network.recalc_pdr();
    //     }
    pub fn pdr_from_nack(&mut self, node_id: NodeId) {
        let old = self.drone_pdr.entry(node_id).or_insert(0.0);
        let new_pdr = ALPHA * 1.0 + (1.0 - ALPHA) * *old;
        *old = new_pdr.clamp(0.0, 1.0);
        self.recalc_pdr();
    }

    /// Finds the path from `start` to `destination` that maximizes delivery probability,
    /// equivalently minimizing the sum of costs `-ln(1 - pdr)` along the route.
    ///
    /// Returns `Some(path)` if a route exists, or `None` otherwise.
    ///
    /// # Logging
    /// - INFO when a valid route is found.
    /// - WARN if no route exists.
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
            if cost_u > *dist.get(&u).unwrap_or(&f64::INFINITY) {
                continue;
            }
            if u == destination {
                break;
            }
            if let Some((_, edges)) = self.nodes.get(&u) {
                for edge in edges {
                    let v = edge.to;
                    // If not start or destination, v must be a drone node
                    if v != start && v != destination {
                        match self.nodes.get(&v) {
                            Some((NodeType::Drone, _pdr)) => {  }
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
                    let cost = -reliability.ln();
                    let next_cost = cost_u + cost;
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

        // Reconstruct path
        let mut path = Vec::new();
        let mut curr = destination;
        while curr != start {
            path.push(curr);
            curr = prev[&curr];
        }
        path.push(start);
        path.reverse();

        Some(path)
    }

    /// Like `find_best_route`, but excludes any nodes in `banned_nodes`.
    ///
    /// # Logging
    /// - INFO when an alternative route is found.
    /// - WARN if no route exists under the exclusion constraints.
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
            if cost_u > *dist.get(&u).unwrap_or(&f64::INFINITY) {
                continue;
            }
            if u == destination {
                break;
            }
            if let Some((_, edges)) = self.nodes.get(&u) {
                for edge in edges {
                    let v = edge.to;
                    if banned_nodes.contains(&v) {
                        continue;
                    }
                    if v != start && v != destination {
                        match self.nodes.get(&v) {
                            Some((NodeType::Drone, _pdr)) => {  }
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
                    let cost = -reliability.ln();
                    let next_cost = cost_u + cost;
                    if next_cost < *dist.get(&v).unwrap_or(&f64::INFINITY) {
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

        // Reconstruct alternative path
        let mut path = Vec::new();
        let mut curr = destination;
        while curr != start {
            path.push(curr);
            curr = prev[&curr];
        }
        path.push(start);
        path.reverse();

        Some(path)
    }
}
