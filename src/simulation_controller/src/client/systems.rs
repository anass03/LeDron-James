use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use wg_2024::controller::DroneEvent;
use wg_2024::network::NodeId;
use wg_internal::packet::Packet;
use host::{ClientCommand, ClientId, FileId, MediaId, Message};
use crate::client::components::Client;
use crate::drone::components::ScDrone;
use crate::drone::systems::{get_drone_crashed_form_id, get_drone_transform_from_id};
use crate::simulation_controller::{log, SelectedClient, SimulationController};

pub fn spawn_clients(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    simulation_controller: Res<SimulationController>,
) {
    let mut i = 0;
    for client in simulation_controller.clients.iter() {
        spawn(&mut commands, &window_query, &asset_server, &client.id, i);
        i += 1;
    }
}

pub fn spawn(
    commands: &mut Commands,
    window_query: &Query<&Window, With<PrimaryWindow>>,
    asset_server: &Res<AssetServer>,
    id: &NodeId,
    i: u8
) {
    let window = window_query.get_single().unwrap();
        commands.spawn((
            SpriteBundle {
                transform: match i {
                    0 => Transform::from_xyz(window.width() / 6.0, window.height() / 3.0, 0.0),
                    1 => Transform::from_xyz(window.width() / 6.0, window.height() / 3.0 * 2.0, 0.0),
                    _ => Transform::from_xyz(0.0, 0.0, 0.0),
                },
                texture: asset_server.load("/Volumes/SSD FILIPPO/UNI/Advanced Programming/Drone Project/LeDron-James/src/simulation_controller/assets/sprites/client.png"),
                ..default()
            },
            Client { id: id.clone() },
        ));
}


pub fn join(
    mut gizmos: Gizmos,
    client_query: Query<(&Client, &Transform), With<Client>>,
    drone_query: Query<(&ScDrone, &Transform), With<ScDrone>>,
    simulation_controller: Res<SimulationController>,
) {
    log("joining clients");
    for (client1, transform) in client_query.iter() {
        let start = transform.translation.truncate(); // Convert to Vec2

        for conn_drone in simulation_controller.drones.iter() {
            if conn_drone.connected_node_ids.contains(&client1.id) && !get_drone_crashed_form_id(&drone_query, &conn_drone.id) {
                if let Some(trans2) = get_drone_transform_from_id(&drone_query, &conn_drone.id) {
                    let end = trans2.translation.truncate(); // Convert to Vec2

                    // Draw a red line between the client and drone
                    gizmos.line_2d(start, end, Color::srgb(1.0, 0.0, 0.0));
                }
            }
        }
    }
}

pub fn server_type(
    selected_client: &Res<SelectedClient>,
    simulation_controller: &Res<SimulationController>,
) {
    let current_client = match selected_client.client_id {
        Some(client_id) => client_id,
        None => {
            println!("No client selected!");
            return;
        }
    };
    simulation_controller.client_command_sender.get(&current_client).unwrap().send(ClientCommand::ServerType).expect("failed to send Server Type request");
}

pub fn file_list(
    selected_client: &Res<SelectedClient>,
    simulation_controller: &Res<SimulationController>,
) {
    let current_client = match selected_client.client_id {
        Some(client_id) => client_id,
        None => {
            println!("No client selected!");
            return;
        }
    };
    simulation_controller.client_command_sender.get(&current_client).unwrap().send(ClientCommand::FilesList).expect("failed to send File List request");
}

pub fn file(
    selected_client: &Res<SelectedClient>,
    simulation_controller: &Res<SimulationController>,
    file_id: usize,
) {
    let current_client = match selected_client.client_id {
        Some(client_id) => client_id,
        None => {
            println!("No client selected!");
            return;
        }
    };
    simulation_controller.client_command_sender.get(&current_client).unwrap().send(ClientCommand::File(file_id)).expect("failed to send File request");
}

pub fn media_list(
    selected_client: &Res<SelectedClient>,
    simulation_controller: &Res<SimulationController>,
) {
    let current_client = match selected_client.client_id {
        Some(client_id) => client_id,
        None => {
            println!("No client selected!");
            return;
        }
    };
    simulation_controller.client_command_sender.get(&current_client).unwrap().send(ClientCommand::MediaList).expect("failed to send Media List request");
}

pub fn media(
    selected_client: &Res<SelectedClient>,
    simulation_controller: &Res<SimulationController>,
    media: MediaId,
) {
    let current_client = match selected_client.client_id {
        Some(client_id) => client_id,
        None => {
            println!("No client selected!");
            return;
        }
    };
    simulation_controller.client_command_sender.get(&current_client).unwrap().send(ClientCommand::Media(media)).expect("failed to send Media request");
}

pub fn registration_to_chat(
    selected_client: &Res<SelectedClient>,
    simulation_controller: &Res<SimulationController>,
) {
    let current_client = match selected_client.client_id {
        Some(client_id) => client_id,
        None => {
            println!("No client selected!");
            return;
        }
    };
    simulation_controller.client_command_sender.get(&current_client).unwrap().send(ClientCommand::RegistrationToChat).expect("failed to send Registration to Chat request");
}

pub fn client_list(
    selected_client: &Res<SelectedClient>,
    simulation_controller: &Res<SimulationController>,
) {
    let current_client = match selected_client.client_id {
        Some(client_id) => client_id,
        None => {
            println!("No client selected!");
            return;
        }
    };
    simulation_controller.client_command_sender.get(&current_client).unwrap().send(ClientCommand::ClientList).expect("failed to send Client List request");
}

pub fn message_for(
    selected_client: &Res<SelectedClient>,
    simulation_controller: &Res<SimulationController>,
    recipient_client: usize,
    message: String
) {
    let current_client = match selected_client.client_id {
        Some(client_id) => client_id,
        None => {
            println!("No client selected!");
            return;
        }
    };
    simulation_controller.client_command_sender.get(&current_client).unwrap().send(ClientCommand::MessageFor(recipient_client as ClientId, message)).expect("failed to send Message request");
}

pub fn controller_shortcut(
    selected_client: &Res<SelectedClient>,
    simulation_controller: &Res<SimulationController>,
    packet: Packet
) {
    let current_client = match selected_client.client_id {
        Some(client_id) => client_id,
        None => {
            println!("No client selected!");
            return;
        }
    };
    simulation_controller.client_command_sender.get(&current_client).unwrap().send(ClientCommand::ControllerShortcut(packet)).expect("failed to send Controller Shortcut Packet");
}

pub fn handle_client_packet_sent (
    simulation_controller: Res<SimulationController>,
    client_query: Query<(&Client, &Transform), With<Client>>,
) {
    for reciever in simulation_controller.client_event_receiver.iter() {
        for recieved in reciever.1.iter() {
            match recieved {
                DroneEvent::PacketSent(packet) => {
                    println!("Packet sent: {:?}", packet);
                },
                DroneEvent::PacketDropped(packet) => {
                    println!("Packet dropped: {:?}", packet);
                },
                DroneEvent::ControllerShortcut(packet) => {
                    println!("Controller shortcut: {:?}", packet);
                }
            }
        }
    }
}