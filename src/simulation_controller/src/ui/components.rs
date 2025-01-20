use bevy::prelude::*;
use wg_2024::network::NodeId;
#[derive(Component)]
pub struct NodeConnection (pub NodeId, pub NodeId);
#[derive(Component)]
pub struct CrashButton {}