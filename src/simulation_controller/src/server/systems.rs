use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use wg_2024::network::NodeId;
use crate::drone::components::ScDrone;
use crate::drone::systems::{get_drone_crashed_form_id, get_drone_transform_from_id};
use crate::server::components::Server;
use crate::simulation_controller::SimulationController;

pub fn spawn_server(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    simulation_controller: Res<SimulationController>,
) {
    for server in simulation_controller.servers.iter() {
        spawn(&mut commands, &asset_server, &window_query, &server.id)
    }
}

pub fn spawn(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    window_query: &Query<&Window, With<PrimaryWindow>>,
    id: &NodeId
) {
    let window = window_query.get_single().unwrap();
    commands.spawn((
        SpriteBundle {
            transform: Transform::from_xyz(window.width() / 6.0 * 5.0, window.height() / 2.0, 0.0),
            texture: asset_server.load("/Volumes/SSD FILIPPO/UNI/Advanced Programming/Drone Project/LeDron-James/src/simulation_controller/assets/sprites/server.png"),
            ..default()
        },
        Server { id: id.clone()}
    ));
}

pub fn join(
    mut gizmos: Gizmos,
    drone_query: Query<(&ScDrone, &Transform), With<ScDrone>>,
    server_query: Query<(&Server, &Transform), With<Server>>,
    simulation_controller: Res<SimulationController>,
) {
    for (server1, transform) in server_query.iter() {
        let start = transform.translation.truncate(); // Convert to Vec2

        for conn_drone in simulation_controller.drones.iter() {
            if conn_drone.connected_node_ids.contains(&server1.id) && !get_drone_crashed_form_id(&drone_query, &conn_drone.id) {
                if let Some(trans2) = get_drone_transform_from_id(&drone_query, &conn_drone.id) {
                    let end = trans2.translation.truncate(); // Convert to Vec2

                    // Draw a red line between the server and the drone using gizmos
                    gizmos.line_2d(start, end, Color::srgb(1.0, 0.0, 0.0));
                }
            }
        }
    }
}