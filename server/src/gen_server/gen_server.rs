use crate::delegate_genserver_host_ref;
use crossbeam_channel::{Receiver, Sender};
use host::flood::FloodControl;
use host::functionality::Functionality;
use host::logging::{LogControl, LogModule, LogType};
use host::network::Edge;
use host::session::{generate_unique_session_id, RecSessionData, SentSessionData};
use host::{
    ContentID, Host, HostCommand, HostEvent, Message, SendingCodes, ServerToClient, ServerTypes,
};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::io::Cursor;
use std::time::Instant;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Fragment, NodeType, Packet, FRAGMENT_DSIZE};
use zstd::{decode_all, encode_all};

/// Generic Server structure used to implement Content or Communication servers.
pub struct GenServer {
    pub host: Host,
    pub controller_recv: Receiver<HostCommand>,
    pub server_type: ServerTypes,
    pub(crate) pending_downloads: HashMap<u64, ScheduledDownload>,
}

pub struct ScheduledDownload {
    msg: Message,
    pub destination: NodeId,
    pub total: usize,
    pub content_type: ContentType,
    pub id: String,
}
impl GenServer {
    delegate_genserver_host_ref! {
        pub fn send_logging(&self, log_type: LogType, module: LogModule, msg: String);
        fn send_packet(&self, packet: Packet, channel: Option<&Sender<Packet>>) -> SendingCodes;
        fn build_packet_ack(&self, packet: Packet, fragment_index: Option<u64>) -> Packet;
    }

    /// Constructs a new `GenServer` instance.
    pub fn new(
        id: NodeId,
        type_host: NodeType,
        connected_drone_ids: Vec<NodeId>,
        controller_send: Sender<HostEvent>,
        controller_recv: Receiver<HostCommand>,
        packet_recv: Receiver<Packet>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        server_type: ServerTypes,
    ) -> Self {
        assert!(
            connected_drone_ids.len() >= 2,
            "Server must be connected to at least 2 drones"
        );
        let host = Host::new(id, type_host, controller_send, packet_recv, packet_send);
        Self {
            host,
            controller_recv,
            server_type,
            pending_downloads: HashMap::new(),
        }
    }
    /// Handles commands from the Simulation Controller.
    pub fn handle_sim_command(&mut self, cmd: HostCommand) {
        match cmd {
            HostCommand::AddSender(id, sender) => {
                if self.host.packet_send.contains_key(&id) {
                    self.send_logging(
                        LogType::WARN,
                        LogModule::NETWORK,
                        "Channel already present!".to_string(),
                    );
                } else if self.host.packet_send.len() >= 2 {
                    self.send_logging(
                        LogType::WARN,
                        LogModule::PACKET,
                        "Cannot add more than 2 channels.".to_string(),
                    );
                } else {
                    self.host.packet_send.insert(id, sender);
                    self.host
                        .network
                        .nodes
                        .entry(self.host.id)
                        .or_insert_with(|| (NodeType::Server, Vec::new()))
                        .1
                        .push(Edge {
                            to: id,
                            weight: 0.0,
                        });
                    self.host
                        .network
                        .nodes
                        .entry(id)
                        .or_insert_with(|| (NodeType::Drone, Vec::new()))
                        .1
                        .push(Edge {
                            to: self.host.id,
                            weight: 0.0,
                        });
                    self.send_logging(
                        LogType::INFO,
                        LogModule::NETWORK,
                        "Sender added successfully.".to_string(),
                    );
                }
            }
            HostCommand::RemoveSender(id) => {
                if self.host.packet_send.remove(&id).is_some() {
                    if let Some((_, edges)) = self.host.network.nodes.get_mut(&self.host.id) {
                        edges.retain(|e| e.to != id);
                    }
                    if let Some((_, edges)) = self.host.network.nodes.get_mut(&id) {
                        edges.retain(|e| e.to != self.host.id);
                    }
                    self.send_logging(
                        LogType::INFO,
                        LogModule::NETWORK,
                        "Sender removed successfully.".to_string(),
                    );
                } else {
                    self.send_logging(
                        LogType::WARN,
                        LogModule::PACKET,
                        "Sender not found.".to_string(),
                    );
                }
            }
        }
    }

    /// Returns the type of the server (Content/Chat).
    pub fn servertype(&mut self) -> Result<ServerTypes, String> {
        match self.host.type_host {
            NodeType::Server => Ok(self.server_type.clone()),
            _ => Err(format!("Invalid node type for {}", self.host.id)),
        }
    }

