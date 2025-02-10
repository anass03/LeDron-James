use bevy::prelude::*;
use bevy::color::Color as BColor;
use crate::client::ClientPlugin;
use crate::drone::DronePlugin;
use crate::server::ServerPlugin;
use crate::ui::UiPlugin;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::collections::HashMap;
use std::{fs, thread};
use std::thread::JoinHandle;
use bevy::window::WindowCreated;
use wg_2024::config::Config;
use wg_2024::config::Drone as DroneConfig;
use wg_2024::config::Client as ClientConfig;
use wg_2024::config::Server as ServerConfig;
use wg_2024::controller::*;
use wg_2024::network::NodeId;
use wg_2024::drone::Drone;
use wg_2024::packet::{NodeType, Packet};
use drone::Drone as LeDron;
use client::Client as LeClient;
use host::ClientCommand;
use server::ContentServer as LeContentServer;
use server::chatserver::ChatServer as LeChatServer;
use colored::*;
use crate::ui::option_menu::components::SenderStatus;

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
    pub client_command_sender: HashMap<NodeId, Sender<ClientCommand>>,
    pub client_event_receiver: HashMap<NodeId, Receiver<DroneEvent>>,
    pub server_event_receiver: HashMap<NodeId, Receiver<DroneEvent>>,
    pub packet_sender: HashMap<NodeId, Sender<Packet>>,         //packet sender of each node
    //pub packet_receiver: HashMap<NodeId, Receiver<Packet>>,     //packet receiver of each node
}

impl SimulationController {
    pub fn new(
        config: Config,
        drone_command_sender: HashMap<NodeId, Sender<DroneCommand>>,
        drone_event_receiver: HashMap<NodeId, Receiver<DroneEvent>>,
        client_command_sender: HashMap<NodeId, Sender<ClientCommand>>,
        client_event_receiver: HashMap<NodeId, Receiver<DroneEvent>>,
        server_event_receiver: HashMap<NodeId, Receiver<DroneEvent>>,
        packet_sender: HashMap<NodeId, Sender<Packet>>,
        //packet_receiver: HashMap<NodeId, Receiver<Packet>>,
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
            client_command_sender,
            client_event_receiver,
            server_event_receiver,
            packet_sender,
            //packet_receiver,
        }
    }

/*pub fn update_channels(
        &mut self,
        drone_command_sender: HashMap<NodeId, Sender<DroneCommand>>,
        drone_event_receiver: HashMap<NodeId, Receiver<DroneEvent>>,
        client_command_sender: HashMap<NodeId, Sender<ClientCommand>>,
        client_event_receiver: HashMap<NodeId, Receiver<DroneEvent>>,
        server_event_receiver: HashMap<NodeId, Receiver<DroneEvent>>,
    ) {
        log("Updating channels");
        self.drone_command_sender = drone_command_sender;
        self.drone_event_receiver = drone_event_receiver;
        self.client_command_sender = client_command_sender;
        self.client_event_receiver = client_event_receiver;
        self.server_event_receiver = server_event_receiver;
        log("Channels updated");
    }*/
}

