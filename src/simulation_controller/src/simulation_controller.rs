use bevy::prelude::*;
use crate::systems::{spawn_camera, spawn_demo};

pub fn build_simulation_controller() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::srgb(0.3, 0.3, 0.3)))
        .add_systems(Startup, spawn_camera)
        .add_systems(Update, spawn_demo)
        .run();
}
