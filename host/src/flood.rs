use crate::functionality::Functionality;
use crate::logging::{LogControl, LogModule, LogType};
use crate::network::Edge;
use crate::session::generate_unique_session_id;
use crate::Host;
use rand::prelude::ThreadRng;
use rand::Rng;
use wg_2024::network::SourceRoutingHeader;
use wg_2024::packet::{FloodRequest, NodeType, Packet, PacketType};

/// Flood control operations for topology discovery.
pub trait FloodControl {
    /// Begins a new flood discovery from this host.
    fn initiate_flood(&mut self);

    /// Handles an incoming flood request packet.
    fn process_flood_request(&mut self, flood_request: FloodRequest, packet: Packet);

    /// Handles an incoming flood response packet.
    fn process_flood_response(&mut self, packet: Packet);
}
impl FloodControl for Host {
    /// Clears previous state and sends a FloodRequest to all neighbors.
    ///
    /// # Logging
    /// - DEBUG when clearing state
    /// - WARN if no neighbors exist
    /// - INFO on flood start
    /// - DEBUG for each packet send and its result
    fn initiate_flood(&mut self) {
        // clear previous discovery state
        self.send_logging(
            LogType::INFO,
            LogModule::FLOOD,
            "Clearing previous flood cache and topology".to_string(),
        );
        self.network.nodes.clear();
        self.network.drone_pdr.clear();
        self.flood_cache.clear();

        // abort if no neighbors
        if self.packet_send.is_empty() {
            self.send_logging(
                LogType::WARN,
                LogModule::FLOOD,
                "No neighbors available, aborting flood discovery".to_string(),
            );
            return;
        }

        // generate IDs
        let session_id = generate_unique_session_id();
        let flood_id = generate_unique_flood_id();
        let host_id = self.id;

        // mark as seen
        self.flood_cache.insert(host_id, vec![flood_id]);

        // build request
        let header = SourceRoutingHeader::new(vec![], 0);
        let request = Packet::new_flood_request(
            header,
            session_id,
            FloodRequest::initialize(flood_id, host_id, self.type_host),
        );

        self.send_logging(
            LogType::INFO,
            LogModule::FLOOD,
            format!(
                "Starting flood discovery: flood({}), session({})",
                flood_id, session_id
            ),
        );

        // send to all neighbors
        for (&neighbor, chan) in &self.packet_send {
            self.send_logging(
                LogType::INFO,
                LogModule::FLOOD,
                format!("Sending FloodRequest to neighbor node({})", neighbor),
            );
            let result = self.send_packet(request.clone(), Some(chan));
            self.send_logging(
                LogType::DEBUG,
                LogModule::FLOOD,
                format!(
                    "FloodRequest send result: neighbor({}) result={:?}",
                    neighbor, result
                ),
            );
        }
    }

    /// Processes a FloodRequest:
    /// - if duplicate, responds immediately
    /// - otherwise, records flood_id and forwards (or responds at leaf)
    ///
    /// # Logging
    /// - INFO on duplicate detection and immediate response
    /// - DEBUG on path trace updates and forwarding decisions
    fn process_flood_request(&mut self, req: FloodRequest, pkt: Packet) {
        let flood_id = req.flood_id;
        let initiator = req.initiator_id;
        let session = pkt.session_id;
        let host_id = self.id;

        self.send_logging(
            LogType::DEBUG,
            LogModule::FLOOD,
            format!(
                "Received FloodRequest: initiator({}), flood_id({}), session({})",
                initiator, flood_id, session
            ),
        );

        let mut fr = req.clone();
        let seen = self.flood_cache.entry(initiator).or_default();

        // duplicate?
        if seen.contains(&flood_id) {
            self.send_logging(
                LogType::INFO,
                LogModule::FLOOD,
                "Duplicate flood detected, sending immediate response".to_string(),
            );
            fr.increment(host_id, self.type_host);
            let mut resp = fr.generate_response(session);
            resp.routing_header.hop_index += 1;
            let next = resp.routing_header.hops[resp.routing_header.hop_index];
            self.send_logging(
                LogType::DEBUG,
                LogModule::FLOOD,
                format!("Responding to duplicate: next_hop={}", next),
            );
            let _ = self.send_packet(resp, None);
            return;
        }

        // first time seeing this flood
        seen.push(flood_id);

        // determine incoming neighbor
        let from = if let Some(&(nid, _)) = req.path_trace.last() {
            nid
        } else {
            self.send_logging(
                LogType::ERROR,
                LogModule::FLOOD,
                "Empty path_trace in FloodRequest, dropping".into(),
            );
            return;
        };

        self.send_logging(
            LogType::DEBUG,
            LogModule::FLOOD,
            format!("Path trace before increment: {:?}", fr.path_trace),
        );

        // append self
        fr.increment(host_id, self.type_host);
        self.send_logging(
            LogType::DEBUG,
            LogModule::FLOOD,
            format!("Path trace after increment: {:?}", fr.path_trace),
        );

        // forward to others or respond if leaf
        let next_neighbors: Vec<_> = self
            .packet_send
            .keys()
            .filter(|&&nid| nid != from)
            .cloned()
            .collect();

        if next_neighbors.is_empty() {
            self.send_logging(
                LogType::INFO,
                LogModule::FLOOD,
                "Leaf node reached, sending FloodResponse".to_string(),
            );
            let mut resp = fr.generate_response(session);
            resp.routing_header.hop_index += 1;
            let _ = self.send_packet(resp, None);
        } else {
            self.send_logging(
                LogType::DEBUG,
                LogModule::FLOOD,
                format!(
                    "Forwarding FloodRequest to {} neighbors",
                    next_neighbors.len()
                ),
            );
            for nbr in next_neighbors {
                self.send_logging(
                    LogType::DEBUG,
                    LogModule::FLOOD,
                    format!("Forwarding to neighbor {}", nbr),
                );
                let fwd =
                    Packet::new_flood_request(pkt.routing_header.clone(), session, fr.clone());
                let _ = self.send_packet(fwd, self.packet_send.get(&nbr));
            }
        }
    }

