/*use bevy::color::Color as BColor;
use crate::drone::DronePlugin;
//use crate::client::ClientPlugin;
//use crate::server::ServerPlugin;
use crate::ui::UiPlugin;
use std::{fs, thread};
use bevy::window::WindowCreated;
use drone::Drone as LeDron;
use client::Client as LeClient;
use host::ClientCommand;
use server::ContentServer as LeContentServer;
use server::chatserver::ChatServer as LeChatServer;
use bevy::prelude::*;
use crate::ui::option_menu::components::SenderStatus;*/
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::collections::HashMap;
use std::thread::JoinHandle;
use wg_2024::config::Config;
use wg_2024::config::Drone as DroneConfig;
use wg_2024::config::Client as ClientConfig;
use wg_2024::config::Server as ServerConfig;
use wg_2024::controller::*;
use wg_2024::network::NodeId;
use wg_2024::drone::Drone;
use wg_2024::packet::{NodeType, Packet};
use colored::*;

use std::io;
use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
use std::time::{Duration, Instant};

/// Simulation Controller struct contains:
/// - the drones
/// - the clients
/// - the servers
/// - the command sender channels
/// - the event receiver channels
//#[derive(Resource)]
pub struct SimulationController {
    pub drones: Vec<DroneConfig>,   //vector of drones
    pub clients: Vec<ClientConfig>, //vector of clients
    pub servers: Vec<ServerConfig>, //vector of servers
    pub drone_command_sender: HashMap<NodeId, Sender<DroneCommand>>,  //command sender channel to drone
    pub drone_event_receiver: HashMap<NodeId, Receiver<DroneEvent>>,   //event receiver channel from drone
    /*pub client_command_sender: HashMap<NodeId, Sender<ClientCommand>>,
    pub client_event_receiver: HashMap<NodeId, Receiver<DroneEvent>>,
    pub server_event_receiver: HashMap<NodeId, Receiver<DroneEvent>>,*/
    pub packet_sender: HashMap<NodeId, Sender<Packet>>,         //packet sender of each node
    //pub packet_receiver: HashMap<NodeId, Receiver<Packet>>,     //packet receiver of each node
}

impl SimulationController {
    pub fn new(
        config: Config,
        drone_command_sender: HashMap<NodeId, Sender<DroneCommand>>,
        drone_event_receiver: HashMap<NodeId, Receiver<DroneEvent>>,
        /*client_command_sender: HashMap<NodeId, Sender<ClientCommand>>,
        client_event_receiver: HashMap<NodeId, Receiver<DroneEvent>>,
        server_event_receiver: HashMap<NodeId, Receiver<DroneEvent>>,*/
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
            /*client_command_sender,
            client_event_receiver,
            server_event_receiver,*/
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
pub struct App {
    pub controller: SimulationController,
    pub selected_node: Option<NodeId>,
    pub logs: Vec<String>
}

pub fn simulate(controller: SimulationController, mut handles: Vec<JoinHandle<()>>) {

    let app = App {
        controller,
        selected_node: None,
        logs: vec![],
    };


    log("Starting simulation controller");
    /*App::new()
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
            //ClientPlugin,
            //ServerPlugin,
        ))
        //.add_systems(Update, quit)
        .insert_resource(ClearColor(BColor::srgb(0.3, 0.3, 0.3)))
        .run();*/
    log("Closing simulation controller");

    while let Some(handle) = handles.pop() {
        log("Closing node thread");
        handle.join().unwrap();
    }
    log("The End")
}

pub fn log(txt: &str) {
    //println!("{} - {}",Colorize::on_green("SimulationController"), colored::Colorize::on_green(txt))
}

/*
fn quit (
    mut app_exit_events: EventWriter<AppExit>,
    input: Res<ButtonInput<KeyCode>>
) {
    if input.just_pressed(KeyCode::Esc) {
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

fn check_window_spawn(mut events: EventReader<WindowCreated>) {
    log("loop!!!");
    for _ in events.read() {
        println!("Finestra creata con successo!");
    }
}
*/
