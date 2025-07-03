use std::collections::HashSet;
use std::time::{Instant};
use crossbeam_channel::{Sender};
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Ack, Fragment, Nack, NackType, NodeType, Packet, FRAGMENT_DSIZE};
use crate::{Host, HostEvent, Message, SendingCodes};
use crate::flood::FloodControl;
use crate::logging::{LogControl, LogType, LogModule};
use crate::session::SentSessionData;

/// Core functionality for sending, fragmenting, and retransmitting packets.
pub trait Functionality {
    /// Sends a low-level `packet` on the given channel or the next hop.
    fn send_packet(&self, packet: Packet, channel: Option<&Sender<Packet>>) -> SendingCodes;

    /// Notifies the SimulationController that `packet` was (or wasn’t) sent.
    fn sendto_controller(&self, packet: Packet) -> Result<(), String>;

    /// Processes an incoming ACK: updates PDR, possibly reroutes, and marks fragments.
    fn handle_ack(&mut self, ack: Ack, hops: &[NodeId], session_id: u64);

    /// Processes an incoming NACK: retransmits or reroutes fragments as needed.
    fn handle_nack(&mut self, nack: Nack, generator: NodeId, session_id: u64);

    /// Splits `message` into `FRAGMENT_DSIZE` chunks, sends each fragment, and
    /// invokes `on_fragment_sent(bytes)` after sending each.
    fn build_msg_fragment<F>(&mut self, destination: NodeId, message: Message, on_fragment_sent: F) -> Result<(), String>
    where
        F: FnMut(usize);

    /// Constructs an ACK packet for `fragment_index` in `packet`, reversing the route.
    fn build_packet_ack(&self, packet: Packet, fragment_index: Option<u64>) -> Packet;

    /// Retrieves a previously‐sent fragment by `(session_id, fragment_index)`.
    fn get_fragment(&self, session_id: u64, fragment_index: u64) -> Option<Fragment>;

    /// Sends a single `fragment` of `session_id` along its recorded route.
    fn send_fragment(&self, session_id: u64, fragment: Fragment);
}

