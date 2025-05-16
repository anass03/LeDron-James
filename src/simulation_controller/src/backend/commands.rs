use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::network::NodeId;
use wg_2024::packet::Packet;
use host::{ClientCommand, ClientId, FileId, HostEvent, MediaId, Message, NetCommand, ServerToClient};
use host::HostEvent::PacketSent;
use crate::backend::simulation_controller::{log, SimulationControllerData};

impl SimulationControllerData {

    /// - Drone Commands
    pub fn crash(&mut self, drone_id: NodeId) -> Result<(), String> {
        match self.drone_command_sender.get(&drone_id) {
            Some(drone_command_sender) => {
                match drone_command_sender.send(DroneCommand::Crash) {
                    Ok(_) => Ok(()),
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

    /*/// - Drone Events

    pub fn handle_drone_packet_sent(&self) -> Result<(), String> {
        for receiver in self.drone_event_receiver.iter() {
            match receiver {
                DroneEvent::PacketSent(packet) => Ok(()),
                DroneEvent::PacketDropped(packet) => Ok(()),
                DroneEvent::ControllerShortcut(packet) => Ok(()),
            }
        }
    }*/

    /// - Client Commands

    pub fn request_files_list(&self, client_id: ClientId) {
        self.client_command_sender.get(&client_id).expect("no such Client Id").send(NetCommand::Net(ClientCommand::FilesList)).expect("error sending files list request");
    }

    pub fn request_file(&self, client_id: ClientId, file_id: FileId) {
        self.client_command_sender.get(&client_id).expect("no such Client Id").send(NetCommand::Net(ClientCommand::File(file_id))).expect("error sending file request");
    }

    pub fn request_media_list(&self, client_id: ClientId) {
        self.client_command_sender.get(&client_id).expect("no such Client Id").send(NetCommand::Net(ClientCommand::MediaList)).expect("error sending media list request");
    }

    pub fn request_media(&self, client_id: ClientId, media_id: MediaId) {
        self.client_command_sender.get(&client_id).expect("no such Client Id").send(NetCommand::Net(ClientCommand::Media(media_id))).expect("error sending media request");
    }

    pub fn request_registration_to_chat(&self, client_id: ClientId) {
        self.client_command_sender.get(&client_id).expect("no such Client Id").send(NetCommand::Net(ClientCommand::RegistrationToChat)).expect("error sending registration to chat request");
    }

    pub fn request_client_list(&self, client_id: ClientId) {
        self.client_command_sender.get(&client_id).expect("no such Client Id").send(NetCommand::Net(ClientCommand::ClientList)).expect("error sending client list request");
    }

    pub fn message_for(&self, sender_client_id: ClientId, receiver_client_id: ClientId, message: Message) {
        self.client_command_sender.get(&sender_client_id).expect("no such Client Id").send(NetCommand::Net(ClientCommand::MessageFor(receiver_client_id, message))).expect("error sending message");
    }

    pub fn controller_shortcut(&self, sender_client_id: ClientId, packet: Packet) {
        self.client_command_sender.get(&sender_client_id).expect("no such Client Id").send(NetCommand::ControllerShortcut(packet)).expect("error sending controller shortcut");
    }

    // /// - Client Events

   /* pub fn files_list(&self) {
        for receiver in self.client_event_receiver.iter() {
            for received in receiver.1.iter() {
                match received {
                    HostEvent::PacketReconstructed(server_to_client) => {
                        match server_to_client {
                            ServerToClient::ServerType(server_type) => { log(format!("Client {} got {:?}", receiver.0, server_type).as_str())},
                            ServerToClient::FilesList(file_list) => { log(format!("Files list received: {:?}", file_list).as_str())},
                            ServerToClient::File(file_id, file) => { log(format!("File {} received: {:?}", file_id, file).as_str())},
                            ServerToClient::MediaList(media_list) => { log(format!("Media list received: {:?}", media_list).as_str())},
                            ServerToClient::Media(media_id, image) => { log(format!("File {} received: {:?}", media_id, image).as_str())},
                            ServerToClient::ClientList(client_list) => { log(format!("Client list received: {:?}", client_list).as_str())},
                            ServerToClient::MessageFrom(client_id, message) => { log(format!("Message from  Client {} received: {:?}", client_id, message).as_str())},
                            ServerToClient::ErrNotFound => { log("Err Not Found"); },
                            ServerToClient::ErrClientId => { log("Err Client Id Not Found"); },
                        }
                    },
                    PacketSent(packet) => { log(format!("Packet Sent: {:?}", packet).as_str()); },
                }
            }
        }
    }*/
}