use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::network::NodeId;
use wg_2024::packet::Packet;
use host::{ClientCommand, ClientId, FileId, HostEvent, MediaId, Message, NetCommand, ServerToClient};
use host::ClientCommand::ServerType;
use host::HostEvent::PacketSent;
use wg_internal::packet::PacketType;
use crate::backend::simulation_controller::{log, LogType, SimulationControllerData};

impl SimulationControllerData {

    /// - Drone Commands
    pub fn crash(&mut self, drone_id: NodeId) -> Result<(), String> {
        let command = DroneCommand::Crash;
        match self.drone_command_sender.get(&drone_id) {
            Some(drone_command_sender) => {
                match drone_command_sender.send(command) {
                    Ok(_) => {Ok(())
                        /*//removing all connected channels
                        for connected_node in self.drones.*/
                    },
                    Err(e) => Err(e.to_string()),
                }
            },
            None => Err(format!("Drone {drone_id} command sender not found"))
        }
    }

    pub fn add_sender(&mut self, node_id: NodeId, sender_id: NodeId) -> Result<(), String> {
        //checking if packet sender for forward node exists
        match self.packet_sender.get(&sender_id) {
            Some(forward_packet_sender) => {
                //preparing AddSender command forward
                let forward_command = DroneCommand::AddSender(sender_id, forward_packet_sender.clone());
                //checking if packet sender for backwards node exists
                match self.packet_sender.get(&node_id) {
                    Some(backwards_packet_sender) => {
                        //preparing AddSender command backwards
                        let backwards_command = DroneCommand::AddSender(node_id, backwards_packet_sender.clone());
                        //if the two commands are ready, check if command sender for forward node exists
                        match self.drone_command_sender.get(&node_id) {
                            Some(forward_drone_command_sender) => {
                                //trying to send command forward
                                match forward_drone_command_sender.send(forward_command) {
                                    Ok(()) => {
                                        //if successful, check if command sender for backwards node exists
                                        match self.drone_command_sender.get(&sender_id) {
                                            Some(backward_command_sender) => {
                                                //trying to send command backwards
                                                match backward_command_sender.send(backwards_command) {
                                                    Ok(()) => Ok(()),
                                                    Err(e) => Err(e.to_string()),
                                                }
                                            },
                                            None => Err(format!("Drone {sender_id} command sender not found")),
                                        }
                                    },
                                    Err(e) => Err(e.to_string())
                                }
                            },
                            None => Err(format!("Drone {node_id} command sender not found")),
                        }
                    },
                    None => Err(format!("Drone {node_id} packet sender not found")),
                }
            },
            None => Err(format!("Drone {sender_id} packet sender not found")),
        }
    }

    pub fn remove_sender(&mut self, node_id: NodeId, sender_id: NodeId) -> Result<(), String> {

        let forward_command = DroneCommand::RemoveSender(sender_id);
        let backwards_command = DroneCommand::RemoveSender(node_id);
        //checking if command sender for forward node exists
        match self.drone_command_sender.get(&node_id) {
            Some(forward_drone_command_sender) => {
                //trying to send command forward
                match forward_drone_command_sender.send(forward_command) {
                    Ok(()) => {
                        //if successful, check if command sender for backwards node exists
                        match self.drone_command_sender.get(&sender_id) {
                            Some(backward_command_sender) => {
                                //trying to send command backwards
                                match backward_command_sender.send(backwards_command) {
                                    Ok(()) => Ok(()),
                                    Err(e) => Err(e.to_string()),
                                }
                            },
                            None => Err(format!("Drone {sender_id} command sender not found")),
                        }
                    },
                    Err(e) => Err(e.to_string())
                }
            },
            None => Err(format!("Drone {node_id} command sender not found")),
        }
    }

    pub fn set_pdr(&mut self, drone_id: NodeId, new_pdr: f32) -> Result<(), String> {
        let valid_pdr;
        if new_pdr >= 0.0 && new_pdr <= 1.0 {
            valid_pdr = new_pdr;
        } else {
            return Err(format!("Invalid pdr value: {} | 0.0 < pdr < 1.0", new_pdr));
        }
        let command = DroneCommand::SetPacketDropRate(new_pdr);
        self.drone_command_sender.get(&drone_id).unwrap().send(DroneCommand::SetPacketDropRate(new_pdr)).expect("error setting pdr");
        match self.drone_command_sender.get(&drone_id) {
            Some(drone_command_sender) => {
                match drone_command_sender.send(command) {
                    Ok(()) => Ok(()),
                    Err(e) => Err(e.to_string()),
                }
            },
            None => Err(format!("Drone {drone_id} command sender not found")),
        }
    }

    /// - Drone Events

