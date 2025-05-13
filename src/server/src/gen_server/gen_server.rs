use std::collections::HashMap;
use std::time::Instant;
use crossbeam_channel::{Receiver, Sender};
use delegate::delegate;
use wg_2024::network::NodeId;
use wg_2024::packet::{Fragment, NodeType, Packet, FRAGMENT_DSIZE};
use host::{ClientCommand, Functionality, Host, Message, generate_unique_session_id, RecSessionData, HostEvent, NetCommand};
use host::ServerTypes;
use host::ClientCommand::ServerType;
use tracing_subscriber::EnvFilter;
use tracing::{info, error};

pub struct GenServer {
    pub host: Host,
    pub controller_recv: Receiver<NetCommand>,
    pub server_type: ServerTypes,
}

pub trait GenServerBehavior {
    fn create_message(&mut self, data: Vec<u8>, destination: NodeId);
    fn servertype(&mut self, request: ClientCommand, destination: NodeId);
    fn handle_msg_fragment(&mut self, packet: Packet, fragment: Fragment) -> Option<Vec<u8>>;
}

impl GenServer {
    delegate! {
        to self.host {
            pub fn build_msg_fragment(&mut self, destination: NodeId, message: Message) -> Result<(), String>;
            //fn log<S: AsRef<str>>(&self, message: S);
        }
    }

    pub fn new(
        id: NodeId,
        type_host: NodeType,
        connected_drone_ids: Vec<NodeId>,
        controller_send: Sender<HostEvent>,
        controller_recv: Receiver<NetCommand>,
        packet_recv: Receiver<Packet>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        server_type: ServerTypes,
    ) -> Self {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .init();

        assert!(
            connected_drone_ids.len() >= 1 && connected_drone_ids.len() <= 2,
            "Server must be connected to at least one and at most two drones!"
        );

        let host = Host::new(id, type_host, controller_send, packet_recv, packet_send);
        Self {
            host,
            controller_recv,
            server_type,
        }
    }
}

impl GenServerBehavior for GenServer {
    fn create_message(&mut self, data: Vec<u8>, destination: NodeId) {
        let message = Message {
            session_id: generate_unique_session_id(),
            content: data,
        };
        match self.host.build_msg_fragment(destination, message) {
            Ok(_) => info!("Server ID {} - message built successfully", self.host.id),
            Err(e) => error!("Failed to build message: {}", e),
        }
    }

    fn servertype(&mut self, request: ClientCommand, destination: NodeId) {
        if let ServerType = request {
            if let NodeType::Server = self.host.type_host {
                let response = match self.server_type {
                    ServerTypes::Chat => ServerTypes::Chat,
                    ServerTypes::Content => ServerTypes::Content,
                };
                if let Ok(data) = serde_json::to_vec(&host::ServerToClient::ServerType(response)) {
                    self.create_message(data, destination);
                } else {
                    error!("Failed to serialize ServerType response");
                }
            }
        }
    }

    fn handle_msg_fragment(&mut self, packet: Packet, fragment: Fragment) -> Option<Vec<u8>> {
        let mut received_fragments = self.host.received_fragments.lock().unwrap();
        let session_data = received_fragments
            .entry(packet.session_id)
            .or_insert_with(|| RecSessionData {
                buffer: vec![None; fragment.total_n_fragments as usize],
                last_activity: Instant::now(),
                pending: fragment.total_n_fragments as usize,
            });

        let index = fragment.fragment_index as usize;
        session_data.last_activity = Instant::now();

        if index >= session_data.buffer.len() {
            /*self.host.log(format!(
                "Invalid fragment index: {} (session ID: {})",
                fragment.fragment_index, packet.session_id
            ));*/
            return None;
        }

        if session_data.buffer[index].is_some() {
            /*self.host.log(format!(
                "Duplicate fragment received: {} (session ID: {})",
                fragment.fragment_index, packet.session_id
            ));*/
            return None;
        }

        session_data.buffer[index] = Some(fragment.clone());
        session_data.pending = session_data.pending.saturating_sub(1);

        let ack_packet = self.host.build_packet_ack(packet.clone(), Some(fragment.fragment_index));
        self.host.send_packet(ack_packet, None);

        if session_data.pending == 0 {
            //self.host.log(format!("All fragments received for session ID: {}", packet.session_id));
            let mut content = Vec::with_capacity(fragment.total_n_fragments as usize * FRAGMENT_DSIZE);
            for frag in session_data.buffer.iter().filter_map(|f| f.as_ref()) {
                content.extend_from_slice(&frag.data[..frag.length as usize]);
            }
            received_fragments.remove(&packet.session_id);
            return Some(content);
        }

        None
    }
}
