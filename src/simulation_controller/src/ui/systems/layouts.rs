use bevy::prelude::*;
use bevy::window::{PrimaryWindow};
use crate::drone::components::ScDrone;
use crate::simulation_controller::{OptionMenu, SelectedNode};

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

pub fn drone_clicked(
    drone_query: Query<(&ScDrone, &Transform), With<ScDrone>>,
    mouse: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut next_app_state: ResMut<NextState<OptionMenu>>,
    mut selected_node: ResMut<SelectedNode>
) {
    let window = window_query.get_single().unwrap();
    let mut clicked = false;
    if mouse.pressed(MouseButton::Left) {
        if let Some(mouse_pos) = window.cursor_position() {
            for (drone, transform) in drone_query.iter() {
                if ((mouse_pos.x - transform.translation.x) as f64).powi(2) + ((mouse_pos.y - (window.height() - transform.translation.y)) as f64).powi(2) <= (32 as f64).powi(2) {
                    clicked = true;
                    selected_node.node_id = Some(drone.id.clone());
                    break;
                } else {
                    clicked = false;
                }
            }
            if clicked {
                next_app_state.set(OptionMenu::On);
            } else {
                next_app_state.set(OptionMenu::Off);
            }
        }
    }
}