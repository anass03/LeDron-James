use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::ui::styles::*;

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

pub fn spawn_demo(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(
        NodeBundle {
            style: DEMO_TEXT_STYLE,
            ..default()
        }
    ).with_children(|parent| {
        parent.spawn(
            TextBundle {
                text: Text {
                    sections: vec![TextSection::new(
                        "SIMULATION CONTROLLER DEMO",
                        get_demo_text_style(&asset_server)
                    )],
                    ..default()
                },
                ..default()
            }
        );
    })
    ;
}