    pub fn handle_drone_event(&mut self) -> Result<(), String> {
        for receiver in self.drone_event_receiver.try_iter() {
            match receiver {
                DroneEvent::PacketSent(packet) => {
                    let sender_id = packet.routing_header.hops[packet.routing_header.hop_index - 1 ];
                    handle_packet(&packet);
                    let session_id = packet.session_id;
                    self.logs.push_log(LogType::INFO, format!("Drone ID {:?} - Sent packet [ Origin: {sender_id}, Session ID: {session_id} ]", packet.routing_header.hops[packet.routing_header.hop_index - 1]).as_str());
                },
                DroneEvent::PacketDropped(packet) => {
                    let sender_id = packet.routing_header.hops[packet.routing_header.hop_index - 1 ];
                    handle_packet(&packet);
                    let session_id = packet.session_id;
                    self.logs.push_log(LogType::INFO, format!("Drone ID {:?} - Dropped packet [ Origin: {sender_id}, Session ID: {session_id}", packet.routing_header.hops[packet.routing_header.hop_index - 1]).as_str());
                },
                DroneEvent::ControllerShortcut(packet) => {
                    let sender_id = packet.routing_header.hops[packet.routing_header.hop_index - 1 ];
                    handle_packet(&packet);
                    let session_id = packet.session_id;
                    self.logs.push_log(LogType::INFO, format!("Drone ID {:?} - Sent packet shortcut [ Origin: {sender_id}, Session ID: {session_id}", packet.routing_header.hops[packet.routing_header.hop_index - 1]).as_str());
                },
            }
        }
        Ok(())
    }

    /// - Client Commands

    fn send_client_command(&self, client_id: ClientId, command: NetCommand) -> Result<(), String> {
        match self.client_command_sender.get(&client_id) {
            Some(client_command_sender) => {
                match client_command_sender.send(command) {
                    Ok(()) => Ok(()),
                    Err(e) => Err(e.to_string()),
                }
            },
            None => Err(format!("Client {client_id} command sender not found")),
        }
    }

    pub fn request_server_type(&self, client_id: ClientId) -> Result<(), String> {
        let command = NetCommand::Net(ServerType);
        self.send_client_command(client_id, command)
    }

    pub fn request_files_list(&self, client_id: ClientId) -> Result<(), String> {
        let command = NetCommand::Net(ClientCommand::FilesList);
        self.send_client_command(client_id, command)
    }

    pub fn request_file(&self, client_id: ClientId, file_id: FileId) -> Result<(), String> {
        let command = NetCommand::Net(ClientCommand::File(file_id));
        self.send_client_command(client_id, command)
    }

    pub fn request_media_list(&self, client_id: ClientId) -> Result<(), String> {
        let command = NetCommand::Net(ClientCommand::MediaList);
        self.send_client_command(client_id, command)
    }

    pub fn request_media(&self, client_id: ClientId, media_id: MediaId) -> Result<(), String> {
        let command = NetCommand::Net(ClientCommand::Media(media_id));
        self.send_client_command(client_id, command)
    }

    pub fn request_registration_to_chat(&self, client_id: ClientId) -> Result<(), String> {
        let command = NetCommand::Net(ClientCommand::RegistrationToChat);
        self.send_client_command(client_id, command)
    }

    pub fn request_client_list(&self, client_id: ClientId) -> Result<(), String> {
        let command = NetCommand::Net(ClientCommand::ClientList);
        self.send_client_command(client_id, command)
    }

    pub fn message_for(&self, sender_client_id: ClientId, receiver_client_id: ClientId, message: Message) -> Result<(), String> {
        match self.clients.get(receiver_client_id as usize) {
            Some(_) => {
                let command = NetCommand::Net(ClientCommand::MessageFor(receiver_client_id, message));
                self.send_client_command(sender_client_id, command)
            },
            None => Err(format!("Client {receiver_client_id} not found")),
        }

    }

    pub fn controller_shortcut(&self, client_id: ClientId, packet: Packet) -> Result<(), String> {
        let command = NetCommand::ControllerShortcut(packet);
        self.send_client_command(client_id, command)
    }

    // /// - Client Events

    pub fn handle_client_event(&self) -> Option<String> {
        for received in self.host_event_receiver.try_iter() {
            return match received {
                HostEvent::PacketReconstructed(server_to_client) => {
                    Some(match server_to_client {
                        ServerToClient::ServerType(server_type) => format!("Server type: {:?}", server_type),
                        ServerToClient::FilesList(file_list) => format!("Files list received: {:?}", file_list),
                        ServerToClient::File(file_id, file) => format!("File {} received: {:?}", file_id, file),
                        ServerToClient::MediaList(media_list) => format!("Media list received: {:?}", media_list),
                        ServerToClient::Media(media_id, image) => format!("File {} received: {:?}", media_id, image),
                        ServerToClient::ClientList(client_list) => format!("Client list received: {:?}", client_list),
                        ServerToClient::MessageFrom(client_id, message) => format!("Message from Client {} received: {:?}", client_id, message),
                        ServerToClient::ErrNotFound => "Err Not Found".to_owned(),
                        ServerToClient::ErrClientId => "Err Client Id Not Found".to_owned(),
                    })
                },
                HostEvent::PacketSent(packet) => {
                    Some(format!("Packet Sent: {:?}", packet))
                },
            }
        }
        None
    }
}

fn handle_packet (packet: &Packet) {
    match packet.clone().pack_type {
        PacketType::MsgFragment(fragment) => {}
        PacketType::Ack(ack) => {}
        PacketType::Nack(nack) => {}
        PacketType::FloodRequest(flood_request) => {}
        PacketType::FloodResponse(flood_response) => {}
    }
}