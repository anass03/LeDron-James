use std::fmt::{Display, Formatter};
use wg_2024::controller::DroneCommand;
use wg_2024::network::NodeId;
use wg_2024::packet::Packet;
use host::{ClientId, ClientNetCommand, FileId, HostCommand, HostEvent, MediaId, ServerToClient};
use client::ClientCommand;
use host::ClientNetCommand::{FilesList, ServerType};
use crate::backend::simulation_controller::{LogType, SimulationControllerData};

#[derive(Default, Clone, Copy)]
enum NodeType {
    #[default]
    Drone,
    Client,
    Server,
}

impl Display for NodeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeType::Drone => f.write_str("Drone"),
            NodeType::Client => f.write_str("Client"),
            NodeType::Server => f.write_str("Server"),
        }
    }
}

impl NodeType {
    fn from(sim: & SimulationControllerData, node_id: NodeId) -> NodeType {
        for drone in sim.drones.iter() {
            if drone.id == node_id {
                return NodeType::Drone;
            }
        }
        for client in sim.clients.iter() {
            if client.id == node_id {
                return NodeType::Client;
            }
        }
        for server in sim.servers.iter() {
            if server.id == node_id {
                return NodeType::Server;
            }
        }
        NodeType::default()
    }
}

impl SimulationControllerData {

    /// - Drone Commands
    pub fn crash(&mut self, drone_id: NodeId) -> Result<(), String> {
        self.log(LogType::DEBUG, format!("Sending drone crash to drone: {drone_id}"));
        let command = DroneCommand::Crash;
        match self.drone_command_sender.get(&drone_id) {
            Some(drone_command_sender) => {
                match drone_command_sender.send(command) {
                    Ok(_) => {
                        //removing all neighboring channels
                        let mut neighbors = vec![];
                        match self.drones.get((drone_id as usize) - 1) {
                            Some(drone) => {
                                for neighbor in drone.connected_node_ids.iter() {
                                    neighbors.push(*neighbor);
                                }
                            },
                            None => { return Err(format!("Drone {drone_id} does not exist")); }
                        }
                        for neighbor in neighbors.iter() {
                            self.remove_sender(drone_id, *neighbor)?
                        }
                        Ok(())
                    },
                    Err(e) => Err(format!("Error sending crash command - {}", e.to_string())),
                }
            },
            None => Err(format!("Drone {drone_id} command sender not found"))
        }
    }

    fn mono_add_sender(&mut self, first_type: NodeType, first_id: NodeId, second_type: NodeType, second_id: NodeId) -> Result<(), String> {
        self.log(LogType::DEBUG, format!("{first_type} + {second_type}"));
        //checking if packet sender for forward node exists
        match self.packet_sender.get(&second_id) {
            Some(forward_packet_sender) => {
                //checking if packet sender for backwards node exists
                match first_type {
                    NodeType::Drone => {
                        match self.drone_command_sender.get(&first_id) {
                            Some(forward_drone_command_sender) => {
                                let forward_command = DroneCommand::AddSender(second_id, forward_packet_sender.clone());
                                //trying to send command forward
                                match forward_drone_command_sender.send(forward_command) {
                                    Ok(()) => {
                                        self.log(LogType::DEBUG, format!("Asking to add {second_type} {second_id} sender to {first_type} {first_id}"));
                                    },
                                    Err(e) => return Err(e.to_string()),
                                }
                            },
                            None => return Err(format!("Drone {first_id} command sender not found")),
                        }
                    },
                    NodeType::Client => {
                        match self.client_command_sender.get(&first_id) {
                            Some(forward_client_command_sender) => {
                                let forward_command = ClientCommand::Internal(HostCommand::AddSender(second_id, forward_packet_sender.clone()));
                                //trying to send command forward
                                match forward_client_command_sender.send(forward_command) {
                                    Ok(()) => {
                                        self.log(LogType::DEBUG, format!("Asking to add {second_type} {second_id} sender to {first_type} {first_id}"));
                                    },
                                    Err(e) => return Err(e.to_string()),
                                }
                            },
                            None => return Err(format!("Client {first_id} command sender not found💥")),
                        }
                    },
                    NodeType::Server => {
                        match self.server_command_sender.get(&first_id) {
                            Some(forward_server_command_sender) => {
                                let forward_command = HostCommand::AddSender(second_id, forward_packet_sender.clone());
                                //trying to send command forward
                                match forward_server_command_sender.send(forward_command) {
                                    Ok(()) => {
                                        self.log(LogType::DEBUG, format!("Asking to add {second_type} {second_id} sender to {first_type} {first_id}"));
                                    },
                                    Err(e) => return Err(e.to_string()),
                                }
                            },
                            None => return Err(format!("Server {first_id} command sender not found")),
                        }
                    }
                }
            },
            None => return Err(format!("{second_type} {second_id} packet sender not found")),
        }
        Ok(())
    }

