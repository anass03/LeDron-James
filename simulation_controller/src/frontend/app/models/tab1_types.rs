use crate::frontend::app::models::general_types::{FnList, NetNodeList};
use host::{ClientId, FileId, MediaId, ServerId};
use ratatui::layout::Rect;
use wg_2024::network::NodeId;

/// Tab n.01 data
pub(in crate::frontend::app) struct Tab1 {
    pub net_node_list: NetNodeList,

    pub media_list_render_utils: MediaListRenderUtils,
    pub file_list_render_utils: FileListRenderUtils,

    // Popup cursor and selection works accordingly to each other.
    // Popup selection helps to understand which item cursor refers to
    pub popup_cursor: usize,
    pub popup_selection: PopupSelection,

    pub image_rendering_data: RenderingImage,
    pub file_exploring_data: FileExploring,

    pub viewing: ViewedTab1,

    pub selected_fn: FnList, // Deprecated

    pub navigation_history: Vec<ViewedTab1>, // Track navigation history for backspace
}

#[derive(Clone, Debug, Default)]
pub(crate) enum RenderingImage {
    #[default]
    False,
    True((Rect, ClientId, ServerId, MediaId)),
}
pub(crate) struct MediaListRenderUtils {
    pub list_len: usize,
    pub selected: u8,
}
impl MediaListRenderUtils {
    fn default() -> Self {
        Self {
            list_len: 0,
            selected: 0,
        }
    }
    pub(crate) fn increase(&mut self) {
        self.selected += 1;
    }
    pub(crate) fn decrease(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }
}
pub(crate) struct FileListRenderUtils {
    pub list_len: usize,
    pub selected: u8,
}
impl FileListRenderUtils {
    fn default() -> Self {
        Self {
            list_len: 0,
            selected: 0,
        }
    }
    pub(crate) fn increase(&mut self) {
        self.selected += 1;
    }
    pub(crate) fn decrease(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }
}
#[derive(Clone, Debug)]
pub(crate) struct FileExploring {
    pub file_request_source: u8, // 0 for none, 1 for Chat Page, 2 for NetPage Media, 3 for NetPage File
    // All the fields that come here doesn't need a None, as they are going to get overwrote by the new selected client, etc.
    pub last_selected_cl_sender: ClientId,
    pub last_selected_cl_receiver: ClientId,
    pub last_selected_sr: ServerId,
}
impl FileExploring {
    fn default() -> Self {
        Self {
            file_request_source: 0,
            last_selected_cl_sender: 0,
            last_selected_cl_receiver: 0,
            last_selected_sr: 0,
        }
    }
    pub(crate) fn set_fields(
        &mut self,
        cl_sender: ClientId,
        cl_receiver: ClientId,
        server_id: ServerId,
        req_source: u8,
    ) {
        self.last_selected_cl_receiver = cl_receiver;
        self.last_selected_cl_sender = cl_sender;
        self.last_selected_sr = server_id;
        self.file_request_source = req_source;
    }

    /// ( Client Sender, Client Receiver, Server )
    pub(crate) fn get_fields(&self) -> (ClientId, ClientId, ServerId) {
        (
            self.last_selected_cl_sender,
            self.last_selected_cl_receiver,
            self.last_selected_sr,
        )
    }
}

#[derive(PartialEq, Eq, Debug)]
pub(crate) enum PopupSelection {
    MenuArea,
    OptArea,
}

// This section is needed to keep always the current viewed in memory ready for the renderer.
// Improves design logic, better user-event handling and easier rendering.

#[derive(Debug, Clone)]
pub(crate) enum ViewedTab1 {
    Main,
    Popup((NodeId, NodePopup)), // NodeId of the selected item
}
#[derive(Debug, Clone)]
pub(crate) enum NodePopup {
    Client(ClientPopup),
    Server(ServerPopup),
    Drone(DronePopup),
}
#[derive(Debug, Clone)]
pub(crate) enum ClientPopup {
    ChatPage(ClientChatPopup),    // Choose the server to communicate with
    InternetPage(ClientNetPopup), // Choose the contact you want to chat with
    DataPage(ClientDataPopup),    // Message with the selected contact
    SettingsPage(ClientSettingsPopup),
}
#[derive(Debug, Clone)]
pub(crate) enum ClientChatPopup {
    FirstPage(ChFirstPage),   // Choose the server to communicate with
    SecondPage(ChSecondPage), // Choose the contact you want to chat with
    ThirdPage(ChThirdPage),   // Message with the selected contact
}
#[derive(Debug, Clone, Default)]
pub(crate) struct ChFirstPage {
    // Depending on the selected client, we got to view the available servers the client knows.
    pub scroll_offset: u8,
}
#[derive(Debug, Clone)]
pub(crate) struct ChSecondPage {
    pub selected_server: NodeId,
}
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) enum ChatThirdPageSelection {
    #[default]
    BottomTab,
    ScrollingMessages,
}
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ChThirdPage {
    pub scroll_offset: u8,
    pub selected_item: ChatThirdPageSelection,

    pub selected_server: NodeId,
    pub selected_contact: NodeId,
}
#[derive(Debug, Clone)]
pub(crate) enum ClientNetPopup {
    FirstPage(NetFirstPage),   // Choose the server to communicate with
    SecondPage(NetSecondPage), // Choose the media you want to receive and view it
}
#[derive(Debug, Clone, Default)]
pub(crate) struct NetFirstPage {
    // Depending on the selected client, we got to view the available servers the client knows.
    pub scroll_offset: u8,
}
#[derive(Debug, Clone, Default)]
pub(crate) struct NetSecondPage {
    pub selected_server: NodeId,
    pub selected_file: SelectedFileMedia,
    pub image_rendering_data: RenderingImage,
    pub selected_item: NetSecondPageItems,
    // Depending on the selected client, we got to view the available servers the client knows.
    pub scroll_offset: u8,
}

