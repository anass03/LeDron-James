use host::{ClientId, FileId, MediaId, ServerId};
use ratatui::layout::Rect;
use ratatui::widgets::ListState;
use wg_2024::network::NodeId;
use crate::backend::sc_models::ChatCache;
use crate::frontend::app::models::general_types::{FnList, NetNodeList};


/// Tab n.01 data
pub(in crate::frontend::app) struct Tab1{
    pub netnode_list: NetNodeList,

    pub media_list_render_utils: MediaListRenderUtils,
    pub file_list_render_utils: FileListRenderUtils,

    // Popup cursor and selection works accordingly to each other.
    // Popup selection helps to understand which item cursor refers to
    pub popup_cursor: usize,
    pub popup_selection: PopupSelection,

    // Needed fast accessible data for Tab1-Third Page
    pub chat_third_page_selection: ChatThirdPageSelection,

    pub image_rendering_data: RenderingImage,
    pub file_exploring_data: FileExploring,

    pub viewing: ViewedTab1,

    pub selected_fn: FnList, // Deprecated
}
#[derive(Clone)]
pub(crate) enum RenderingImage{
    False, True((Rect,ClientId, ServerId, MediaId)),
}
pub(crate) struct MediaListRenderUtils{
    pub selected: u8,
    pub state: ListState,
}
impl MediaListRenderUtils{
    fn default()->Self{
        Self{
            selected: 0,
            state: Default::default(),
        }
    }
}
pub(crate) struct FileListRenderUtils{
    pub selected: u8,
    pub state: ListState,
}
impl FileListRenderUtils{
    fn default()->Self{
        Self{
            selected: 0,
            state: Default::default(),
        }
    }
}
pub(crate) struct FileExploring {
    last_selected_cl_sender: ClientId,
    last_selected_cl_receiver: ClientId,
    last_selected_sr: ServerId,
}
impl FileExploring {
    fn default()->Self{
        Self{
            last_selected_cl_sender: 0,
            last_selected_cl_receiver: 0,
            last_selected_sr: 0,
        }
    }
    pub(crate) fn set_fields(&mut self, cl_sender:ClientId, cl_receiver:ClientId, server_id:ServerId) {
        self.last_selected_cl_receiver = cl_receiver;
        self.last_selected_cl_sender = cl_sender;
        self.last_selected_sr = server_id;
    }

    /// ( Client Sender, Client Receiver, Server )
    pub(crate) fn get_fields(&self)->(ClientId, ClientId, ServerId) {
        (self.last_selected_cl_sender, self.last_selected_cl_receiver, self.last_selected_sr)
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ChatThirdPageSelection{
    ScrollingMessages, BottomTab
}


#[derive(PartialEq, Eq, Debug)]
pub(crate) enum PopupSelection{
    MenuArea, OptArea
}

// This section is needed to keep always the current viewed in memory ready for the renderer.
// Improves design logic, better user-event handling and easier rendering.

#[derive(Debug, Clone)]
pub(crate) enum ViewedTab1{
    Main, Popup((NodeId,NodePopup)) // NodeId of the selected item
}
#[derive(Debug, Clone)]
pub(crate) enum NodePopup{
    Client(ClientPopup),
    Server(ServerPopup),
    Drone(DronePopup),
}
#[derive(Debug, Clone)]
pub(crate) enum ClientPopup {
    ChatPage(ClientChatPopup), // Choose the server to communicate with
    InternetPage(ClientNetPopup), // Choose the contact you wanna chat with
    DataPage(ClientDataPopup), // Message with the selected contact
    SettingsPage(ClientSettingsPopup)
}
#[derive(Debug, Clone)]
pub(crate) enum ClientChatPopup{
    FirstPage(ChFirstPage), // Choose the server to communicate with
    SecondPage(ChSecondPage), // Choose the contact you wanna chat with
    ThirdPage(ChThirdPage) // Message with the selected contact
}
#[derive(Debug, Clone, Default)]
pub(crate) struct ChFirstPage {
    // Depending on the selected client, we gotta view the available servers the client knows.
    pub scroll_offset:u8
}
#[derive(Debug, Clone)]
pub(crate) struct ChSecondPage {
    pub selected_server: NodeId
}
#[derive(Debug, Clone)]
pub(crate) struct ChThirdPage {
    pub selected_server: NodeId,
    pub selected_contact: NodeId
}
#[derive(Debug, Clone)]
pub(crate) enum ClientNetPopup{
    FirstPage(NetFirstPage), // Choose the server to communicate with
    SecondPage(NetSecondPage), // Choose the media you want to receive and view it
}
#[derive(Debug, Clone, Default)]
pub(crate) struct NetFirstPage {
    // Depending on the selected client, we gotta view the available servers the client knows.
    pub scroll_offset:u8
}
#[derive(Debug, Clone, Default)]
pub(crate) struct NetSecondPage {
    pub selected_server: NodeId,

    pub selected_file: SelectedFileMedia,

    // Depending on the selected client, we gotta view the available servers the client knows.
    pub scroll_offset:u8
}
#[derive(Debug, Clone, Default)]
pub(crate) enum SelectedFileMedia{
    #[default]
    None, File(FileId), Media(MediaId)
}

#[derive(Debug, Clone)]
pub(crate) enum ClientDataPopup{
    FirstPage(), // View some statistics
}
#[derive(Debug, Clone)]
pub(crate) enum ClientSettingsPopup{
    FirstPage(), // View the client popup settings
}

#[derive(Debug, Clone)]
pub(crate) enum ServerPopup{
    FirstPage()
}
#[derive(Debug, Clone)]
pub(crate) enum DronePopup{
    DataPage(),
    SettingsPage()
}

// END LOGIC SECTION
// - - - - - - - - - - - - - - -


/// Tab1 Impl fns
impl Tab1{
    pub(crate) fn default() ->Self{
        Self{
            netnode_list: NetNodeList::default(),
            media_list_render_utils: MediaListRenderUtils::default(),
            file_list_render_utils: FileListRenderUtils::default(),
            selected_fn: FnList::default(),
            popup_cursor: 0,

            popup_selection: PopupSelection::MenuArea,
            chat_third_page_selection: ChatThirdPageSelection::BottomTab,
            image_rendering_data: RenderingImage::False,
            file_exploring_data: FileExploring::default(),
            viewing: ViewedTab1::Main
            /*ViewedTab1::Popup((1, NodePopup::Client(ClientPopup::ChatPage(ClientChatPopup::ThirdPage(ClThirdPage{
                selected_server: 1,
                selected_contact: 2,
            })))))*/,

        }
    }
    pub(crate) fn return_to_main(&mut self){
        self.popup_selection = PopupSelection::MenuArea;
        self.popup_cursor = 0;
        self.viewing = ViewedTab1::Main;
    }
}