impl Functionality for Host {
    fn send_packet(&self, packet: Packet, channel: Option<&Sender<Packet>>) -> SendingCodes {
        // Log the attempt and route
        self.send_logging(
            LogType::DEBUG,
            LogModule::PACKET,
            format!(
                "Attempting to send packet session={} hops={:?}",
                packet.session_id, packet.routing_header.hops
            ),
        );

        // Determine the channel
        let next = channel.or_else(|| {
            self.packet_send
                .get(&packet.routing_header.hops[packet.routing_header.hop_index])
        });

        // Try sending
        let result = match next {
            Some(ch) => match ch.send(packet.clone()) {
                Ok(_) => {
                    // Notify controller
                    if let Err(e) = self.sendto_controller(packet.clone()) {
                        self.send_logging(
                            LogType::ERROR,
                            LogModule::PACKET,
                            format!("Failed to notify controller: {}", e),
                        );
                    }
                    SendingCodes::SuccessfullySent(packet.session_id)
                }
                Err(err) => {
                    let err_str = err.to_string();
                    let _ = self.sendto_controller(packet.clone());
                    SendingCodes::ErrorSending(err_str)
                }
            },
            None => {
                let msg = format!(
                    "No next hop for packet session={} host={}",
                    packet.session_id, self.id
                );
                let _ = self.sendto_controller(packet.clone());
                SendingCodes::NoNextHop(msg)
            }
        };

        // Final log
        self.send_logging(
            LogType::INFO,
            LogModule::PACKET,
            format!("send_packet result: {:?}", result),
        );
        result
    }
    fn sendto_controller(&self, packet: Packet) -> Result<(), String> {
        self.send_logging(
            LogType::DEBUG,
            LogModule::PACKET,
            format!("Notifying controller of packet session={}", packet.session_id),
        );
        match self.controller_send.send(HostEvent::PacketSent(packet)) {
            Ok(_) => {
                self.send_logging(
                    LogType::DEBUG,
                    LogModule::PACKET,
                    "Controller notified successfully".to_string(),
                );
                Ok(())
            }
            Err(e) => {
                let err = e.to_string();
                self.send_logging(
                    LogType::ERROR,
                    LogModule::PACKET,
                    format!("Failed to notify controller: {}", err),
                );
                Err(err)
            }
        }
    }
    fn handle_ack(&mut self, ack: Ack, hops: &[NodeId], session_id: u64) {
        // Update PDR estimates on drone hops
        let drone_hops: Vec<_> = hops
            .iter()
            .copied()
            .filter(|nid| {
                matches!(
                    self.network.nodes.get(nid).map(|(ty, _)| ty),
                    Some(NodeType::Drone)
                )
            })
            .collect();
        self.network.pdr_helper(&drone_hops);

        self.send_logging(
            LogType::INFO,
            LogModule::ACK,
            format!(
                "Received ACK session={} fragment={} via hops={:?}",
                session_id, ack.fragment_index, hops
            ),
        );

        // Possibly reroute based on updated PDR
        {
            let mut sent = self.sent_fragments.lock().unwrap();
            if let Some(sess) = sent.get_mut(&session_id) {
                let dest = sess.destination;
                if let Some(new_route) = self.network.find_best_route(self.id, dest) {
                    sess.route = new_route.clone();
                    sess.last_activity = Instant::now();
                    self.send_logging(
                        LogType::DEBUG,
                        LogModule::ACK,
                        format!(
                            "Route updated session={} new_route={:?}",
                            session_id, new_route
                        ),
                    );
                }
            }
        }

        // Mark fragment confirmed or log errors
        let idx = ack.fragment_index as usize;
        let mut sent = self.sent_fragments.lock().unwrap();
        match sent.get_mut(&session_id) {
            None => self.send_logging(
                LogType::WARN,
                LogModule::ACK,
                format!("ACK for unknown session {}", session_id),
            ),
            Some(sess) => match sess.buffer.get(idx) {
                None => self.send_logging(
                    LogType::WARN,
                    LogModule::ACK,
                    format!("ACK for out-of-range fragment {}", ack.fragment_index),
                ),
                Some(None) => self.send_logging(
                    LogType::INFO,
                    LogModule::ACK,
                    format!("ACK for already confirmed fragment {}", ack.fragment_index),
                ),
                Some(Some(_)) => {
                    sess.buffer[idx] = None;
                    sess.pending = sess.pending.saturating_sub(1);
                    sess.last_activity = Instant::now();
                    self.send_logging(
                        LogType::INFO,
                        LogModule::ACK,
                        format!(
                            "Fragment {} confirmed, pending={}",
                            ack.fragment_index, sess.pending
                        ),
                    );
                }
            },
        }
    }
    fn handle_nack(&mut self, nack: Nack, generator: NodeId, session_id: u64) {
        self.send_logging(
            LogType::INFO,
            LogModule::NACK,
            format!(
                "Received NACK session={} fragment={} type={:?}",
                session_id, nack.fragment_index, nack.nack_type
            ),
        );
        // Retrieve session data
        let (frag, route, dest, mut banned) = {
            let sent = self.sent_fragments.lock().unwrap();
            match sent.get(&session_id).and_then(|s| s.buffer.get(nack.fragment_index as usize)) {
                Some(Some(f)) => (f.clone(), sent[&session_id].route.clone(), sent[&session_id].destination, sent[&session_id].banned_nodes.clone()),
                _ => {
                    self.send_logging(
                        LogType::WARN,
                        LogModule::NACK,
                        format!("NACK for invalid fragment {}", nack.fragment_index),
                    );
                    return;
                }
            }
        };

        match nack.nack_type {
            NackType::Dropped => {
                self.send_logging(
                    LogType::INFO,
                    LogModule::NACK,
                    format!("Retransmitting dropped fragment {}", nack.fragment_index),
                );
                self.send_fragment(session_id, frag.clone());
                self.network.pdr_from_nack(generator);
            }
            NackType::ErrorInRouting(bad) | NackType::UnexpectedRecipient(bad) => {
                self.send_logging(
                    LogType::ERROR,
                    LogModule::NACK,
                    format!("Routing error at {} for fragment {}", bad, nack.fragment_index),
                );
                if !route.contains(&bad) {
                    self.send_fragment(session_id, frag.clone());
                    return;
                }
                banned.insert(bad);
                if let Some(new_route) = self.network.find_best_route_excluding(self.id, dest, &banned) {
                    self.send_logging(
                        LogType::INFO,
                        LogModule::NACK,
                        format!("Alternative route found: {:?}", new_route),
                    );
                    {
                        let mut sent = self.sent_fragments.lock().unwrap();
                        if let Some(sess) = sent.get_mut(&session_id) {
                            sess.route = new_route.clone();
                            sess.banned_nodes = banned.clone();
                            sess.last_activity = Instant::now();
                        }
                    }
                    let header = SourceRoutingHeader::new(new_route.clone(), 1);
                    let pkt = Packet::new_fragment(header, session_id, frag.clone());
                    self.send_logging(
                        LogType::DEBUG,
                        LogModule::NACK,
                        "Retransmitting on new route".to_string(),
                    );
                    let _ = self.send_packet(pkt, None);
                } else {
                    self.send_logging(
                        LogType::WARN,
                        LogModule::NACK,
                        "No alternative route: triggering flood".to_string(),
                    );
                    self.initiate_flood();
                }
            }
            NackType::DestinationIsDrone => {
                self.send_logging(
                    LogType::ERROR,
                    LogModule::NACK,
                    "Destination is a drone—dropping fragment".to_string(),
                );
            }
        }
    }
    fn build_msg_fragment<F>(&mut self, destination: NodeId, message: Message, mut on_fragment_sent: F, ) -> Result<(), String>
    where
        F: FnMut(usize),
    {
        self.send_logging(
            LogType::INFO,
            LogModule::FRAGMENT,
            format!(
                "Starting fragmentation session={} dest={}",
                message.session_id, destination
            ),
        );
        // Find best route
        let route = self
            .network
            .find_best_route(self.id, destination)
            .ok_or_else(|| {
                let err = format!("No route to {}", destination);
                self.send_logging(
                    LogType::ERROR,
                    LogModule::FRAGMENT,
                    format!("Fragmentation error: {}", err),
                );
                err
            })?;

        // Calculate number of fragments
        let bytes = &message.content;
        let total = (bytes.len() + FRAGMENT_DSIZE - 1) / FRAGMENT_DSIZE;

        // Initialize session data if needed
        {
            let mut sent = self.sent_fragments.lock().map_err(|_| "Mutex poisoned")?;
            let entry = sent.entry(message.session_id).or_insert_with(|| {
                self.send_logging(
                    LogType::DEBUG,
                    LogModule::FRAGMENT,
                    format!("Allocating buffer for {} fragments", total),
                );
                SentSessionData {
                    buffer: vec![None; total],
                    last_activity: Instant::now(),
                    route: route.clone(),
                    destination,
                    pending: total,
                    banned_nodes: HashSet::new(),
                }
            });
            entry.pending = total;
            entry.last_activity = Instant::now();
            if entry.buffer.len() < total {
                entry.buffer.resize(total, None);
            }
        }

        // Send each fragment
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
                sent.get_mut(&message.session_id)
                    .unwrap()
                    .buffer[i] = Some(frag.clone());
            }
            let header = SourceRoutingHeader::new(route.clone(), 1);
            let pkt = Packet::new_fragment(header, message.session_id, frag.clone());
            let res = self.send_packet(pkt, None);
            on_fragment_sent(chunk.len());
            self.send_logging(
                LogType::DEBUG,
                LogModule::FRAGMENT,
                format!("Sent fragment #{} result={:?}", i, res),
            );
        }

        self.send_logging(
            LogType::INFO,
            LogModule::FRAGMENT,
            format!("Completed fragmentation session={}", message.session_id),
        );
        Ok(())
    }
    fn build_packet_ack(&self, packet: Packet, fragment_index: Option<u64>) -> Packet {
        let mut header = packet.routing_header;
        header.reverse();
        Packet::new_ack(header, packet.session_id, fragment_index.unwrap_or(0))
    }
    fn get_fragment(&self, session_id: u64, fragment_index: u64) -> Option<Fragment> {
        let sent = self.sent_fragments.lock().unwrap();
        sent.get(&session_id)
            .and_then(|s| s.buffer.get(fragment_index as usize).cloned().flatten())
    }
    fn send_fragment(&self, session_id: u64, fragment: Fragment) {
        self.send_logging(
            LogType::DEBUG,
            LogModule::FRAGMENT,
            format!(
                "Re-sending fragment {} of session={}",
                fragment.fragment_index, session_id
            ),
        );
        // Retrieve route
        let route = {
            let mut sent = self.sent_fragments.lock().unwrap();
            if let Some(sess) = sent.get_mut(&session_id) {
                sess.last_activity = Instant::now();
                sess.route.clone()
            } else {
                self.send_logging(
                    LogType::WARN,
                    LogModule::FRAGMENT,
                    format!("No session data for {}", session_id),
                );
                return;
            }
        };
        // Build and send
        let mut header = SourceRoutingHeader::new(route.clone(), 0);
        let mut pkt = Packet::new_fragment(header, session_id, fragment.clone());
        pkt.routing_header.hop_index += 1;
        let res = self.send_packet(pkt, None);
        self.send_logging(
            LogType::DEBUG,
            LogModule::FRAGMENT,
            format!("Re-send result: {:?}", res),
        );
    }
}