// File: src/network.rs

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use ordered_float::OrderedFloat;
use wg_2024::network::NodeId;
use wg_2024::packet::NodeType;
use crate::logging::{InlineLogger, LogControl, LogType, LogModule};

const ALPHA: f64 = 0.01;

#[derive(Clone)]
pub struct Network {
    pub servers: HashSet<NodeId>,
    pub nodes: HashMap<NodeId, (NodeType, Vec<Edge>)>,
    pub drone_pdr: HashMap<NodeId, f64>,
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub to: NodeId,
    pub weight: f64,
}

impl Network {
    pub fn new() -> Self {
        Self {
            servers: HashSet::new(),
            nodes: HashMap::new(),
            drone_pdr: HashMap::new(),
        }
    }

    pub fn recalc_pdr(&mut self, logger: &dyn LogControl) {
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
                        format!("Recalculated weight for drone {} to {:.3}", drone_id, clamped),
                    );
                }
            }
        }
    }

    pub fn pdr_helper(&mut self, hops: &[NodeId], logger: &dyn LogControl) {
        for &drone_id in hops {
            let old = self.drone_pdr.entry(drone_id).or_insert(0.0);
            let new_pdr = ALPHA * 0.0 + (1.0 - ALPHA) * *old;
            *old = new_pdr.clamp(0.0, 1.0);
            logger.send_logging(
                LogType::DEBUG,
                LogModule::NETWORK,
                format!("Drone {} PDR updated to {:.3}", drone_id, new_pdr),
            );
        }
        self.recalc_pdr(logger);
    }

    pub fn pdr_from_nack(&mut self, node_id: NodeId, logger: &dyn LogControl) {
        let old = self.drone_pdr.entry(node_id).or_insert(0.0);
        let new_pdr = ALPHA * 1.0 + (1.0 - ALPHA) * *old;
        *old = new_pdr.clamp(0.0, 1.0);
        logger.send_logging(
            LogType::INFO,
            LogModule::NETWORK,
            format!("Drone {} PDR penalized to {:.3} due to NACK", node_id, new_pdr),
        );
        self.recalc_pdr(logger);
    }

    pub fn find_best_route(
        &self,
        start: NodeId,
        destination: NodeId,
        logger: &dyn LogControl,
    ) -> Option<Vec<NodeId>> {
        self.find_best_route_excluding(start, destination, &HashSet::new(), logger)
    }

    pub fn find_best_route_excluding(
        &self,
        start: NodeId,
        destination: NodeId,
        banned_nodes: &HashSet<NodeId>,
        logger: &dyn LogControl,
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
            format!("Route from {} to {} found: {:?}", start, destination, path),
        );

        Some(path)
    }
}