pub mod systems;
pub mod components;
pub mod style;

use bevy::prelude::*;
use crate::drone::systems::*;

pub struct DronePlugin;

impl Plugin for DronePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, spawn_drone)
            .add_systems(Update, (
                join,
                handle_drone_packet_sent,
                neighbours
            ))
        ;
    }
}