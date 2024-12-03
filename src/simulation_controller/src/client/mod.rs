pub mod components;
pub mod systems;

use bevy::prelude::*;
use crate::client::systems::spawn_clients;

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, spawn_clients)
        ;
    }
}