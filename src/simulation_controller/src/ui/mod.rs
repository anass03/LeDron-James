use bevy::app::*;
use bevy::window::close_when_requested;
use crate::ui::systems::layouts::*;

pub mod styles;
pub mod systems;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, spawn_camera)
        ;
    }
}