    /// Handles a received message fragment.
    pub fn handle_msg_fragment(&mut self, packet: Packet, fragment: Fragment) -> Option<Vec<u8>> {
        let session_id = packet.session_id;
        let idx = fragment.fragment_index as usize;
        let total = fragment.total_n_fragments as usize;

        let mut received = self.host.received_fragments.lock().unwrap();
        let session = received
            .entry(session_id)
            .or_insert_with(|| RecSessionData {
                buffer: vec![None; total],
                last_activity: Instant::now(),
                pending: total,
            });

        if idx >= session.buffer.len() {
            self.send_logging(
                LogType::WARN,
                LogModule::FRAGMENT,
                format!("Fragment index {} out of bounds (max {})", idx, total),
            );
            return None;
        }

        session.last_activity = Instant::now();
        if session.buffer[idx].is_none() {
            session.buffer[idx] = Some(fragment);
            session.pending = session.pending.saturating_sub(1);
        }

        let mut ack = self.build_packet_ack(packet.clone(), Some(idx as u64));
        ack.routing_header.hop_index += 1;
        let _ = self.send_packet(ack, None);

        let drone_hops: Vec<NodeId> = packet
            .routing_header
            .hops
            .iter()
            .filter(|&&nid| {
                matches!(
                    self.host.network.nodes.get(&nid).map(|(ty, _)| ty),
                    Some(NodeType::Drone)
                )
            })
            .copied()
            .collect();
        self.host.network.pdr_helper(&drone_hops);

        if session.pending == 0 {
            let data = session
                .buffer
                .iter()
                .filter_map(|f| f.as_ref())
                .flat_map(|f| f.data[..f.length as usize].iter().copied())
                .collect::<Vec<u8>>();

            received.remove(&session_id);

            match self.decompress_anything(&data) {
                Ok(decompressed_data) => Some(decompressed_data),
                Err(e) => {
                    self.send_logging(
                        LogType::ERROR,
                        LogModule::FRAGMENT,
                        format!("Failed to decompress fragments: {}", e),
                    );
                    None
                }
            }
        } else {
            None
        }
    }

    /// Decompresses a byte slice.
    fn decompress_anything(&self, compressed: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        decode_all(Cursor::new(compressed)).map_err(|e| Box::new(e) as Box<dyn Error>)
    }

    /// Compresses a byte slice.
    fn compress_anything(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        encode_all(data, 0).map_err(|e| Box::new(e) as Box<dyn Error>)
    }

    /// Sends a high-level response to a client.
    pub fn send_response(&mut self, response: ServerToClient, destination: NodeId) {
        if let Some((msg, meta, total)) = self.prepare_message(&response) {
            if let Some((content_type, id)) = meta {
                // it’s a File or Media
                if !matches!(response, ServerToClient::Sending(_, _)) {
                    // 1) build the Sending notification
                    let cid = match content_type {
                        ContentType::File  => ContentID::File(id.clone()),
                        ContentType::Media => ContentID::Media(id.clone()),
                    };
                    let sending = ServerToClient::Sending(cid, total);

                    // 2) prepare it so we get its session_id
                    if let Some((send_msg, _, _)) = self.prepare_message(&sending) {
                        let send_session = send_msg.session_id;

                        // 3) stash the real content msg for later
                        self.pending_downloads.insert(send_session, ScheduledDownload {
                            msg,
                            destination,
                            total,                        // numero di frammenti calcolato
                            content_type,                 // File o Media
                            id: id.clone(),
                        });

                        // 4) send *only* the Sending notification now
                        if let Err(e) = self.build_msg_fragment(destination, send_msg) {
                            self.send_logging(
                                LogType::ERROR,
                                LogModule::PACKET,
                                format!(
                                    "Failed to send download notification to {}: {}",
                                    destination, e
                                ),
                            );
                        }
                    } else {
                        self.send_logging(
                            LogType::ERROR,
                            LogModule::PACKET,
                            "Failed to prepare Sending notification".to_string(),
                        );
                    }
                    return; // skip sending the real msg for now
                }
            }

            // all other cases (plain msgs or a “Sending” itself) go straight through
            if let Err(e) = self.build_msg_fragment(destination, msg) {
                self.send_logging(
                    LogType::ERROR,
                    LogModule::PACKET,
                    format!("Failed to send response to {}: {}", destination, e),
                );
            }
        } else {
            self.send_logging(
                LogType::ERROR,
                LogModule::PACKET,
                "prepare_message returned None".to_string(),
            );
        }
    }
    // pub fn send_response(&mut self, response: ServerToClient, destination: NodeId) {
    //     if let Some((msg, meta, total)) = self.prepare_message(&response) {
    //         if let Some((content_type, id)) = meta {
    //             if !matches!(response, ServerToClient::Sending(_, _)) {
    //                 self.send_download(destination, total, content_type, id);
    //             }
    //         }
    //         if let Err(e) = self.build_msg_fragment(destination, msg) {
    //             self.send_logging(
    //                 LogType::ERROR,
    //                 LogModule::PACKET,
    //                 format!("Failed to send response to {}: {}", destination, e),
    //             );
    //         }
    //     } else {
    //         self.send_logging(
    //             LogType::ERROR,
    //             LogModule::PACKET,
    //             "prepare_message returned None".to_string(),
    //         );
    //     }
    // }

