use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::client::components::Client;

pub fn spawn_clients(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    window_query: Query<&Window, With<PrimaryWindow>>
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
            Client {}
        ));
    }
}