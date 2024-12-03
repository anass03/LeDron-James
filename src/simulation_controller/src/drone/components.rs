use std::collections::HashMap;
use bevy::prelude::*;
use crossbeam_channel::Sender;
use wg_2024::network::NodeId;

#[derive(Component)]
pub struct ScDrone {
    pub id: NodeId,
}