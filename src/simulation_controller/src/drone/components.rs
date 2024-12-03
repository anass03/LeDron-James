use bevy::prelude::*;
use wg_2024::network::NodeId;

#[derive(Component)]
pub struct ScDrone {
    pub id: NodeId,
}