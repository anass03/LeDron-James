use std::collections::HashMap;
use std::error::Error;
use std::io::Cursor;
use std::time::Instant;
use crossbeam_channel::{Receiver, Sender};
use delegate::delegate;
use wg_2024::network::NodeId;
use wg_2024::packet::{Fragment, NodeType, Packet};
use host::{HostCommand, Message, Host, Functionality, ServerTypes, RecSessionData, HostEvent, Edge, ServerToClient, generate_unique_session_id};
use zstd::{decode_all, encode_all};

/// GenServer represents a generic server (Chat or Content server),
/// responsible for receiving fragmented packets, reassembling them
/// and handling simulation commands from the Simulation Controller.
pub struct GenServer {
    pub host: Host,                               // Internal host logic-shared between clients and servers
    pub controller_recv: Receiver<HostCommand>,   // Channel receiving simulation controller commands
    pub server_type: ServerTypes,                 // Indicates whether this server is Chat or Content
}

/// Trait defining the behavior that every server must implement.
/// This allows separating logic for packets, decompression, and simulation commands.
pub trait GenServerBehavior {
    /// Handles commands received from the Simulation Controller
    fn handle_sim_command(&mut self, cmd: HostCommand);

    /// Returns the server type (Chat or Content)
    fn servertype(&mut self) -> Result<ServerTypes, String>;

    /// Handles the reception of message fragments and performs reassembly when complete
    fn handle_msg_fragment(&mut self, packet: Packet, fragment: Fragment) -> Option<Vec<u8>>;

    /// Decompresses a compressed message using zstd
    fn decompress_anything(&self, compressed: &[u8]) -> Result<Vec<u8>, Box<dyn Error>>;

    /// Compresses data using zstd
    fn compress_anything(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>>;
    /// Serializes, compresses, and sends ServerToClient responses back to clients.
    fn send_response(&mut self, response: ServerToClient, destination: NodeId);
}

impl GenServer {
    delegate! {
        to self.host {
            /// Builds a message fragment from a high-level message
            pub fn build_msg_fragment(&mut self, destination: NodeId, message: Message) -> Result<(), String>;
            /// Sends log messages to the central logger
            pub fn send_logging(&self, message: String);
        }
    }

    /// Creates a new generic server.
    /// Ensures the server is connected to 1 or 2 drones (as required by the protocol).
    pub fn new(
        id: NodeId,
        type_host: NodeType,
        connected_drone_ids: Vec<NodeId>,
        controller_send: Sender<HostEvent>,
        controller_recv: Receiver<HostCommand>,
        packet_recv: Receiver<Packet>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        server_type: ServerTypes,
        log_sender: Sender<String>,
    ) -> Self {
        assert!(
            connected_drone_ids.len() >= 1 && connected_drone_ids.len() <= 2,
            "Server must be connected to at least one and at most two drones!"
        );

        let host = Host::new(id, type_host, controller_send, packet_recv, packet_send, log_sender);
        Self {
            host,
            controller_recv,
            server_type,
        }
    }
}

impl GenServerBehavior for GenServer {
    fn handle_sim_command(&mut self, cmd: HostCommand) {
        match cmd {
            HostCommand::AddSender(id, sender) => {
                let current = self.host.packet_send.len();
                if self.host.packet_send.contains_key(&id) {
                    self.send_logging("Channel already present!".to_string());
                } else if current >= 2 {
                    self.send_logging("Cannot add more than 2 channels: limit reached".to_string());
                } else {
                    self.host.packet_send.insert(id, sender);

                    // Updates internal topology graph (used for discovery protocol and debugging)
                    let entry = self.host.network.nodes
                        .entry(self.host.id)
                        .or_insert_with(|| (NodeType::Server, Vec::new()));
                    entry.1.push(Edge { to: id, weight: 0.0 });

                    let entry = self.host.network.nodes
                        .entry(id)
                        .or_insert_with(|| (NodeType::Drone, Vec::new()));
                    entry.1.push(Edge { to: self.host.id, weight: 0.0 });

                    self.send_logging("Successfully added sender!".to_string());
                }
            }
            HostCommand::RemoveSender(id) => {
                let current = self.host.packet_send.len();
                if !self.host.packet_send.contains_key(&id) {
                    self.send_logging("Channel not found!".to_string());
                } else if current <= 1 {
                    self.send_logging("Cannot remove last channel: at least one connection required".to_string());
                } else {
                    self.host.packet_send.remove(&id);

                    if let Some((_, edges)) = self.host.network.nodes.get_mut(&self.host.id) {
                        edges.retain(|e| e.to != id);
                    }
                    if let Some((_, edges)) = self.host.network.nodes.get_mut(&id) {
                        edges.retain(|e| e.to != self.host.id);
                        self.send_logging("Successfully removed sender!".to_string());
                    }
                }
            }
        }
    }

