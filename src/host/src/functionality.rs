// File: src/functionality.rs

use std::collections::HashSet;
use std::time::Instant;
use crossbeam_channel::Sender;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Ack, Fragment, Nack, NackType, Packet, FRAGMENT_DSIZE};
use crate::FloodControl;
use crate::host::{Host, HostEvent, Message, SendingCodes};
use crate::logging::{LogControl, LogType, LogModule, LogEntry};
use crate::session::SentSessionData;
use crate::logging::InlineLogger;

pub trait Functionality {
    fn send_packet(&self, packet: Packet, channel: Option<&Sender<Packet>>) -> SendingCodes;
    fn sendto_controller(&self, packet: Packet) -> Result<(), String>;
    fn handle_ack(&mut self, ack: Ack, hops: &[NodeId], session_id: u64);
    fn handle_nack(&mut self, nack: Nack, generator: NodeId, session_id: u64);
    fn build_msg_fragment<F>(&mut self, destination: NodeId, message: Message, on_fragment_sent: F) -> Result<(), String>
    where F: FnMut(usize);
    fn build_packet_ack(&self, packet: Packet, fragment_index: Option<u64>) -> Packet;
    fn get_fragment(&self, session_id: u64, fragment_index: u64) -> Option<Fragment>;
    fn send_fragment(&self, session_id: u64, fragment: Fragment);
}

impl Functionality for Host {
    fn send_packet(&self, packet: Packet, channel: Option<&Sender<Packet>>) -> SendingCodes {
        let next = channel.or_else(|| {
            self.packet_send
                .get(&packet.routing_header.hops[packet.routing_header.hop_index])
        });

        match next {
            Some(ch) => match ch.send(packet.clone()) {
                Ok(_) => {
                    let _ = self.sendto_controller(packet.clone());
                    SendingCodes::SuccessfullySent(packet.session_id)
                }
                Err(err) => {
                    let _ = self.sendto_controller(packet);
                    SendingCodes::ErrorSending(err.to_string())
                }
            },
            None => {
                let _ = self.sendto_controller(packet);
                SendingCodes::NoNextHop(format!("No valid channel for host {}", self.id))
            }
        }
    }

    fn sendto_controller(&self, packet: Packet) -> Result<(), String> {
        self.controller_send
            .send(HostEvent::PacketSent(packet))
            .map_err(|e| e.to_string())
    }

    fn handle_ack(&mut self, ack: Ack, hops: &[NodeId], session_id: u64) {

        let logger = InlineLogger(&self.log_sender);

        let drone_hops: Vec<_> = hops
            .iter()
            .copied()
            .filter(|nid| matches!(self.network.nodes.get(nid).map(|(ty, _)| ty), Some(wg_2024::packet::NodeType::Drone)))
            .collect();

        self.network.pdr_helper(&drone_hops, &logger);

        if let Some(session) = self.sent_fragments.lock().unwrap().get_mut(&session_id) {
            let dest = session.destination;
            if let Some(new_route) = self.network.find_best_route(self.id, dest, &logger) {
                session.route = new_route;
                session.last_activity = Instant::now();
            }
            let idx = ack.fragment_index as usize;
            if idx < session.buffer.len() {
                session.buffer[idx] = None;
                session.pending = session.pending.saturating_sub(1);
                session.last_activity = Instant::now();
            }
        }
    }

