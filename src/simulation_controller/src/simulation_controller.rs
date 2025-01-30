use bevy::prelude::*;
use crate::client::ClientPlugin;
use crate::drone::DronePlugin;
use crate::server::ServerPlugin;
use crate::ui::UiPlugin;

use crossbeam_channel::{unbounded, Receiver, Sender};
use std::collections::HashMap;
use std::{fs, thread};
use wg_2024::config::Config;
use wg_2024::config::Drone as DroneConfig;
use wg_2024::config::Client as ClientConfig;
use wg_2024::config::Server as ServerConfig;
use wg_2024::controller::*;
use wg_2024::network::NodeId;
use wg_2024::drone::Drone;
use wg_2024::packet::Packet;
use drone::Drone as LeDron;

/// Simulation Controller struct contains:
/// - the drones
/// - the clients
/// - the servers
/// - the command sender channels
/// - the event receiver channels
#[derive(Resource)]
pub struct SimulationController {
    pub drones: Vec<DroneConfig>,   //vector of drones
    pub clients: Vec<ClientConfig>, //vector of clients
    pub servers: Vec<ServerConfig>, //vector of servers
    pub drone_command_sender: HashMap<NodeId, Sender<DroneCommand>>,  //command sender channel to drone
    pub drone_event_receiver: HashMap<NodeId, Receiver<DroneEvent>>,   //event receiver channel from drone
    pub packet_sender: HashMap<NodeId, Sender<Packet>>,         //packet sender of each node
    pub packet_receiver: HashMap<NodeId, Receiver<Packet>>,     //packet receiver of each node
    //pub client_command_sender: HashMap<NodeId, Sender<>>
}


impl SimulationController {
    pub fn new(config: Config,
               drone_command_sender: HashMap<NodeId, Sender<DroneCommand>>,
               drone_event_receiver: HashMap<NodeId, Receiver<DroneEvent>>,
               packet_sender: HashMap<NodeId, Sender<Packet>>,
               packet_receiver: HashMap<NodeId, Receiver<Packet>>,
    ) -> Self {
        let mut drones = vec![];
        for drone in config.drone {
            drones.push(drone);
        }
        let mut clients = vec![];
        for client in config.client {
            clients.push(client);
        }
        let mut servers = vec![];
        for server in config.server {
            servers.push(server);
        }
        Self {
            drones,
            clients,
            servers,
            drone_command_sender,
            drone_event_receiver,
            packet_sender,
            packet_receiver,
        }
    }
}

pub fn main() {
    // translating the config file to know the topology
    let config = parse_config("/Volumes/SSD FILIPPO/UNI/Advanced Programming/Drone Project/LeDron-James/src/simulation_controller/src/config");


    // - packet_channels contains the NodeId linked with its packet channels
    let mut packet_channels = HashMap::new();
    for drone in config.drone.iter() {
        packet_channels.insert(drone.id, unbounded());
    }
    for client in config.client.iter() {
        packet_channels.insert(client.id, unbounded());
    }
    for server in config.server.iter() {
        packet_channels.insert(server.id, unbounded());
    }

    let mut handles = Vec::new();

    // - command_sender contains the channels senders to send commands to drones
    let mut command_sender = HashMap::new();
    // - event_receiver contains the channels receiver to receive events from drones
    let mut event_receiver = HashMap::new();

    //let (node_event_send, _node_event_recv) = unbounded();

    for drone in config.clone().drone.into_iter() {
        // command channel
        let (controller_drone_send, controller_drone_recv) = unbounded();
        // event channel
        let (drone_event_sender, drone_event_recv) = unbounded();
        //creating the HashMap needed to send commands to drones
        command_sender.insert(drone.id, controller_drone_send);
        //creating the HashMap needed to receive events from drones
        event_receiver.insert(drone.id, drone_event_recv);
        // let node_event_send = node_event_send.clone();
        // packet
        let packet_recv = packet_channels[&drone.id].1.clone();
        let packet_send = drone
            .connected_node_ids
            .into_iter()
            .map(|id| (id, packet_channels[&id].0.clone()))
            .collect();

        handles.push(thread::spawn(move || {
            let mut drone = LeDron::new(
                drone.id,
                drone_event_sender,
                controller_drone_recv,
                packet_recv,
                packet_send,
                drone.pdr,
            );

            drone.run();
        }));
    }
    let mut packet_senders = HashMap::new();
    let mut packet_receivers = HashMap::new();
    for (node_id, (sender, receiver)) in packet_channels.iter() {
        packet_senders.insert(*node_id, sender.clone());
        packet_receivers.insert(*node_id, receiver.clone());
    }
    let controller = SimulationController::new(config, command_sender, event_receiver, packet_senders, packet_receivers);

    // controller.crash_all();

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_state(OptionMenu::default())
        .insert_resource(controller)
        .insert_resource(SelectedNode::default())
        .init_gizmo_group::<MyGizmos>()
        .add_plugins((
            UiPlugin,
            DronePlugin,
            ClientPlugin,
            ServerPlugin,
        ))
        .insert_resource(ClearColor(Color::srgb(0.3, 0.3, 0.3)))
        .run();

    while let Some(handle) = handles.pop() {
        handle.join().unwrap();
    }
}

pub fn parse_config(file: &str) -> Config {
    let file_str = fs::read_to_string(file).unwrap();
    toml::from_str(&file_str).unwrap()
}

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum OptionMenu {
    #[default]
    Off,
    On
}

#[derive(Resource, Default, Debug)]
pub struct SelectedNode {
    pub node_id: Option<NodeId>
}

#[derive(Default, Reflect, GizmoConfigGroup)]
struct MyGizmos {}