    pub fn add_sender(&mut self, node_id: NodeId, sender_id: NodeId) -> Result<(), String> {
        /*match self.drones.get((node_id as usize) - 1) {
            Some(drone) => {
                    self.logs.push_log(LogType::INFO, format!("[Simulation Controller] - Drone {node_id} neighbors: {:?}", drone.connected_node_ids).as_str())
            },
            None => { return Err(format!("[Simulation Controller] Drone {node_id} does not exist")); }
        }*/
        let forward_node_type = NodeType::from(&self, node_id);
        let backwards_node_type = NodeType::from(&self, sender_id);

        self.mono_add_sender(forward_node_type, node_id, backwards_node_type, sender_id)?;
        self.mono_add_sender(backwards_node_type, sender_id, forward_node_type, node_id)
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
                                    Ok(()) => {
                                        //removing neighbors from connected_node_ids vector
                                        for mut drone in self.drones.iter_mut() {
                                            if drone.id == node_id && drone.connected_node_ids.contains(&sender_id) {
                                                drone.connected_node_ids.retain(|x| *x != sender_id);
                                            }
                                            if drone.id == sender_id && drone.connected_node_ids.contains(&node_id){
                                                drone.connected_node_ids.retain(|x| *x != node_id);
                                            }
                                        }
                                        Ok(()) },
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
            return Err(format!("Invalid PDR value: {} | 0.0 < PDR < 1.0", new_pdr));
        }
        let command = DroneCommand::SetPacketDropRate(valid_pdr);
        match self.drone_command_sender.get(&drone_id) {
            Some(drone_command_sender) => {
                match drone_command_sender.send(command) {
                    Ok(()) => {
                        self.logs.push_log(LogType::INFO, format!("Changing Drone {drone_id} PDR to: {valid_pdr}").as_str());
                        Ok(()) },
                    Err(e) => Err(e.to_string()),
                }
            },
            None => Err(format!("Drone {drone_id} command sender not found")),
        }
    }

    /// - Drone Events

    pub fn handle_drone_event(&mut self) -> Result<(), String> {
        /*for receiver in self.drone_event_receiver.try_iter() {
            match receiver {
                DroneEvent::PacketSent(packet) => {
                    let sender_id = packet.routing_header.hops[packet.routing_header.hop_index - 1 ];
                    handle_packet(&packet);
                    let session_id = packet.session_id;
                    //self.logs.push_log(LogType::INFO, format!("Drone ID {:?} - Sent packet [ Origin: {sender_id}, Session ID: {session_id} ]", packet.routing_header.hops[packet.routing_header.hop_index - 1]).as_str());
                },
                DroneEvent::PacketDropped(packet) => {
                    let sender_id = packet.routing_header.hops[packet.routing_header.hop_index - 1 ];
                    handle_packet(&packet);
                    let session_id = packet.session_id;
                    //self.logs.push_log(LogType::INFO, format!("Drone ID {:?} - Dropped packet [ Origin: {sender_id}, Session ID: {session_id}", packet.routing_header.hops[packet.routing_header.hop_index - 1]).as_str());
                },
                DroneEvent::ControllerShortcut(packet) => {
                    let sender_id = packet.routing_header.hops[packet.routing_header.hop_index - 1 ];
                    handle_packet(&packet);
                    let session_id = packet.session_id;
                    //self.logs.push_log(LogType::INFO, format!("Drone ID {:?} - Sent packet shortcut [ Origin: {sender_id}, Session ID: {session_id}", packet.routing_header.hops[packet.routing_header.hop_index - 1]).as_str());
                },
            }
        }*/

        /*for drone in self.drones.iter() {
            self.logs.push_log(LogType::INFO, format!("[Simulation Controller] - Drone {} neighbors: {:?}",drone.id, drone.connected_node_ids).as_str())
        }*/
        Ok(())
    }

    /// - Client Commands

    fn send_client_command(&self, client_id: ClientId, command: ClientCommand) -> Result<(), String> {
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
        let command = ClientCommand::Net(ServerType);
        self.send_client_command(client_id, command)
    }

    pub fn request_files_list(&self, client_id: ClientId) -> Result<(), String> {
        let command = ClientCommand::Net(FilesList);
        self.send_client_command(client_id, command)
    }

    pub fn request_file(&self, client_id: ClientId, file_id: FileId) -> Result<(), String> {
        let command = ClientCommand::Net(ClientNetCommand::File(file_id));
        self.send_client_command(client_id, command)
    }

    pub fn request_media_list(&self, client_id: ClientId) -> Result<(), String> {
        let command = ClientCommand::Net(ClientNetCommand::MediaList);
        self.send_client_command(client_id, command)
    }

    pub fn request_media(&self, client_id: ClientId, media_id: MediaId) -> Result<(), String> {
        let command = ClientCommand::Net(ClientNetCommand::Media(media_id));
        self.send_client_command(client_id, command)
    }

    pub fn request_registration_to_chat(&self, client_id: ClientId) -> Result<(), String> {
        let command = ClientCommand::Net(ClientNetCommand::RegistrationToChat);
        self.send_client_command(client_id, command)
    }

    pub fn request_client_list(&self, client_id: ClientId) -> Result<(), String> {
        let command = ClientCommand::Net(ClientNetCommand::ClientList);
        self.send_client_command(client_id, command)
    }

    pub fn message_for(&self, sender_client_id: ClientId, receiver_client_id: ClientId, message: String) -> Result<(), String> {
                let command = ClientCommand::Net(ClientNetCommand::MessageFor(receiver_client_id, message));
                self.send_client_command(sender_client_id, command)
    }

    pub fn controller_shortcut(&self, client_id: ClientId, packet: Packet) -> Result<(), String> {
        let command = ClientCommand::ControllerShortcut(packet);
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
                HostEvent::PacketSent(_packet) => {
                    //Some(format!("💥Packet Sent: {:?}", packet))
                    None
                },
            }
        }
        None
    }

    pub fn handle_log_event(&mut self){
        for logs in self.log_receiver.try_iter(){
            self.logs.push_log(LogType::INFO, logs.as_str());
        }
    }
}

/*fn handle_packet (_packet: &Packet) {
    match packet.clone().pack_type {
        PacketType::MsgFragment(fragment) => {}
        PacketType::Ack(ack) => {}
        PacketType::Nack(nack) => {}
        PacketType::FloodRequest(flood_request) => {}
        PacketType::FloodResponse(flood_response) => {}
    }
}*/