    fn handle_nack(&mut self, nack: Nack, generator: NodeId, session_id: u64) {

        let logger = InlineLogger(&self.log_sender);

        let (frag, route, dest, mut banned) = {
            let sent = self.sent_fragments.lock().unwrap();
            match sent.get(&session_id).and_then(|s| s.buffer.get(nack.fragment_index as usize)) {
                Some(Some(f)) => (f.clone(), sent[&session_id].route.clone(), sent[&session_id].destination, sent[&session_id].banned_nodes.clone()),
                _ => return,
            }
        };

        match nack.nack_type {
            NackType::Dropped => {
                self.send_fragment(session_id, frag);
                self.network.pdr_from_nack(generator, &logger);
            }
            NackType::ErrorInRouting(bad) | NackType::UnexpectedRecipient(bad) => {
                if !route.contains(&bad) {
                    self.send_fragment(session_id, frag);
                    return;
                }
                banned.insert(bad);
                if let Some(new_route) = self.network.find_best_route_excluding(self.id, dest, &banned, &logger) {
                    if let Some(sess) = self.sent_fragments.lock().unwrap().get_mut(&session_id) {
                        sess.route = new_route.clone();
                        sess.banned_nodes = banned.clone();
                        sess.last_activity = Instant::now();
                    }
                    let header = SourceRoutingHeader::new(new_route.clone(), 1);
                    let pkt = Packet::new_fragment(header, session_id, frag);
                    let _ = self.send_packet(pkt, None);
                } else {
                    self.initiate_flood();
                }
            }
            NackType::DestinationIsDrone => {
                let _ = self.log_sender.send(LogEntry {
                    log_type: LogType::WARN,
                    module: LogModule::NACK,
                    message: "Destination is a drone, dropping fragment".into(),
                });
            }
        }
    }

    fn build_msg_fragment<F>(&mut self, destination: NodeId, message: Message, mut on_fragment_sent: F) -> Result<(), String>
    where F: FnMut(usize) {


        let logger = InlineLogger(&self.log_sender);

        let route = self.network.find_best_route(self.id, destination, &logger)
            .ok_or_else(|| format!("No route to {}", destination))?;

        let bytes = &message.content;
        let total = (bytes.len() + FRAGMENT_DSIZE - 1) / FRAGMENT_DSIZE;

        {
            let mut sent = self.sent_fragments.lock().map_err(|_| "Mutex poisoned")?;
            let entry = sent.entry(message.session_id).or_insert_with(|| SentSessionData {
                buffer: vec![None; total],
                last_activity: Instant::now(),
                route: route.clone(),
                destination,
                pending: total,
                banned_nodes: HashSet::new(),
            });
            entry.pending = total;
            entry.last_activity = Instant::now();
            if entry.buffer.len() < total {
                entry.buffer.resize(total, None);
            }
        }

        for (i, chunk) in bytes.chunks(FRAGMENT_DSIZE).enumerate() {
            let frag = Fragment {
                fragment_index: i as u64,
                total_n_fragments: total as u64,
                length: chunk.len() as u8,
                data: {
                    let mut buf = [0; FRAGMENT_DSIZE];
                    buf[..chunk.len()].copy_from_slice(chunk);
                    buf
                },
            };
            {
                let mut sent = self.sent_fragments.lock().unwrap();
                sent.get_mut(&message.session_id).unwrap().buffer[i] = Some(frag.clone());
            }
            let header = SourceRoutingHeader::new(route.clone(), 1);
            let pkt = Packet::new_fragment(header, message.session_id, frag.clone());
            let _ = self.send_packet(pkt, None);
            on_fragment_sent(chunk.len());
        }

        Ok(())
    }

    fn build_packet_ack(&self, packet: Packet, fragment_index: Option<u64>) -> Packet {
        let mut header = packet.routing_header;
        header.reverse();
        Packet::new_ack(header, packet.session_id, fragment_index.unwrap_or(0))
    }

    fn get_fragment(&self, session_id: u64, fragment_index: u64) -> Option<Fragment> {
        self.sent_fragments.lock().unwrap()
            .get(&session_id)
            .and_then(|s| s.buffer.get(fragment_index as usize).cloned().flatten())
    }

    fn send_fragment(&self, session_id: u64, fragment: Fragment) {
        let route = {
            let mut sent = self.sent_fragments.lock().unwrap();
            if let Some(sess) = sent.get_mut(&session_id) {
                sess.last_activity = Instant::now();
                sess.route.clone()
            } else {
                return;
            }
        };
        let header = SourceRoutingHeader::new(route.clone(), 1);
        let pkt = Packet::new_fragment(header, session_id, fragment);
        let _ = self.send_packet(pkt, None);
    }
}
