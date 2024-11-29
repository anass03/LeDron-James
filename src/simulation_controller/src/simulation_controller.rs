use bevy::prelude::*;
use crate::drone::systems::*;
use crate::ui::systems::layouts::*;

pub fn build_simulation_controller() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::srgb(0.3, 0.3, 0.3)))
        .add_systems(Startup, (spawn_camera, spawn))
        .add_systems(Update, crash)
        .run();
}
