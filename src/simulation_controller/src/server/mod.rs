mod components;
mod systems;

use bevy::prelude::*;
use crate::server::systems::*;
pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, spawn_server)
            .add_systems(Update, join)
        ;
    }
}