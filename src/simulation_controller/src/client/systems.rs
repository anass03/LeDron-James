use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use wg_2024::network::NodeId;
use crate::client::components::Client;
use crate::drone::components::ScDrone;
use crate::drone::systems::get_drone_from_id;
use crate::simulation_controller::SimulationController;

pub fn spawn_clients(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    simulation_controller: Res<SimulationController>,
) {
    for client in simulation_controller.clients.iter() {
        spawn(&mut commands, &window_query, &asset_server, &client.id);
    }
}

pub fn spawn(
    commands: &mut Commands,
    window_query: &Query<&Window, With<PrimaryWindow>>,
    asset_server: &Res<AssetServer>,
    id: &NodeId
) {
    let window = window_query.get_single().unwrap();
    for i in 0..2 {
        commands.spawn((
            SpriteBundle {
                transform: match i {
                    0 => Transform::from_xyz(window.width() / 6.0, window.height() / 3.0, 0.0),
                    1 => Transform::from_xyz(window.width() / 6.0, window.height() / 3.0 * 2.0, 0.0),
                    _ => Transform::from_xyz(0.0, 0.0, 0.0),
                },
                texture: asset_server.load("/Users/bibo/UNI/Advanced Programming/Drone Project/LeDron-James/src/simulation_controller/assets/sprites/client.png"),
                ..default()
            },
            Client { id: id.clone() },
        ));
    }
}

pub fn join(
    mut commands: Commands,
    drone_query: Query<(&ScDrone, &Transform), With<ScDrone>>,
    client_query: Query<(&Client, &Transform), With<Client>>,
    simulation_controller: Res<SimulationController>,
) {
    for (client1, transform) in client_query.iter() {
        let start = transform.translation;
        for conn_drone in simulation_controller.drones.iter() {
            if conn_drone.connected_node_ids.contains(&client1.id) {      //connects two drones if needed
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