/*pub fn main() {
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

    //prepare shutdown channels
    let (shutdown_sender, shutdown_receiver) = unbounded::<()>();
    //prepare the handles vector that handles all the threads
    let mut handles = Vec::new();

    // - command_sender contains the channels senders to send commands to drones
    let mut drone_command_sender = HashMap::new();
    // - event_receiver contains the channels receiver to receive events from drones
    let mut drone_event_receiver = HashMap::new();

    //let (node_event_send, _node_event_recv) = unbounded();

    for drone in config.clone().drone.into_iter() {
        // drone command channel
        let (controller_drone_send, controller_drone_recv) = unbounded();
        // drone event channel
        let (drone_event_sender, drone_event_recv) = unbounded();

        let shutdown_recv = shutdown_receiver.clone();

        //creating the HashMap needed to send commands to drones
        drone_command_sender.insert(drone.id, controller_drone_send);
        //creating the HashMap needed to receive events from drones
        drone_event_receiver.insert(drone.id, drone_event_recv);
        // packet
        let drone_packet_recv = packet_channels[&drone.id].1.clone();
        let drone_packet_send = drone
            .connected_node_ids
            .into_iter()
            .map(|id| (id, packet_channels[&id].0.clone()))
            .collect();

        handles.push(thread::spawn(move || {
            let mut drone = LeDron::new(
                drone.id,
                drone_event_sender,
                controller_drone_recv,
                drone_packet_recv,
                drone_packet_send,
                drone.pdr,
            );

            loop {
                crossbeam_channel::select! {
                    recv(shutdown_recv) -> _ => {
                        println!("Drone {} received shutdown signal", drone.id);
                        break;
                    }
                    default => drone.run(),
                }
            }
        }));
    }

    // - command_sender contains the channels senders to send commands to clients
    let mut client_command_sender = HashMap::new();
    // - event_receiver contains the channels receiver to receive events from clients
    let mut client_event_receiver = HashMap::new();

    for client in config.clone().client.iter() {  // ✅ Use `iter()` instead of `into_iter()`
        let client_id = client.id;  // ✅ Copy primitive value
        let connected_drone_ids = client.connected_drone_ids.clone(); // ✅ Clone so it can be moved later

        // Client command channel
        let (controller_client_send, controller_client_recv) = unbounded();
        let (client_event_sender, client_event_recv) = unbounded();

        let shutdown_recv = shutdown_receiver.clone();

        // Insert into HashMaps
        client_command_sender.insert(client_id, controller_client_send);
        client_event_receiver.insert(client_id, client_event_recv);

        // Packet channels
        let packet_recv = packet_channels[&client_id].1.clone(); // ✅ Clone before move
        let packet_send: HashMap<_, _> = connected_drone_ids
            .iter()
            .map(|&id| (id, packet_channels[&id].0.clone())) // ✅ Clone values, avoid move
            .collect();

        // Spawn the thread with only the necessary cloned values
        handles.push(thread::spawn(move || {
            let mut client = LeClient::new(
                client_id,
                NodeType::Client,
                connected_drone_ids, // ✅ Uses cloned values
                client_event_sender,
                controller_client_recv,
                packet_recv,
                packet_send,
            );

            loop {
                crossbeam_channel::select! {
                    recv(shutdown_recv) -> _ => {
                        println!("Client {} received shutdown signal", client_id);
                        break;
                    }
                    default => client.run(),
                }
            }
        }));
    }
    // - event_receiver contains the channels receiver to receive events from clients
    let mut server_event_receiver = HashMap::new();
    let mut curr_server = 0;
    for server in config.clone().server.iter() {  // ✅ Use `iter()` instead of `into_iter()`

        let server_id = server.id;  // ✅ Copy primitive value
        let connected_drone_ids = server.connected_drone_ids.clone(); // ✅ Clone so it can be moved later
        let server_clone = server.clone(); // ✅ Clone full `client` if needed

        // Client event channel
        let (server_event_sender, server_event_recv) = unbounded();

        let shutdown_recv = shutdown_receiver.clone();

        // Insert into HashMaps
        server_event_receiver.insert(server_id, server_event_recv);

        // Packet channels
        let packet_recv = packet_channels[&server_id].1.clone(); // ✅ Clone before move
        let packet_send: HashMap<_, _> = connected_drone_ids
            .iter()
            .map(|&id| (id, packet_channels[&id].0.clone())) // ✅ Clone values, avoid move
            .collect();

        // Spawn the thread with only the necessary cloned values
        if curr_server == 0 {
            handles.push(thread::spawn(move || {
                let mut server = LeChatServer::new(
                    server_id,
                    NodeType::Server,
                    connected_drone_ids,
                    server_event_sender,
                    packet_recv,
                    packet_send,
                );
                loop {
                    crossbeam_channel::select! {
                        recv(shutdown_recv) -> _ => {
                            println!("Server {} received shutdown signal", server_id);
                            break;
                        }
                        default => server.run(),
                    }
                }
            }));
            curr_server = curr_server + 1;
        } else if curr_server == 1 {
            handles.push(thread::spawn(move || {
                let mut server = LeContentServer::new(
                    server_id,
                    NodeType::Server,
                    connected_drone_ids, // ✅ Uses cloned values
                    server_event_sender,
                    packet_recv,
                    packet_send,
                );
                loop {
                    crossbeam_channel::select! {
                        recv(shutdown_recv) -> _ => {
                            println!("Server {} received shutdown signal", server_id);
                            break;
                        }
                        default => server.run(),
                    }
                }
            }));
            curr_server = curr_server + 1;
        } else {
            println!("Error running servers")
        }

    }

    let mut packet_senders = HashMap::new();
    let mut packet_receivers = HashMap::new();
    for (node_id, (sender, receiver)) in packet_channels.iter() {
        packet_senders.insert(*node_id, sender.clone());
        packet_receivers.insert(*node_id, receiver.clone());
    }
    let controller = SimulationController::new(
        config,
        drone_command_sender,
        drone_event_receiver,
        client_command_sender,
        client_event_receiver,
        server_event_receiver,
        packet_senders,
        packet_receivers
    );

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some( Window {
                title: "Simulation Controller".into(),
                resolution: (1280.0, 720.0).into(),
                visible: true,
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        .insert_state(OptionMenu::default())
        .insert_resource(controller)
        .insert_resource(SelectedDrone::default())
        .insert_resource(SelectedClient::default())
        .add_plugins((
            UiPlugin,
            DronePlugin,
            ClientPlugin,
            ServerPlugin,
        ))
        .insert_resource(ClearColor(Color::srgb(0.3, 0.3, 0.3)))
        .add_systems(Update, handle_exit_events)
        .insert_resource(ShutdownSender(shutdown_sender))
        .run();

    while let Some(handle) = handles.pop() {
        handle.join().unwrap();
    }
}*/
/*
///--------------------------------------------
// Resource per memorizzare gli handle dei thread
#[derive(Resource, Default)]
struct ThreadHandles(Vec<thread::JoinHandle<()>>);

// Resource per memorizzare lo stato di inizializzazione
#[derive(Resource)]
struct ThreadsInitialized(bool);

// Implementazione Default per ThreadsInitialized
impl Default for ThreadsInitialized {
    fn default() -> Self {
        Self(false)
    }
}*/

