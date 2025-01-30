use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use wg_2024::network::NodeId;
use crate::client::components::Client;
use crate::drone::components::ScDrone;
use crate::drone::systems::{get_drone_crashed_form_id, get_drone_transform_from_id};
use crate::simulation_controller::SimulationController;

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