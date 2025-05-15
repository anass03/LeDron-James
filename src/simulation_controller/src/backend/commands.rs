use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::network::NodeId;
use wg_2024::packet::Packet;
use host::{ClientCommand, ClientId, FileId, HostEvent, MediaId, Message, NetCommand, ServerToClient};
use host::HostEvent::PacketSent;
use crate::backend::simulation_controller::{log, SimulationControllerData};

impl SimulationControllerData {

    /// - Drone Commands
    pub fn crash(&mut self, drone_id: NodeId) {
        self.drone_command_sender.get(&drone_id).unwrap().send(DroneCommand::Crash).expect("error crashing drone");
    }

    pub fn add_sender(&mut self, node_id: NodeId, sender_id: NodeId) {
        let sender_forward = DroneCommand::AddSender(sender_id, self.packet_sender.get(&sender_id).unwrap().clone());
        let sender_backwards = DroneCommand::AddSender(node_id, self.packet_sender.get(&node_id).unwrap().clone());
        self.drone_command_sender.get(&node_id).unwrap().send(sender_forward).expect("error adding sender forward");
        self.drone_command_sender.get(&sender_id).unwrap().send(sender_backwards).expect("error adding sender backwards");
    }

    pub fn remove_sender(&mut self, node_id: NodeId, sender_id: NodeId) {
        let sender_forward = DroneCommand::RemoveSender(sender_id);
        let sender_backwards = DroneCommand::RemoveSender(node_id);
        self.drone_command_sender.get(&node_id).unwrap().send(sender_forward).expect("error removing sender forward");
        self.drone_command_sender.get(&sender_id).unwrap().send(sender_backwards).expect("error removing sender backwards");
    }

    pub fn set_pdr(&mut self, drone_id: NodeId, new_pdr: f32) {
        self.drone_command_sender.get(&drone_id).unwrap().send(DroneCommand::SetPacketDropRate(new_pdr)).expect("error setting pdr");
    }

    /// - Drone Events

    pub fn handle_drone_packet_sent(&self) {
        for receiver in self.drone_event_receiver.iter() {
            for recieved in receiver.1.iter() {
                match recieved {
                    DroneEvent::PacketSent(packet) => {
                        log(format!("Packet sent: {:?}", packet).as_str());
                    },
                    DroneEvent::PacketDropped(packet) => {
                        log(format!("Packet dropped: {:?}", packet).as_str());
                    },
                    DroneEvent::ControllerShortcut(packet) => {
                        log(format!("Controller shortcut: {:?}", packet).as_str());
                    }
                }
            }
        }
    }

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

    /// - Client Events

    pub fn files_list(&self) {
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
    }
}