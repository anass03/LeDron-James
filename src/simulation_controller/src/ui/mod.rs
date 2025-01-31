use bevy::app::*;
use crate::ui::option_menu::OptionMenuPlugin;
use crate::ui::systems::layouts::*;

pub mod styles;
pub mod systems;
pub mod components;
pub mod option_menu;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(OptionMenuPlugin)
            .add_systems(Startup, spawn_camera)
            .add_systems(Update, (
                drone_clicked,
                client_clicked,
            ))

        ;
    }
}