    /// Processes a FloodResponse:
    /// - if not the initiator, forwards back along the recorded path
    /// - otherwise, integrates the path into the local topology
    ///
    /// # Logging
    /// - WARN if path_trace is empty
    /// - INFO when final initiator handles the response
    /// - DEBUG when forwarding back
    fn process_flood_response(&mut self, mut pkt: Packet) {
        let (resp, header) = match &mut pkt.pack_type {
            PacketType::FloodResponse(r) => (r, &mut pkt.routing_header),
            _ => return, // only handle FloodResponse here
        };

        let flood_id = resp.flood_id;
        let host_id = self.id;

        if resp.path_trace.is_empty() {
            self.send_logging(
                LogType::WARN,
                LogModule::FLOOD,
                "FloodResponse with empty path_trace, dropping".to_string(),
            );
            return;
        }

        // check if we are the initiator
        let (initiator, _) = resp.path_trace.first().unwrap();
        if *initiator != host_id {
            // forward back
            header.hop_index += 1;
            if header.hop_index < header.hops.len() {
                let next = header.hops[header.hop_index];
                self.send_logging(
                    LogType::DEBUG,
                    LogModule::FLOOD,
                    format!("Forwarding FloodResponse back to {}", next),
                );
                let _ = self.send_packet(pkt.clone(), None);
            }
            return;
        }

        // we are initiator: integrate topology
        self.send_logging(
            LogType::INFO,
            LogModule::FLOOD,
            format!(
                "Handling FloodResponse as initiator: path {:?}",
                resp.path_trace
                    .iter()
                    .map(|(nid, ty)| format!("{:?}({})", ty, nid))
                    .collect::<Vec<_>>()
            ),
        );

        // register nodes and PDR entries
        for &(node_id, node_type) in &resp.path_trace {
            let entry = self
                .network
                .nodes
                .entry(node_id)
                .or_insert_with(|| (node_type, Vec::new()));
            if entry.0 == NodeType::Drone {
                self.network.drone_pdr.entry(node_id).or_insert(0.0);
            }
            if entry.0 == NodeType::Server {
                self.network.servers.insert(node_id);
            }
        }

        // record edges
        for window in resp.path_trace.windows(2) {
            let (curr_id, _) = window[0];
            let (next_id, _) = window[1];
            if let Some((_, edges)) = self.network.nodes.get_mut(&curr_id) {
                if !edges.iter().any(|e| e.to == next_id) {
                    edges.push(Edge {
                        to: next_id,
                        weight: 0.0,
                    });
                    self.send_logging(
                        LogType::DEBUG,
                        LogModule::FLOOD,
                        format!("Recorded edge {} -> {}", curr_id, next_id),
                    );
                }
            }
            if let Some((_, edges)) = self.network.nodes.get_mut(&next_id) {
                if !edges.iter().any(|e| e.to == curr_id) {
                    edges.push(Edge {
                        to: curr_id,
                        weight: 0.0,
                    });
                    self.send_logging(
                        LogType::DEBUG,
                        LogModule::FLOOD,
                        format!("Recorded edge {} -> {}", next_id, curr_id),
                    );
                }
            }
        }

        // optionally recalc PDR immediately
        self.network.recalc_pdr();
    }
}

/// Generates a new random identifier for flood sessions.
pub fn generate_unique_flood_id() -> u64 {
    let mut rng: ThreadRng = rand::rng();
    rng.gen()
}
