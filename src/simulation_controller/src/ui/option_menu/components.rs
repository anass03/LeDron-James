use bevy::prelude::*;
use wg_2024::network::NodeId;

#[derive(Component)]
pub struct OptionMenu{}

#[derive(Component)]
pub struct CrashButton{}

#[derive(Component)]
pub struct DroneIdText {}

#[derive(Component)]
pub struct ClientIdText {}

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
pub struct PdrInputText {
    pub(crate) value: String
}

#[derive(Resource)]
pub struct IsPdrMenuOpen(pub bool);

//Client Commands

#[derive(Component)]
pub struct ServerTypeButton;

#[derive(Component)]
pub struct FileListButton;

#[derive(Component)]
pub struct FileButton;

#[derive(Component)]
pub struct MediaListButton;

#[derive(Component)]
pub struct MediaButton;

#[derive(Component)]
pub struct RegistrationToChatButton;

#[derive(Component)]
pub struct ClientListButton;

#[derive(Component)]
pub struct MessageForButton;

#[derive(Component)]
pub struct ControllerShortcutButton;

#[derive(Component)]
pub struct ChooseFileMenu;

#[derive(Component)]
pub struct FileNameInputText {
    pub(crate) value: String
}

#[derive(Component)]
pub struct ConfirmFileNameButton;

#[derive(Component)]
pub struct ChooseMediaMenu;

#[derive(Component)]
pub struct MediaNameInputText {
    pub(crate) value: String
}

#[derive(Component)]
pub struct ConfirmMediaNameButton;