    fn servertype(&mut self) -> Result<ServerTypes, String> {
        if let NodeType::Server = self.host.type_host {
            Ok(self.server_type.clone())
        } else {
            Err(format!("Unknown type {}", self.host.id))
        }
    }

    fn handle_msg_fragment(&mut self, packet: Packet, fragment: Fragment) -> Option<Vec<u8>> {
        let session_id = packet.session_id;
        let idx = fragment.fragment_index as usize;
        let total = fragment.total_n_fragments as usize;

        // Initialize reassembly buffer for this session if needed
        let mut received = self.host.received_fragments.lock().unwrap();
        let session = received.entry(session_id).or_insert_with(|| {
            RecSessionData {
                buffer: vec![None; total],
                last_activity: Instant::now(),
                pending: total,
            }
        });

        if idx >= session.buffer.len() {
            self.send_logging(format!("Fragment index {} out of bounds (total {})", idx, total));
            return None;
        }

        // Refresh last activity timestamp
        session.last_activity = Instant::now();

        // Store fragment only if it's new
        if session.buffer[idx].is_none() {
            session.buffer[idx] = Some(fragment.clone());
            session.pending = session.pending.saturating_sub(1);

            self.host.send_logging(format!(
                "Received fragment, pending: {}, host: {}, session: {}",
                self.host.id,
                session_id, session.pending
            ));
        }

        // Send ACK for this fragment (required by protocol for reliability)
        let mut ack = self.host.build_packet_ack(packet.clone(), Some(fragment.fragment_index));
        ack.routing_header.hop_index += 1;
        let _ = self.host.send_packet(ack, None);

        // Update internal PDR helper for logging/debugging purposes
        let drone_hops: Vec<NodeId> = packet.routing_header.hops.iter()
            .filter(|&&nid| matches!(self.host.network.nodes.get(&nid).map(|(ty, _)| ty), Some(NodeType::Drone)))
            .copied()
            .collect();
        self.host.network.pdr_helper(&drone_hops);

        self.host.send_logging(format!(
            "PDR updated host={} header hops={:?}",
            self.host.id,
            packet.routing_header.hops,
        ));

        // If all fragments have been received, proceed with reassembly
        if session.pending == 0 {
            self.host.send_logging(format!("All fragments received host={} session={}", self.host.id, session_id));

            let data = session.buffer
                .iter()
                .filter_map(|f| f.as_ref())
                .flat_map(|f| f.data[..f.length as usize].iter().cloned())
                .collect::<Vec<u8>>();

            received.remove(&session_id);

            // Decompress reassembled data before returning a complete message
            match self.decompress_anything(&data) {
                Ok(decompressed_data) => return Some(decompressed_data),
                Err(e) => self.send_logging(format!("Failed to decompress fragments: {}", e)),
            }
        }
        None
    }

    fn decompress_anything(&self, compressed: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        decode_all(Cursor::new(compressed)).map_err(|_| "Failed to decompress".into())
    }

    fn compress_anything(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        encode_all(data, 0).map_err(|e| e.into())
    }
    /// Serializes, compresses, and sends ServerToClient responses back to clients.
    fn send_response(&mut self, response: ServerToClient, destination: NodeId) {
        match serde_json::to_vec(&response) {
            Ok(data) => {
                if let Ok(compressed_data) = self.compress_anything(data.as_slice()) {
                    let msg = Message {
                        session_id: generate_unique_session_id(),
                        content: compressed_data,
                    };
                    if let Err(e) = self.build_msg_fragment(destination, msg) {
                        self.send_logging(format!("Failed to send response to {}: {}", destination, e));
                    }
                }
            }
            Err(e) => {
                self.send_logging(format!("Failed to serialize ServerToClient: {}", e));
            }
        }
    }
}
