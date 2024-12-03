pub mod systems;
mod components;
pub mod style;

use bevy::prelude::*;
use bevy_prototype_lyon::plugin::ShapePlugin;
use crate::drone::systems::*;
use crate::simulation_controller::SimulationController;

pub struct DronePlugin;

impl Plugin for DronePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, spawn_drone)
            .add_systems(Update, join)
        ;
    }
}