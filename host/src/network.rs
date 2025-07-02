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
    pub fn recalc_pdr<L: LogControl>(&mut self, logger: &L) {
        logger.send_logging(
            LogType::DEBUG,
            LogModule::NETWORK,
            "Starting PDR weights recalculation for drone edges".to_string(),
        );

        for (&drone_id, &pdr) in &self.drone_pdr {
            if let Some((ty, edges)) = self.nodes.get_mut(&drone_id) {
                if *ty == NodeType::Drone {
                    let clamped = pdr.clamp(0.0, 1.0);
                    for edge in edges.iter_mut() {
                        edge.weight = clamped;
                    }
                    logger.send_logging(
                        LogType::DEBUG,
                        LogModule::NETWORK,
                        format!(
                            "  • Drone {}: set outgoing edge weight to {:.3}",
                            drone_id, clamped
                        ),
                    );
                }
            }
        }

        logger.send_logging(
            LogType::INFO,
            LogModule::NETWORK,
            "Completed PDR weights recalculation".to_string(),
        );
    }

    /// Updates the PDR estimate for each drone in `hops` after a successful forward.
    ///
    /// EWMA formula: new_pdr = α * 0.0 + (1 − α) * old_pdr
    ///
    /// # Logging
    /// - DEBUG for each updated drone.
    pub fn pdr_helper<L: LogControl>(&mut self, hops: &[NodeId], logger: &L) {
        for &drone_id in hops {
            let old = self.drone_pdr.entry(drone_id).or_insert(0.0);
            let new_pdr = ALPHA * 0.0 + (1.0 - ALPHA) * *old;
            *old = new_pdr.clamp(0.0, 1.0);
            logger.send_logging(
                LogType::DEBUG,
                LogModule::NETWORK,
                format!(
                    "PDR helper: drone {} updated from {:.3} to {:.3}",
                    drone_id, *old, new_pdr
                ),
            );
        }
        // Recompute edge weights with updated PDRs
        self.recalc_pdr(logger);
    }

    /// Penalizes the PDR estimate for `node_id` after receiving a NACK drop.
    ///
    /// EWMA formula: new_pdr = α * 1.0 + (1 − α) * old_pdr
    ///
    /// # Logging
    /// - INFO when penalizing a drone link.
    pub fn pdr_from_nack<L: LogControl>(&mut self, node_id: NodeId, logger: &L) {
        let old = self.drone_pdr.entry(node_id).or_insert(0.0);
        let new_pdr = ALPHA * 1.0 + (1.0 - ALPHA) * *old;
        *old = new_pdr.clamp(0.0, 1.0);
        logger.send_logging(
            LogType::INFO,
            LogModule::NETWORK,
            format!("PDR penalized for drone {} to {:.3} after NACK", node_id, new_pdr),
        );
        self.recalc_pdr(logger);
    }

    /// Finds the path from `start` to `destination` that maximizes delivery probability,
    /// equivalently minimizing the sum of costs `-ln(1 - pdr)` along the route.
    ///
    /// Returns `Some(path)` if a route exists, or `None` otherwise.
    ///
    /// # Logging
    /// - INFO when a valid route is found.
    /// - WARN if no route exists.
    pub fn find_best_route<L: LogControl>(
        &self,
        start: NodeId,
        destination: NodeId,
        logger: &L,
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
                        if let Some((NodeType::Drone, _)) = self.nodes.get(&v) {
                            // ok
                        } else {
                            continue;
                        }
                    }
                    let pdr_v = *self.drone_pdr.get(&v).unwrap_or(&0.0);
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
            logger.send_logging(
                LogType::WARN,
                LogModule::NETWORK,
                format!("No route found from {} to {}", start, destination),
            );
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

        logger.send_logging(
            LogType::INFO,
            LogModule::NETWORK,
            format!("Route {} → {} found: {:?}", start, destination, path),
        );
        Some(path)
    }

    /// Like `find_best_route`, but excludes any nodes in `banned_nodes`.
    ///
    /// # Logging
    /// - INFO when an alternative route is found.
    /// - WARN if no route exists under the exclusion constraints.
    pub fn find_best_route_excluding<L: LogControl>(
        &self,
        start: NodeId,
        destination: NodeId,
        banned_nodes: &HashSet<NodeId>,
        logger: &L,
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
                        if let Some((NodeType::Drone, _)) = self.nodes.get(&v) {
                            // ok
                        } else {
                            continue;
                        }
                    }
                    let pdr_v = *self.drone_pdr.get(&v).unwrap_or(&0.0);
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
            logger.send_logging(
                LogType::WARN,
                LogModule::NETWORK,
                format!(
                    "No route found from {} to {} excluding {:?}",
                    start, destination, banned_nodes
                ),
            );
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

        logger.send_logging(
            LogType::INFO,
            LogModule::NETWORK,
            format!(
                "Alternative route {} → {} excluding {:?}: {:?}",
                start, destination, banned_nodes, path
            ),
        );
        Some(path)
    }
}