#[derive(Debug, Clone, Default)]
pub(crate) enum NetSecondPageItems {
    #[default]
    Blocks,
    FileList,
    MediaList,
}
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) enum SelectedFileMedia {
    #[default]
    None,
    File(FileId),
    Media(MediaId),
}

#[derive(Debug, Clone)]
pub(crate) enum ClientDataPopup {
    FirstPage(), // View some statistics
}
#[derive(Debug, Clone)]
pub(crate) struct ClientSettingsPopup {
    pub command_offset: u8, // Which command I've selected

    pub command_popup: ClientSettingsCommandsPopup,
}
impl ClientSettingsPopup {
    pub fn command_up(&mut self) {
        if self.command_offset > 0 {
            self.command_offset -= 1;
        }
    }
    pub fn command_down(&mut self) {
        if self.command_offset < 2 {
            self.command_offset += 1;
        }
    }
}

#[derive(Default, Clone, Debug)]
pub(crate) enum ClientSettingsCommandsPopup {
    #[default]
    None,
    AddSender(CommandPopup),
    RemoveSender(CommandPopup),
}

#[derive(Debug, Clone)]
pub(crate) enum ServerPopup {
    DataPage(ServerDataPopup),
    SettingsPage(ServerSettingsPopup),
}
#[derive(Clone, Debug)]
pub(crate) struct ServerSettingsPopup {
    pub command_offset: u8, // Which command I've selected

    pub command_popup: ServerSettingsCommandsPopup,
}
impl ServerSettingsPopup {
    pub fn command_up(&mut self) {
        if self.command_offset > 0 {
            self.command_offset -= 1;
        }
    }
    pub fn command_down(&mut self) {
        if self.command_offset < 2 {
            self.command_offset += 1;
        }
    }
}
#[derive(Default, Clone, Debug)]
pub(crate) enum ServerSettingsCommandsPopup {
    #[default]
    None,
    AddSender(CommandPopup),
    RemoveSender(CommandPopup),
}

#[derive(Debug, Clone)]
pub(crate) enum ServerDataPopup {
    FirstPage(), // View some statistics
}
#[derive(Debug, Clone)]
pub(crate) enum DronePopup {
    DataPage(DroneDataPopup),
    SettingsPage(DroneSettingsPopup),
}

#[derive(Debug, Clone)]
pub(crate) enum DroneDataPopup {
    FirstPage(), // View some statistics
}
/*#[derive(Debug, Clone)]
pub(crate) enum DroneSettingsPopup{
    FirstPage(DroneSettingsFirstPage), // View the drone popup settings
}*/
#[derive(Clone, Debug)]
pub(crate) struct DroneSettingsPopup {
    pub command_offset: u8, // Which command I've selected

    pub drone_pdr: f32,
    pub command_popup: DroneSettingsCommandsPopup,
}

impl DroneSettingsPopup {
    pub fn command_up(&mut self) {
        if self.command_offset > 0 {
            self.command_offset -= 1;
        }
    }
    pub fn command_down(&mut self) {
        if self.command_offset < 4 {
            self.command_offset += 1;
        }
    }
}
#[derive(Default, Clone, Debug)]
pub(crate) enum DroneSettingsCommandsPopup {
    #[default]
    None,
    SetPDR(CommandPopup),
    AddSender(CommandPopup),
    RemoveSender(CommandPopup),
    Crash(CommandPopup),
}

// Common between all the nodes

#[derive(Clone, Debug)]
pub(crate) struct CommandPopup {
    pub status: SettingsPopupStatus,
    pub cursor: u8,
}

#[derive(Clone, Debug)]
pub(crate) enum SettingsPopupStatus {
    Idle,
    CorrectlySent,
    Error(String),
}

// END LOGIC SECTION
// - - - - - - - - - - - - - - -

/// Tab1 Impl fns
impl Tab1 {
    pub(crate) fn default() -> Self {
        Self {
            net_node_list: NetNodeList::default(),
            media_list_render_utils: MediaListRenderUtils::default(),
            file_list_render_utils: FileListRenderUtils::default(),
            selected_fn: FnList::default(),
            popup_cursor: 0,

            popup_selection: PopupSelection::MenuArea,
            image_rendering_data: RenderingImage::False,
            file_exploring_data: FileExploring::default(),
            viewing: ViewedTab1::Main,
            navigation_history: Vec::new(), // Track navigation history for backspace
        }
    }
    pub(crate) fn return_to_main(&mut self) {
        self.popup_selection = PopupSelection::MenuArea;
        self.popup_cursor = 0;
        self.viewing = ViewedTab1::Main;
        self.navigation_history.clear(); // Clear history when returning to main
    }

    /// Push current state to navigation history
    pub(crate) fn push_navigation_state(&mut self, state: ViewedTab1) {
        self.navigation_history.push(state);
    }

    /// Pop the last navigation state (for backspace)
    pub(crate) fn pop_navigation_state(&mut self) -> Option<ViewedTab1> {
        self.navigation_history.pop()
    }

    /// Get the current navigation depth
    pub(crate) fn navigation_depth(&self) -> usize {
        self.navigation_history.len()
    }

    /// Check if we can go back (have navigation history)
    pub(crate) fn can_go_back(&self) -> bool {
        !self.navigation_history.is_empty()
    }
}
