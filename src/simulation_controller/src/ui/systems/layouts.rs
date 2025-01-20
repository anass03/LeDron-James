use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowCloseRequested};
use crate::ui::styles::*;
use bevy::app::AppExit;
use crate::drone::components::ScDrone;
//use crate::drone::systems::crash;
use crate::ui::components::{CrashButton, NodeConnection};

pub fn spawn_camera(
    mut commands: Commands,
    window_query: Query<&Window, With<PrimaryWindow>>,
) {
    let window = window_query.get_single().unwrap();
    commands.spawn(
        Camera2dBundle {
            transform: Transform::from_xyz(window.width() / 2.0, window.height() / 2.0, 0.0),
            ..default()
        }
    );
}
/*
pub fn spawn_button(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(NodeBundle {
        style: Style {
            width: Val::Px(100.0),
            height: Val::Px(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        ..default()
    }).with_children(|parent| {
        parent.spawn((ButtonBundle {
            style: Style {
                width: Val::Px(200.0),
                height: Val::Px(50.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            background_color: Color::BLACK.into(),
            ..default()
        },
                      CrashButton {}
        ))
            .with_children(|button| {
                button.spawn(TextBundle {
                    text: Text::from_section(
                        "Crash Drones",
                        TextStyle {
                            font: asset_server.load("/Users/bibo/UNI/Advanced Programming/Drone Project/LeDron-James/src/simulation_controller/assets/fonts/FiraSans-Bold.ttf"),
                            font_size: 30.0,
                            color: Color::WHITE,
                        },
                    ),
                    ..default()
                });
            });
    });
}

pub fn click_button(
    mut commands: Commands,
    mut button_query: Query<&Interaction, (Changed<Interaction>, With<CrashButton>)>,
    drone_query: Query<&mut ScDrone>,
    mut connection_query: Query<Entity, With<NodeConnection>>,
) {
    if let Ok(interaction) = button_query.get_single() {
        if *interaction == Interaction::Pressed{
            println!("Clicked button: {:?}", interaction);
            crash(commands, drone_query, connection_query)
        }
    }
}*/
