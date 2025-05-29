use wg_2024::controller::DroneCommand;
use wg_2024::network::NodeId;
use wg_2024::packet::Packet;
use host::{ClientCommand, ClientId, FileId, HostEvent, MediaId, NetCommand, ServerToClient};
use host::ClientCommand::ServerType;
use crate::backend::simulation_controller::{LogType, SimulationControllerData};

impl SimulationControllerData {

    /// - Drone Commands
    pub fn crash(&mut self, drone_id: NodeId) -> Result<(), String> {
        self.logs.push_log(LogType::DEBUG, format!("[Simulation Controller] - Sending drone crash to drone: {drone_id}").as_str());
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
                            None => { return Err(format!("[Simulation Controller] Drone {drone_id} does not exist")); }
                        }
                        for neighbor in neighbors.iter() {
                            self.remove_sender(drone_id, *neighbor)?
                        }
                        Ok(())
                    },
                    Err(e) => Err(format!("[Simulation Controller] - Error sending crash command - {}", e.to_string())),
                }
            },
            None => Err(format!("[Simulation Controller] - Drone {drone_id} command sender not found"))
        }
    }


    pub fn add_sender(&mut self, node_id: NodeId, sender_id: NodeId) -> Result<(), String> {
        /*match self.drones.get((node_id as usize) - 1) {
            Some(drone) => {
                    self.logs.push_log(LogType::INFO, format!("[Simulation Controller] - Drone {node_id} neighbors: {:?}", drone.connected_node_ids).as_str())
            },
            None => { return Err(format!("[Simulation Controller] Drone {node_id} does not exist")); }
        }*/
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
                                                    Ok(()) => {
                                                        //adding neighbors to connected_node_ids vector
                                                        for mut drone in self.drones.iter_mut() {
                                                            if drone.id == node_id && !drone.connected_node_ids.contains(&sender_id) {
                                                                drone.connected_node_ids.push(sender_id);
                                                            }
                                                            if drone.id == sender_id && !drone.connected_node_ids.contains(&node_id){
                                                                drone.connected_node_ids.push(node_id);
                                                            }
                                                        }
                                                        /*match self.drones.get((node_id as usize) - 1) {
                                                            Some(drone) => {
                                                                self.logs.push_log(LogType::INFO, format!("[Simulation Controller] - Drone {node_id} updated neighbors: {:?}", drone.connected_node_ids).as_str())
                                                            },
                                                            None => { return Err(format!("[Simulation Controller] Drone {node_id} does not exist")); }
                                                        }*/
                                                        Ok(()) },
                                                    Err(e) => Err(format!("[Simulation Controller] - {}", e.to_string())),
                                                }
                                            },
                                            None => Err(format!("[Simulation Controller] - Drone {sender_id} command sender not found")),
                                        }
                                    },
                                    Err(e) => Err(format!("[Simulation Controller] - {}", e.to_string())),
                                }
                            },
                            None => Err(format!("[Simulation Controller] - Drone {node_id} command sender not found")),
                        }
                    },
                    None => Err(format!("[Simulation Controller] - Drone {node_id} packet sender not found")),
                }
            },
            None => Err(format!("[Simulation Controller] - Drone {sender_id} packet sender not found")),
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
                            None => Err(format!("[Simulation Controller] - Drone {sender_id} command sender not found")),
                        }
                    },
                    Err(e) => Err(e.to_string())
                }
            },
            None => Err(format!("[Simulation Controller] - Drone {node_id} command sender not found")),
        }
    }

    pub fn set_pdr(&mut self, drone_id: NodeId, new_pdr: f32) -> Result<(), String> {
        let valid_pdr;
        if new_pdr >= 0.0 && new_pdr <= 1.0 {
            valid_pdr = new_pdr;
        } else {
            return Err(format!("[Simulation Controller] - Invalid pdr value: {} | 0.0 < pdr < 1.0", new_pdr));
        }
        let command = DroneCommand::SetPacketDropRate(valid_pdr);
        match self.drone_command_sender.get(&drone_id) {
            Some(drone_command_sender) => {
                match drone_command_sender.send(command) {
                    Ok(()) => {
                        self.logs.push_log(LogType::INFO, format!("[Simulation Controller] - Changing Drone {drone_id} pdr to: {valid_pdr}").as_str());
                        Ok(()) },
                    Err(e) => Err(e.to_string()),
                }
            },
            None => Err(format!("[Simulation Controller] - Drone {drone_id} command sender not found")),
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

    fn send_client_command(&self, client_id: ClientId, command: NetCommand) -> Result<(), String> {
        match self.client_command_sender.get(&client_id) {
            Some(client_command_sender) => {
                match client_command_sender.send(command) {
                    Ok(()) => Ok(()),
                    Err(e) => Err(format!("[Simulation Controller] - {}", e.to_string())),
                }
            },
            None => Err(format!("[Simulation Controller] - Client {client_id} command sender not found")),
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

    pub fn message_for(&self, sender_client_id: ClientId, receiver_client_id: ClientId, message: String) -> Result<(), String> {
                let command = NetCommand::Net(ClientCommand::MessageFor(receiver_client_id, message));
                self.send_client_command(sender_client_id, command)
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
                        ServerToClient::ServerType(server_type) => format!("[Simulation Controller] - Server type: {:?}", server_type),
                        ServerToClient::FilesList(file_list) => format!("[Simulation Controller] - Files list received: {:?}", file_list),
                        ServerToClient::File(file_id, file) => format!("[Simulation Controller] - File {} received: {:?}", file_id, file),
                        ServerToClient::MediaList(media_list) => format!("[Simulation Controller] - Media list received: {:?}", media_list),
                        ServerToClient::Media(media_id, image) => format!("[Simulation Controller] - File {} received: {:?}", media_id, image),
                        ServerToClient::ClientList(client_list) => format!("[Simulation Controller] - Client list received: {:?}", client_list),
                        ServerToClient::MessageFrom(client_id, message) => format!("[Simulation Controller] - Message from Client {} received: {:?}", client_id, message),
                        ServerToClient::ErrNotFound => "[Simulation Controller] - Err Not Found".to_owned(),
                        ServerToClient::ErrClientId => "[Simulation Controller] - Err Client Id Not Found".to_owned(),
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