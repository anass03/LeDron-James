use std::collections::HashMap;
use std::time::Instant;
use crossbeam_channel::{Receiver, Sender};
use delegate::delegate;
use wg_2024::network::NodeId;
use wg_2024::packet::{Fragment, NodeType, Packet};
use host::{Functionality, Host, Message, RecSessionData, HostEvent, HostCommand, FileId, MediaId, ClientId};
use host::ServerTypes;

pub struct GenServer {
    pub host: Host,
    pub controller_recv: Receiver<HostCommand>,
    pub server_type: ServerTypes,
}

pub trait GenServerBehavior {
    fn handle_sim_command(&mut self, cmd: HostCommand);
    fn servertype(&mut self) -> Result<ServerTypes, String>;
    fn handle_msg_fragment(&mut self, packet: Packet, fragment: Fragment) -> Option<Vec<u8>>;
}

impl GenServer {
    delegate! {
        to self.host {
            pub fn build_msg_fragment(&mut self, destination: NodeId, message: Message) -> Result<(), String>;
            pub fn send_logging(&self, message: String);
        }
    }

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

        let host = Host::new(id, type_host, controller_send, packet_recv, packet_send,log_sender);
        Self {
            host,
            controller_recv,
            server_type,
        }
    }
}

impl GenServerBehavior for GenServer {
    fn handle_sim_command(&mut self, cmd: HostCommand){
        match cmd {
            HostCommand::AddSender(id, sender) => {
                let current = self.host.packet_send.len();
                if self.host.packet_send.contains_key(&id) {
                    self.host.send_logging("Channel already present!".to_string());
                } else if current >= 2 {
                    self.host.send_logging("Cannot add more than 2 channels: limit reached".to_string());
                } else {
                    self.host.packet_send.insert(id, sender);
                    self.host.send_logging("Successfully added sender!".to_string());
                }
            }
            HostCommand::RemoveSender(id) => {
                let current = self.host.packet_send.len();
                if !self.host.packet_send.contains_key(&id) {
                    self.host.send_logging("Channel not found!".to_string());
                } else if current <= 1 {
                    self.host.send_logging("Cannot remove last channel: at least one connection required".to_string());
                } else {
                    self.host.packet_send.remove(&id);
                    self.host.send_logging("Successfully removed sender!".to_string());
                }
            }
        }
    }
    fn servertype(&mut self) -> Result<ServerTypes, String> {
            if let NodeType::Server = self.host.type_host {
                let response = match self.server_type {
                    ServerTypes::Chat => ServerTypes::Chat,
                    ServerTypes::Content => ServerTypes::Content,
                };
                Ok(response)
            }else{
                Err(format!("Unknown type {}", self.host.id))
            }
            
    }

    fn handle_msg_fragment(&mut self, packet: Packet, fragment: Fragment) -> Option<Vec<u8>> {
        let session_id = packet.session_id;
        let idx = fragment.fragment_index as usize;
        let total = fragment.total_n_fragments as usize;

        // inizializza se necessario
        let mut received = self.host.received_fragments.lock().unwrap();
        let session = received.entry(session_id).or_insert_with(|| {
            RecSessionData {
                buffer: vec![None; total],
                last_activity: Instant::now(),
                pending: total,  // <— inizializzo qui
            }
        });

        // aggiorna timestamp
        session.last_activity = Instant::now();

        // memorizza solo se nuovo
        if idx < session.buffer.len() && session.buffer[idx].is_none() {
            session.buffer[idx] = Some(fragment.clone());
            session.pending = session.pending.saturating_sub(1);

            self.host.send_logging(format!(
                "Received fragment, pending: {}, host: {}, session: {}",
                self.host.id,
                session_id,session.pending
            ));
        }

        // invia ACK
        let mut ack = self.host.build_packet_ack(packet.clone(), Some(fragment.fragment_index));
        ack.routing_header.hop_index += 1;
        let _ = self.host.send_packet(ack, None);

        let drone_hops: Vec<NodeId> = packet.routing_header.hops.iter()
            .filter(|&&nid| {
                matches!(
                        self.host.network.nodes.get(&nid).map(|(ty, _)| ty),
                        Some(NodeType::Drone))}
            )
            .copied()
            .collect();
        self.host.network.pdr_helper(&drone_hops);

        self.host.send_logging(format!(
            "PDR updated host={} header hops={:?}",
            self.host.id,
            packet.routing_header.hops,
        ));
        
        // se finito, reassembly e rimozione
        if session.pending == 0 {
            self.host.send_logging( format!(
                "All fragments received host={} session={}",
                self.host.id,
                session_id,
            ));

            self.host.send_logging(
                format!("host: {}, session: {}, All fragments received",
                        self.host.id, session_id));

            let data = session.buffer
                .iter()
                .filter_map(|f| f.as_ref())
                .flat_map(|f| f.data[..f.length as usize].iter().cloned())
                .collect::<Vec<u8>>();

            received.remove(&session_id);
            return Some(data);
        }
        None
    }
}