    /// Sending directly the prepared message, after receiving ack from client for packet ServerToClient::Sending(_, _)
    pub fn send_stashed_content(&mut self, sd: ScheduledDownload) {
        let session = sd.msg.session_id.clone();
        if let Err(e) = self.build_msg_fragment(sd.destination, sd.msg) {
            self.send_logging(
                LogType::ERROR,
                LogModule::PACKET,
                format!(
                    "Failed to send stashed content (session={}): {}",
                    session, e
                ),
            );
        }
    }
    /// Sends a content download notification before sending the actual data.
    // pub fn send_download(
    //     &mut self,
    //     destination: NodeId,
    //     total: usize,
    //     content: ContentType,
    //     id: String,
    // ) {
    //     let cid = match content {
    //         ContentType::File => ContentID::File(id),
    //         ContentType::Media => ContentID::Media(id),
    //     };
    //     self.send_response(ServerToClient::Sending(cid, total), destination);
    // }

    /// Prepares a message for transmission, optionally extracting content metadata.
    fn prepare_message(
        &self,
        response: &ServerToClient,
    ) -> Option<(Message, Option<(ContentType, String)>, usize)> {
        let data = serde_json::to_vec(response).ok()?;
        let compressed = self.compress_anything(&data).ok()?;
        let total = (compressed.len() + FRAGMENT_DSIZE - 1) / FRAGMENT_DSIZE;
        let msg = Message {
            session_id: generate_unique_session_id(),
            content: compressed,
        };

        let meta = match response {
            ServerToClient::File(_, id, _) | ServerToClient::FileFrom(_, _, _, id, _) => {
                Some((ContentType::File, id.clone()))
            }
            ServerToClient::Media(_, id, _) | ServerToClient::MediaFrom(_, _, _, id, _) => {
                Some((ContentType::Media, id.clone()))
            }
            _ => None,
        };

        Some((msg, meta, total))
    }

    fn build_msg_fragment(&mut self, destination: NodeId, message: Message) -> Result<(), String> {
        self.send_logging(
            LogType::INFO,
            LogModule::FRAGMENT,
            format!(
                "Starting fragmentation host={} session={} dest={}",
                self.host.id, message.session_id, destination,
            ),
        );
        //Find Route
        let route = self
            .host
            .network
            .find_best_route(self.host.id, destination)
            .ok_or_else(|| {
                let err = format!("No route to {}", destination);
                self.send_logging(
                    LogType::ERROR,
                    LogModule::FRAGMENT,
                    format!("Fragmentation error: {}", err),
                );
                err
            })?;
        match self
            .host
            .controller_send
            .send(HostEvent::Route(self.host.id, route.clone()))
        {
            Ok(_) => {}
            Err(err) => {
                self.send_logging(
                    LogType::WARN,
                    LogModule::SESSION,
                    format!("error sending route to controller {}", err),
                );
            }
        }

        // Total fragment number
        let bytes = &message.content;
        let total = (bytes.len() + FRAGMENT_DSIZE - 1) / FRAGMENT_DSIZE;

        // Prepare/Update SessionData
        {
            let mut sent = self
                .host
                .sent_fragments
                .lock()
                .map_err(|_| "Mutex poisoned")?;
            let session = sent.entry(message.session_id).or_insert_with(|| {
                let host = self.host.id;
                let session = message.session_id;
                self.send_logging(
                    LogType::INFO,
                    LogModule::FRAGMENT,
                    format!(
                        "New SentSessionData host={} session={} total={}",
                        host, session, total,
                    ),
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

            session.pending = total;
            session.last_activity = Instant::now();
            if session.buffer.len() < total {
                session.buffer.resize(total, None);
            }
        }

        //Sending Fragments
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

            //Update Buffer
            {
                let mut sent = self
                    .host
                    .sent_fragments
                    .lock()
                    .map_err(|_| "Mutex poisoned")?;
                if let Some(session) = sent.get_mut(&message.session_id) {
                    session.buffer[i] = Some(frag.clone());
                } else {
                    return Err("Missing session in sent_fragments".to_string());
                }
            }

            //Build and Send Packet
            let header = SourceRoutingHeader::new(route.clone(), 1);
            let packet = Packet::new_fragment(header.clone(), message.session_id, frag.clone());
            let result = self.send_packet(packet, None);
            if matches!(result, SendingCodes::NoNextHop(_)) {
                self.host.initiate_flood();
            }

            self.send_logging(
                LogType::INFO,
                LogModule::FRAGMENT,
                format!(
                    "Send result host={} session={} fragment={} result={}",
                    self.host.id, message.session_id, i, result
                ),
            );
        }

        self.send_logging(
            LogType::INFO,
            LogModule::FRAGMENT,
            format!(
                "Fragmentation completed host={} session={}",
                self.host.id, message.session_id,
            ),
        );
        Ok(())
    }
}

/// Enum to indicate content type being transferred.
pub enum ContentType {
    File,
    Media,
}