pub fn simulate(/*controller: SimulationController, mut handles: Vec<JoinHandle<()>>*/) {
    let config = parse_config("/Volumes/SSD FILIPPO/UNI/Advanced Programming/Drone Project/LeDron-James/src/simulation_controller/src/config");

    //let mut packet_channels = HashMap::new();
    let mut packet_senders = HashMap::new();
    let mut packet_receivers = HashMap::new();
    for drone in config.drone.iter() {
        let (s, r) = unbounded();
        packet_senders.insert(drone.id, s);
        packet_receivers.insert(drone.id, r);
    }
    for client in config.client.iter() {
        let (s, r) = unbounded();
        packet_senders.insert(client.id, s);
        packet_receivers.insert(client.id, r);
    }
    for server in config.server.iter() {
        let (s, r) = unbounded();
        packet_senders.insert(server.id, s);
        packet_receivers.insert(server.id, r);
    }

    let mut handles = Vec::new();
    let mut drone_command_sender = HashMap::new();
    let mut drone_event_receiver = HashMap::new();
    let mut client_command_sender = HashMap::new();
    let mut client_event_receiver = HashMap::new();
    let mut server_event_receiver = HashMap::new();

    /// - Drones initialization
    for drone in config.drone.iter() {
        let (controller_drone_send, controller_drone_recv) = unbounded();
        let (drone_event_sender, drone_event_recv) = unbounded();

        drone_command_sender.insert(drone.id, controller_drone_send);
        drone_event_receiver.insert(drone.id, drone_event_recv);

        //let drone_packet_recv = packet_channels[&drone.id].1.clone();
        let drone_packet_recv = packet_receivers.remove(&drone.id).unwrap();
        let drone_packet_send = drone.connected_node_ids
            .iter()
            .map(|id| (*id, packet_senders[id].clone()))
            .collect();

        let drone_clone = drone.clone();
        handles.push(thread::spawn(move || {
            let mut drone = LeDron::new(
                drone_clone.id,
                drone_event_sender,
                controller_drone_recv,
                drone_packet_recv,
                drone_packet_send,
                drone_clone.pdr,
            );
            drone.run();
        }));
    }
    let controller = SimulationController::new(
        config.clone(),
        drone_command_sender,
        drone_event_receiver,
        client_command_sender,
        client_event_receiver,
        server_event_receiver,
        packet_senders,
    );
    log("Starting simulation controller");
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some( Window {
                title: "Simulation Controller".into(),
                resolution: (1280.0, 720.0).into(),
                visible: true,
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        .insert_state(OptionMenu::default())
        .insert_resource(controller)
        .insert_resource(SelectedDrone::default())
        .insert_resource(SelectedClient::default())
        //.add_systems(Update, check_window_spawn)
        .add_plugins((
            UiPlugin,
            DronePlugin,
            ClientPlugin,
            ServerPlugin,
        ))
        //.add_systems(Update, quit)
        .insert_resource(ClearColor(BColor::srgb(0.3, 0.3, 0.3)))
        .run();
    log("Closing simulation controller");

    while let Some(handle) = handles.pop() {
        log("Closing node thread");
        handle.join().unwrap();
    }
    log("The End")
}
/*
// Resource per passare i dati di setup
#[derive(Resource)]
struct SetupData {
    config: Config,
    packet_channels: HashMap<NodeId, (Sender<Packet>, Receiver<Packet>)>,
}


// Sistema per inizializzare i thread dopo che l'app è partita
fn initialize_threads(
    mut commands: Commands,
    setup_data: Res<SetupData>,
    mut threads_initialized: ResMut<ThreadsInitialized>,
    mut thread_handles: ResMut<ThreadHandles>,
    mut next_state: ResMut<NextState<Ready>>
) {
    if threads_initialized.0 {
        return;
    }

    let mut handles = Vec::new();
    let mut drone_command_sender = HashMap::new();
    let mut drone_event_receiver = HashMap::new();
    let mut client_command_sender = HashMap::new();
    let mut client_event_receiver = HashMap::new();
    let mut server_event_receiver = HashMap::new();

    // Inizializzazione dei droni
    for drone in setup_data.config.drone.iter() {
        let (controller_drone_send, controller_drone_recv) = unbounded();
        let (drone_event_sender, drone_event_recv) = unbounded();

        drone_command_sender.insert(drone.id, controller_drone_send);
        drone_event_receiver.insert(drone.id, drone_event_recv);

        let drone_packet_recv = setup_data.packet_channels[&drone.id].1.clone();
        let drone_packet_send = drone.connected_node_ids
            .iter()
            .map(|id| (*id, setup_data.packet_channels[id].0.clone()))
            .collect();

        let drone_clone = drone.clone();
        handles.push(thread::spawn(move || {
            let mut drone = LeDron::new(
                drone_clone.id,
                drone_event_sender,
                controller_drone_recv,
                drone_packet_recv,
                drone_packet_send,
                drone_clone.pdr,
            );
            drone.run();
        }));
    }

    // Inizializzazione dei client (codice simile per client)
    for client in setup_data.config.client.iter() {  // ✅ Use `iter()` instead of `into_iter()`
        let client_id = client.id;  // ✅ Copy primitive value
        let connected_drone_ids = client.connected_drone_ids.clone(); // ✅ Clone so it can be moved later

        // Client command channel
        let (controller_client_send, controller_client_recv) = unbounded();
        let (client_event_sender, client_event_recv) = unbounded();

        // Insert into HashMaps
        client_command_sender.insert(client_id, controller_client_send);
        client_event_receiver.insert(client_id, client_event_recv);

        // Packet channels
        let packet_recv = setup_data.packet_channels[&client_id].1.clone(); // ✅ Clone before move
        let packet_send: HashMap<_, _> = connected_drone_ids
            .iter()
            .map(|&id| (id, setup_data.packet_channels[&id].0.clone())) // ✅ Clone values, avoid move
            .collect();

        // Spawn the thread with only the necessary cloned values
        handles.push(thread::spawn(move || {
            let mut client = LeClient::new(
                client_id,
                NodeType::Client,
                connected_drone_ids, // ✅ Uses cloned values
                client_event_sender,
                controller_client_recv,
                packet_recv,
                packet_send,
            );

            client.run()
        }));
    }

    let mut curr_server = 0;
    // Inizializzazione dei server (codice simile per server)
    for server in setup_data.config.server.iter() {  // ✅ Use `iter()` instead of `into_iter()`

        let server_id = server.id;  // ✅ Copy primitive value
        let connected_drone_ids = server.connected_drone_ids.clone(); // ✅ Clone so it can be moved later
        let server_clone = server.clone(); // ✅ Clone full `client` if needed

        // Client event channel
        let (server_event_sender, server_event_recv) = unbounded();


        // Insert into HashMaps
        server_event_receiver.insert(server_id, server_event_recv);

        // Packet channels
        let packet_recv = setup_data.packet_channels[&server_id].1.clone(); // ✅ Clone before move
        let packet_send: HashMap<_, _> = connected_drone_ids
            .iter()
            .map(|&id| (id, setup_data.packet_channels[&id].0.clone())) // ✅ Clone values, avoid move
            .collect();

        // Spawn the thread with only the necessary cloned values
        if curr_server == 0 {
            handles.push(thread::spawn(move || {
                let mut server = LeChatServer::new(
                    server_id,
                    NodeType::Server,
                    connected_drone_ids,
                    server_event_sender,
                    packet_recv,
                    packet_send,
                );
                server.run()
            }));
            curr_server = curr_server + 1;
        } else if curr_server == 1 {
            handles.push(thread::spawn(move || {
                let mut server = LeContentServer::new(
                    server_id,
                    NodeType::Server,
                    connected_drone_ids, // ✅ Uses cloned values
                    server_event_sender,
                    packet_recv,
                    packet_send,
                );
                server.run()
            }));
            curr_server = curr_server + 1;
        } else {
            println!("Error running servers")
        }

    }
    // Preparazione packet_channels per il controller
    let mut packet_senders = HashMap::new();
    let mut packet_receivers = HashMap::new();
    for (node_id, (sender, receiver)) in &setup_data.packet_channels {
        packet_senders.insert(*node_id, sender.clone());
        packet_receivers.insert(*node_id, receiver.clone());
    }

    // Creiamo il controller con canali vuoti
    let controller = SimulationController::new(
        setup_data.config.clone(),
        drone_command_sender,
        drone_event_receiver,
        client_command_sender,
        client_event_receiver,
        server_event_receiver,
        packet_senders,
        packet_receivers,
    );

    if !threads_initialized.0 {
        commands.insert_resource(controller);
        next_state.set(Ready::Go);
        log("simulation controller resource inserted");
    }
    thread_handles.0 = handles;

    threads_initialized.0 = true;
    log("All threads have been initialized!");
}*/

fn quit (
    mut app_exit_events: EventWriter<AppExit>,
    input: Res<ButtonInput<KeyCode>>
) {
    if input.just_pressed(KeyCode::Escape) {
        log("Escape has been pressed!");
        app_exit_events.send(AppExit::Success);
    }
}


#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum OptionMenu {
    #[default]
    Off,
    On
}

#[derive(Resource, Default, Debug)]
pub struct SelectedDrone {
    pub drone_id: Option<NodeId>
}

#[derive(Resource, Default, Debug)]
pub struct SelectedClient {
    pub client_id: Option<NodeId>
}

#[derive(Default, Reflect, GizmoConfigGroup)]
struct MyGizmos {}

pub fn log(txt: &str) {
    println!("{} - {}","SimulationController".on_green(), txt.green());
}

fn check_window_spawn(mut events: EventReader<WindowCreated>) {
    log("loop!!!");
    for _ in events.read() {
        println!("Finestra creata con successo!");
    }
}

pub fn parse_config(file: &str) -> Config {
    let file_str = fs::read_to_string(file).unwrap();
    toml::from_str(&file_str).unwrap()
}
