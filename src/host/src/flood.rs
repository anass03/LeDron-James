// File: src/flood.rs
use rand::prelude::ThreadRng;
use rand::Rng;
use wg_2024::network::SourceRoutingHeader;
use wg_2024::packet::{FloodRequest, NodeType, Packet, PacketType};
use crate::Functionality;
use crate::host::Host;
use crate::network::Edge;
use crate::session::generate_unique_session_id;
use crate::logging::{LogControl, LogType, LogModule};
use crate::logging::InlineLogger;

pub trait FloodControl {
    fn initiate_flood(&mut self);
    fn process_flood_request(&mut self, flood_request: FloodRequest, packet: Packet);
    fn process_flood_response(&mut self, packet: Packet);
}

impl FloodControl for Host {
    fn initiate_flood(&mut self) {
        self.send_logging(LogType::DEBUG, LogModule::FLOOD, "Clearing topology state".into());
        self.network.nodes.clear();
        self.network.drone_pdr.clear();
        self.flood_cache.clear();

        if self.packet_send.is_empty() {
            self.send_logging(LogType::WARN, LogModule::FLOOD, "No neighbors found".into());
            return;
        }

        let session_id = generate_unique_session_id();
        let flood_id = generate_unique_flood_id();
        let host_id = self.id;

        self.send_logging(LogType::INFO, LogModule::FLOOD, format!(
            "Initiating flood: flood_id={}, session_id={}", flood_id, session_id
        ));

        self.flood_cache.insert(host_id, vec![flood_id]);

        let header = SourceRoutingHeader::new(vec![], 0);
        let request = Packet::new_flood_request(
            header,
            session_id,
            FloodRequest::initialize(flood_id, host_id, self.type_host),
        );

        for (&neighbor, chan) in &self.packet_send {
            self.send_logging(LogType::DEBUG, LogModule::FLOOD, format!("Sending flood to {}", neighbor));
            self.send_packet(request.clone(), Some(chan));
        }
    }

    fn process_flood_request(&mut self, req: FloodRequest, pkt: Packet) {
        let flood_id = req.flood_id;
        let initiator = req.initiator_id;
        let session = pkt.session_id;

        let mut fr = req.clone();
        let seen = self.flood_cache.entry(initiator).or_default();

        if seen.contains(&flood_id) {
            self.send_logging(LogType::INFO, LogModule::FLOOD, "Duplicate flood received".into());
            fr.increment(self.id, self.type_host);
            let mut resp = fr.generate_response(session);
            resp.routing_header.hop_index += 1;
            let _ = self.send_packet(resp, None);
            return;
        }

        seen.push(flood_id);

        let from = match req.path_trace.last() {
            Some(&(nid, _)) => nid,
            None => {
                self.send_logging(LogType::WARN, LogModule::FLOOD, "Empty path trace, dropping request".into());
                return;
            }
        };

        fr.increment(self.id, self.type_host);

        let next_neighbors: Vec<_> = self
            .packet_send
            .keys()
            .filter(|&&nid| nid != from)
            .cloned()
            .collect();

        if next_neighbors.is_empty() {
            self.send_logging(LogType::INFO, LogModule::FLOOD, "Leaf node reached, sending response".into());
            let mut resp = fr.generate_response(session);
            resp.routing_header.hop_index += 1;
            let _ = self.send_packet(resp, None);
        } else {
            self.send_logging(LogType::DEBUG, LogModule::FLOOD, format!("Forwarding flood to {} neighbors", next_neighbors.len()));
            for nbr in next_neighbors {
                self.send_logging(LogType::DEBUG, LogModule::FLOOD, format!("Forwarding to {}", nbr));
                let fwd = Packet::new_flood_request(pkt.routing_header.clone(), session, fr.clone());
                let _ = self.send_packet(fwd, self.packet_send.get(&nbr));
            }
        }
    }

    fn process_flood_response(&mut self, mut pkt: Packet) {
        let (resp, header) = match &mut pkt.pack_type {
            PacketType::FloodResponse(r) => (r, &mut pkt.routing_header),
            _ => return,
        };

        if resp.path_trace.is_empty() {
            self.send_logging(LogType::WARN, LogModule::FLOOD, "Empty path trace in response".into());
            return;
        }

        let (initiator, _) = resp.path_trace.first().unwrap();
        if *initiator != self.id {
            header.hop_index += 1;
            if header.hop_index < header.hops.len() {
                self.send_logging(LogType::DEBUG, LogModule::FLOOD, format!("Forwarding response to next hop: {}", header.hops[header.hop_index]));
                let _ = self.send_packet(pkt.clone(), None);
            }
            return;
        }

        self.send_logging(LogType::INFO, LogModule::FLOOD, "Handling flood response as initiator".into());

        for &(nid, ty) in &resp.path_trace {
            self.network.nodes.entry(nid).or_insert_with(|| (ty, Vec::new()));
            if ty == NodeType::Drone {
                self.network.drone_pdr.entry(nid).or_insert(0.0);
            }
        }

        for window in resp.path_trace.windows(2) {
            let (a, _) = window[0];
            let (b, _) = window[1];
            if let Some((_, edges)) = self.network.nodes.get_mut(&a) {
                if !edges.iter().any(|e| e.to == b) {
                    edges.push(Edge { to: b, weight: 0.0 });
                }
            }
            if let Some((_, edges)) = self.network.nodes.get_mut(&b) {
                if !edges.iter().any(|e| e.to == a) {
                    edges.push(Edge { to: a, weight: 0.0 });
                }
            }
        }

        let logger = InlineLogger(&self.log_sender);
        self.network.recalc_pdr(&logger);
    }
}

pub fn generate_unique_flood_id() -> u64 {
    let mut rng: ThreadRng = rand::rng();
    rng.gen()
}
