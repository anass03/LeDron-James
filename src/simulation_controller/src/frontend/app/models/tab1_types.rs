use wg_2024::network::NodeId;
use crate::backend::sc_models::ChatCache;
use crate::frontend::app::models::general_types::{FnList, NodeList};


/// Tab n.01 data
pub(in crate::frontend::app) struct Tab1{
    pub nodelist: NodeList,
    pub selected_fn: FnList,

    // Popup cursor and selection works accordingly to each other.
    // Popup selection helps to understand which item cursor refers to
    pub popup_cursor: u8,
    pub popup_selection: PopupSelection,

    pub viewing: ViewedTab1,
}

#[derive(PartialEq,Eq)]
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
    // Depending on the selected client, we gotta view the available servers the client knows.
    pub scroll_offset:u8
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
    FirstPage()
}

// END LOGIC SECTION
// - - - - - - - - - - - - - - -


/// Tab1 Impl fns
impl Tab1{
    pub(crate) fn default() ->Self{
        Self{
            nodelist: NodeList::default(),
            selected_fn: FnList::default(),
            popup_cursor: 0,

            popup_selection: PopupSelection::MenuArea,
            viewing: ViewedTab1::Main
            /*ViewedTab1::Popup((1, NodePopup::Client(ClientPopup::ChatPage(ClientChatPopup::ThirdPage(ClThirdPage{
                selected_server: 1,
                selected_contact: 2,
            })))))*/,

        }
    }
}