use bevy::app::*;
use crate::ui::systems::layouts::*;

pub mod styles;
pub mod systems;
pub mod components;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, (spawn_camera, spawn_button))
            .add_systems(Update, click_button)
        ;
    }
}