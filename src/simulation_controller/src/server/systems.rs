use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::server::components::Server;

pub fn spawn_server(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    window_query: Query<&Window, With<PrimaryWindow>>
) {
    let window = window_query.get_single().unwrap();
    commands.spawn((
        SpriteBundle {
            transform: Transform::from_xyz(window.width() / 6.0 * 5.0, window.height() / 2.0, 0.0),
            texture: asset_server.load("/Users/bibo/UNI/Advanced Programming/Drone Project/LeDron-James/src/simulation_controller/assets/sprites/server.png"),
            ..default()
        },
        Server {}
        ));
}