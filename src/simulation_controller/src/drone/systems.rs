use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::drone::components::*;

pub fn spawn(
    mut commands: Commands,
    window_query: Query<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
) {
    let window = window_query.get_single().unwrap();
    commands.spawn((
        SpriteBundle {
            transform: Transform::from_xyz(window.width() / 2.0, window.height() / 2.0, 0.0),
            texture: asset_server.load("/Users/bibo/UNI/Advanced Programming/Drone Project/LeDron-James/src/simulation_controller/assets/sprites/drone.png"),
            ..default()
        },
        Drone {},
        Interaction::None
    ));
}

pub fn crash(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut drone_query: Query<(&Transform, &Handle<Image>), With<Drone>>,
    cursor_moved_events: Res<Events<CursorMoved>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
) {
    let window = window_query.get_single().unwrap();

    if mouse_button_input.just_pressed(MouseButton::Left) {
        if let Some(cursor_position) = window.cursor_position() {
            for (transform, texture_handle) in drone_query.iter_mut() {
                // Load texture size (you can cache this for performance)
                let texture_size = Vec2::new(64.0, 64.0); // Replace with actual texture size

                let sprite_position = transform.translation.truncate();
                let min = sprite_position - texture_size / 2.0;
                let max = sprite_position + texture_size / 2.0;

                // Check if the cursor is within the sprite's bounds
                if cursor_position.x >= min.x
                    && cursor_position.x <= max.x
                    && cursor_position.y >= min.y
                    && cursor_position.y <= max.y
                {
                    // Spawn a crash image
                    commands.spawn((
                        SpriteBundle {
                            texture: asset_server.load("/Users/bibo/UNI/Advanced Programming/Drone Project/LeDron-James/src/simulation_controller/assets/sprites/crashed.png"),
                            transform: Transform::from_xyz(window.width() / 2.0, window.height() / 2.0, 0.0),
                            ..default()
                        },
                    ));
                    println!("Crash detected!");
                }
            }
        }
    }
}