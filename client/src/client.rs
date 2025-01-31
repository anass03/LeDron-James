use crossbeam_channel::{Receiver, Sender};
use wg_2024::controller::{DroneEvent};
use wg_2024::network::{NodeId};
use wg_2024::packet::{Packet, PacketType, NodeType};
use std::collections::{HashMap};
use std::sync::{Arc};
use std::time::Duration;
use host::{ClientCommand, Message, Host, DroneSend, Functionality, generate_unique_session_id, ServerTypes};

pub struct Client<M: DroneSend + Send + 'static> {
    host: Arc<Host<M>>,
    connected_drone_ids: Vec<NodeId>,
    messages: Vec<Message<M>>,
    servers: HashMap<ServerTypes, NodeId>
}

impl<M: DroneSend + Send + 'static> Client<M> {
    pub fn new(id:NodeId,
               type_host:NodeType,
               connected_drone_ids: Vec<NodeId>,
               controller_send:Sender<DroneEvent>,
               controller_recv:Receiver<ClientCommand<M>>,
               packet_recv:Receiver<Packet>,
               packet_send:HashMap<NodeId, Sender<Packet>>) ->Self{
        //Assicura che il client sia conneso a uno o due droni
        assert!(
            connected_drone_ids.len() >= 1 && connected_drone_ids.len() <= 2,
            "Client must be connected to at least one and at most two drones!"
        );

        let host = Host::new(id, type_host, controller_send, controller_recv, packet_recv, packet_send);

        Self{
            host, connected_drone_ids, messages: Vec::new(), servers: HashMap::new()
        }
    }
    pub fn run(&mut self) {
        self.host.initiate_flood();
        loop {
            crossbeam_channel::select_biased! {
                recv(self.host.controller_recv) -> command => {
                    if let Ok(command) = command {
                        self.handle_command(command);
                    }
                }
                recv(self.host.packet_recv) -> packet => {
                    if let Ok(packet) = packet {
                        self.handle_packet(packet);
                    }
                }
                default => {
                    // Simula inattività, evita spinning continuo
                    std::thread::sleep(Duration::from_millis(500));
                }
            }
        }
    }

    fn handle_command(&mut self, command: ClientCommand<M>) {
        // Funzione ausiliaria per costruire e inviare un messaggio
        let send_message = |destination: NodeId, content: String| {
            let message = Message {
                session_id: generate_unique_session_id(), // Funzione che genera un ID univoco per la sessione
                content: M::from(content)
            };
            if let Err(err) = self.host.build_msg_fragment(destination, message) {
                self.host.log(&format!("Failed to send message: {}", err));
            }
        };

        match command {
            ClientCommand::FilesList => {
                if let Some(&server_id) = self.servers.get(&ServerTypes::Content) {
                    send_message(server_id, "FilesList?".to_string());
                } else {
                    self.host.log("No content server found.");
                }
            }
            ClientCommand::File(id) => {
                if let Some(&server_id) = self.servers.get(&ServerTypes::Content) {
                    send_message(server_id, format!("File? id:{}", id));
                } else {
                    self.host.log("No content server found.");
                }
            }
            ClientCommand::MediaList => {
                if let Some(&server_id) = self.servers.get(&ServerTypes::Content) {
                    send_message(server_id, "MediaList?".to_string());
                } else {
                    self.host.log("No content server found.");
                }
            }
            ClientCommand::Media(id) => {
                if let Some(&server_id) = self.servers.get(&ServerTypes::Content) {
                    send_message(server_id, format!("Media? id:{}", id));
                } else {
                    self.host.log("No content server found.");
                }
            }
            ClientCommand::RegistrationToChat => {
                if let Some(&server_id) = self.servers.get(&ServerTypes::Chat) {
                    send_message(server_id, format!("RegisterToChat id:{}", self.host.id));
                } else {
                    self.host.log("No chat server found.");
                }
            }
            ClientCommand::ClientList => {
                if let Some(&server_id) = self.servers.get(&ServerTypes::Chat) {
                    send_message(server_id, "ClientList?".to_string());
                } else {
                    self.host.log("No chat server found.");
                }
            }
            ClientCommand::MessageFor(client_id, message) => {
                if let Some(&server_id) = self.servers.get(&ServerTypes::Chat) {
                    send_message(server_id, format!("MessageFor id:{}, message:{:?}", client_id, message.content));
                } else {
                    self.host.log("No chat server found.");
                }
            }
            ClientCommand::ControllerShortcut(packet) => {
                self.handle_packet(packet);
                self.host.initiate_flood();
            }
            _ => {}
        }
    }
    fn handle_packet(&mut self, packet: Packet) {
        match packet.pack_type.clone() {
            PacketType::FloodResponse(response) => {
                self.host.process_flood_response(response);
            }
            PacketType::MsgFragment(fragment) => {
                 if let Some(message) = self.host.handle_msg_fragment(packet,fragment){
                     self.messages.push(message);
                 }else {
                     self.host.log("Failed to reassemble message fragment."); // ancora non ricevuto tutti i frammenti di quella sessione
                 }
            }
            PacketType::FloodRequest(flood_request) => {
                self.host.process_flood_request(flood_request, packet);
            }
            PacketType::Ack(ack) => {
                self.host.handle_ack(ack, packet.session_id);
            }
            PacketType::Nack(nack) => {
                self.host.handle_nack(nack, packet.session_id);
            }
        }
    }
}
