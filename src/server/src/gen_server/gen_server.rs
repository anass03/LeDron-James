use std::collections::HashMap;
use std::error::Error;
use std::io::Cursor;
use std::time::Instant;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::network::NodeId;
use wg_2024::packet::{Fragment, NodeType, Packet, FRAGMENT_DSIZE};
use host::{HostCommand, Message, Host, ServerTypes, HostEvent, ServerToClient, ContentID, SendingCodes};
use host::{functionality::Functionality};
use host::logging::{LogControl, LogEntry, LogModule, LogType};
use host::network::{Edge};
use host::session::{RecSessionData,generate_unique_session_id};
use zstd::{decode_all, encode_all};
use crate::delegate_genserver_host_ref;

/// Generic Server structure used to implement Content or Communication servers.
pub struct GenServer {
    pub host: Host,
    pub controller_recv: Receiver<HostCommand>,
    pub server_type: ServerTypes,
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
        log_sender: Sender<LogEntry>,
    ) -> Self {
        assert!(connected_drone_ids.len() >= 2, "Server must be connected to at least 2 drones");
        let host = Host::new(id, type_host, controller_send, packet_recv, packet_send, log_sender);
        Self { host, controller_recv, server_type }
    }
    /// Handles commands from the Simulation Controller.
    pub fn handle_sim_command(&mut self, cmd: HostCommand) {
        match cmd {
            HostCommand::AddSender(id, sender) => {
                if self.host.packet_send.contains_key(&id) {
                    // self.send_logging("Channel already present!".to_string());
                } else if self.host.packet_send.len() >= 2 {
                    self.send_logging(LogType::WARN,LogModule::PACKET,"Cannot add more than 2 channels.".to_string());
                } else {
                    self.host.packet_send.insert(id, sender);
                    self.host.network.nodes.entry(self.host.id)
                        .or_insert_with(|| (NodeType::Server, Vec::new()))
                        .1.push(Edge { to: id, weight: 0.0 });
                    self.host.network.nodes.entry(id)
                        .or_insert_with(|| (NodeType::Drone, Vec::new()))
                        .1.push(Edge { to: self.host.id, weight: 0.0 });
                    // self.send_logging("Sender added successfully.".to_string());
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
                    // self.send_logging("Sender removed successfully.".to_string());
                } else {
                    self.send_logging(LogType::WARN,LogModule::PACKET,"Sender not found.".to_string());
                }
            }
        }
    }

    /// Returns the type of the server (Content/Chat).
    pub fn servertype(&mut self) -> Result<ServerTypes, String> {
        match self.host.type_host {
            NodeType::Server => Ok(self.server_type.clone()),
            _ => Err(format!("Invalid node type for {}", self.host.id))
        }
    }

    /// Handles a received message fragment.
    pub fn handle_msg_fragment(&mut self, packet: Packet, fragment: Fragment) -> Option<Vec<u8>> {
        let session_id = packet.session_id;
        let idx = fragment.fragment_index as usize;
        let total = fragment.total_n_fragments as usize;

        let mut received = self.host.received_fragments.lock().unwrap();
        let session = received.entry(session_id).or_insert_with(|| RecSessionData {
            buffer: vec![None; total],
            last_activity: Instant::now(),
            pending: total,
        });

        if idx >= session.buffer.len() {
            self.send_logging(LogType::WARN,LogModule::FRAGMENT,format!("Fragment index {} out of bounds (max {})", idx, total));
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

        let drone_hops: Vec<NodeId> = packet.routing_header.hops.iter()
            .filter(|&&nid| matches!(self.host.network.nodes.get(&nid).map(|(ty, _)| ty), Some(NodeType::Drone)))
            .copied()
            .collect();
        self.host.network.pdr_helper(&drone_hops, &self.host.log_sender);


        if session.pending == 0 {
            let data = session.buffer.iter()
                .filter_map(|f| f.as_ref())
                .flat_map(|f| f.data[..f.length as usize].iter().copied())
                .collect::<Vec<u8>>();

            received.remove(&session_id);

            match self.decompress_anything(&data) {
                Ok(decompressed_data) => Some(decompressed_data),
                Err(e) => {
                    self.send_logging(LogType::ERROR,LogModule::FRAGMENT,format!("Failed to decompress fragments: {}", e));
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
    pub(crate) fn send_response(&mut self, response: ServerToClient, destination: NodeId) {
        if let Some((msg, meta,total)) = self.prepare_message(&response) {
            if let Some((content_type, id)) = meta {
                self.send_download(destination, total, content_type, id);
            }
            if let Err(e) = self.host.build_msg_fragment(destination, msg, |_| {}) {
                self.send_logging(LogType::ERROR,LogModule::PACKET,format!("Failed to send response to {}: {}", destination, e));
            }
        } else {
            self.send_logging(LogType::ERROR,LogModule::PACKET,"prepare_message returned None".to_string());
        }
    }

    /// Sends a content download notification before sending the actual data.
    fn send_download(&mut self, destination: NodeId, total: usize, content: ContentType, id: String) {
        let cid = match content {
            ContentType::File => ContentID::File(id),
            ContentType::Media => ContentID::Media(id),
        };
        self.send_response(ServerToClient::Sending(cid, total), destination);
    }

    /// Prepares a message for transmission, optionally extracting content metadata.
    fn prepare_message(&self, response: &ServerToClient) -> Option<(Message, Option<(ContentType, String)>, usize)> {
        let data = serde_json::to_vec(response).ok()?;
        let total = (data.len() + FRAGMENT_DSIZE - 1) / FRAGMENT_DSIZE;
        let compressed = self.compress_anything(&data).ok()?;
        let msg = Message {
            session_id: generate_unique_session_id(),
            content: compressed,
        };

        let meta = match response {
            ServerToClient::File(_, id, _)
            | ServerToClient::FileFrom(_, _, id, _) => Some((ContentType::File, id.clone())),
            ServerToClient::Media(_, id, _)
            | ServerToClient::MediaFrom(_, _, id, _) => Some((ContentType::Media, id.clone())),
            _ => None,
        };

        Some((msg, meta,total))
    }
}

/// Enum to indicate content type being transferred.
pub enum ContentType {
    File,
    Media,
}
