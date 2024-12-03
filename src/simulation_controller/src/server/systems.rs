use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use wg_2024::network::NodeId;
use crate::drone::components::ScDrone;
use crate::drone::systems::get_drone_from_id;
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
            texture: asset_server.load("/Users/bibo/UNI/Advanced Programming/Drone Project/LeDron-James/src/simulation_controller/assets/sprites/server.png"),
            ..default()
        },
        Server { id: id.clone()}
    ));
}

pub fn join(
    mut commands: Commands,
    drone_query: Query<(&ScDrone, &Transform), With<ScDrone>>,
    server_query: Query<(&Server, &Transform), With<Server>>,
    simulation_controller: Res<SimulationController>,
) {
    for (server1, transform) in server_query.iter() {
        let start = transform.translation;
        for conn_drone in simulation_controller.drones.iter() {
            if conn_drone.connected_node_ids.contains(&server1.id) {      //connects two drones if needed
                let end = get_drone_from_id(&drone_query, &conn_drone.id).unwrap().translation;
                let mid_point = (start + end) / 2.0;
                let length = start.distance(end);
                let angle = (end.y - start.y).atan2(end.x - start.x);

                commands.spawn( SpriteBundle {
                    sprite: Sprite {
                        color: Color::srgb(1.0,0.0,0.0),
                        custom_size: Some(Vec2::new(length, 2.0)),
                        ..default()
                    },
                    transform: Transform {
                        translation: mid_point,
                        rotation: Quat::from_rotation_z(angle),
                        ..default()
                    },
                    ..default()
                });
            }
        }
    }
}