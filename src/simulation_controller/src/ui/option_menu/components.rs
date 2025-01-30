use bevy::prelude::*;
use wg_2024::network::NodeId;

#[derive(Component)]
pub struct OptionMenu{}

#[derive(Component)]
pub struct CrashButton{}

#[derive(Component)]
pub struct NodeIdText{}

#[derive(Component)]
pub struct SetPdrButton{}

#[derive(Component)]
pub struct RemoveSenderButton{}

#[derive(Component)]
pub struct AddSenderButton{}

#[derive(Component)]
pub struct Checkbox {
    pub node_id: NodeId,
    pub checked: bool,
}

#[derive(Resource, Default)]
pub struct SelectedSenders {
    pub node_ids: Vec<NodeId>,
}
#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum SenderStatus {
    #[default]
    Adding,
    Removing
}

#[derive(Component)]
pub struct NodeIdSelector;

#[derive(Component)]
pub struct ConfirmNodeIdButton {}

/// PDR -----------------

#[derive(Component)]
pub struct PdrMenu;

#[derive(Component)]
pub struct ConfirmPdrButton;

#[derive(Resource, Default)]
pub struct SelectedPdr(pub(crate) f32);

#[derive(Component)]
pub struct InputText {
    pub(crate) value: String
}

#[derive(Resource)]
pub struct IsPdrMenuOpen(pub bool);


