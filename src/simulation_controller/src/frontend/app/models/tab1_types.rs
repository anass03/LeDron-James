use wg_2024::network::NodeId;
use toml::value::Datetime;
use toml::value::Time;
use std::collections::HashMap;
use rand::distr::weighted::Weight;
use serde::Serialize;
use crate::frontend::app::models::general_types::{FnList, Node, NodeList};


/// Tab n.01 data
pub(in crate::frontend::app) struct Tab1{
    pub nodelist: NodeList,
    pub selected_fn: FnList,

    // Popup cursor and selection works accordingly to each other.
    // Popup selection helps to understand which item cursor refers to
    pub popup_cursor: u8,
    pub popup_selection: PopupSelection,

    pub viewing: ViewedTab1,



    pub data_chat: ChatCache,
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
    InternetPage(ClSecondPage), // Choose the contact you wanna chat with
    DataPage(ClThirdPage), // Message with the selected contact
    SettingsPage()
}
#[derive(Debug, Clone)]
pub(crate) enum ClientChatPopup{
    FirstPage(ClFirstPage), // Choose the server to communicate with
    SecondPage(ClSecondPage), // Choose the contact you wanna chat with
    ThirdPage(ClThirdPage) // Message with the selected contact
}
#[derive(Debug, Clone, Default)]
pub(crate) struct ClFirstPage{
    // Depending on the selected client, we gotta view the available servers the client knows.
    pub scroll_offset:u8
}
#[derive(Debug, Clone)]
pub(crate) struct ClSecondPage{
    pub selected_server: NodeId
}
#[derive(Debug, Clone)]
pub(crate) struct ClThirdPage{
    pub selected_server: NodeId,
    pub selected_contact: NodeId
}
#[derive(Debug, Clone)]
pub(crate) enum ClientNetPopup{
    FirstPage(), // Choose the server to communicate with
    SecondPage(), // Choose the media you want to receive and view it
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


pub(crate) struct ChatCache{
    // Gets updated each time a chat-history gets modified via-server
    // Each server has its own ChatList saved
    pub local_chats: HashMap<NodeId, ChatList>,

    // Gets updated each time we receive the relative data-packet
    // Each client has its own list in each server.
    // First Key is ClientID, Value is all the contacts it has associated with the relative server.
    pub known_contacts: HashMap<NodeId, HashMap<NodeId, Vec<NodeId>>>,

    // Simply all the chat server the Client knows.
    pub known_chat_servers: HashMap<NodeId, Vec<NodeId>>,

    // First menu data
    // Starts to 0 and ends at known_chat_servers -> Vec<NodeId>.len()
    // Last number points to the button
    pub first_menu_scroll_offset: usize
}
impl ChatCache{
    pub(crate) fn default()->Self{
        Self{
            local_chats: {
                let mut map = HashMap::new();
                map.insert(1, ChatList::default());
                map
            },

            known_contacts: {
                let mut map = HashMap::new();
                let mut map2 = HashMap::new();
                map2.insert(0, vec![1,2,3,4,5]);
                map.insert(1, map2);
                map
            },
            known_chat_servers: {
                let mut map = HashMap::default();
                map.insert(NodeId::ZERO, vec![1,2,3,4]);
                map
            },

            first_menu_scroll_offset: 0,
        }
    }
}

pub(crate) struct ChatList{
    // We keep a copy of the users chats even in the vec for easier finding.
    pub chats_stored: Vec<((NodeId,NodeId),Chat)>,
}
impl ChatList{
    fn default() -> Self {
        // Build a dummy chat history of 20 alternating messages
        let users_pair = (2, 1);
        let mut history = Vec::with_capacity(20);
        for i in 0..20 {
            // Alternate sender: even index -> Node 2, odd -> Node 1
            let sender = if i % 2 == 0 { 2 } else { 1 };
            // Generate message text
            let text = format!("Test message {}\n Hello fdjsnfjsdn fnjsdkn fknsdknfjksd njfnsdkjnfnsdkkfsdjf kdsnjfndsjfnsd jbpjhièjhihuhhuuiphhuih uhuiph gdfsughfdi gdfpihgsughupi fudsgdfuisg dfgudfhgf gsp", i + 1);
            // Generate timestamps starting at 10:00 and +1 minute each
            let minute = (i % 60) as u8;
            let hour = 10 + (i / 60) as u8;
            let timestamp = Datetime {
                date: None,
                time: Some(Time { hour, minute, second: 0, nanosecond: 0 }),
                offset: None,
            };
            history.push((
                Message::Message(text),
                MessageMetadata { id_sender: sender, timestamp },
            ));
        }
        let chat = Chat {
            chat: history,
            users: ChatUsers { user1: 2, user2: 1 },
        };
        ChatList {
            chats_stored: vec![(users_pair, chat)],
        }
    }
    fn is_there(){

    }
    fn last_message(){

    }
}

pub(crate) struct Chat{
    pub chat: Vec<(Message, MessageMetadata)>, // From older to newer message
    users: ChatUsers, // Users that participate in the chat
}
impl Chat{
    pub fn get_last_message(&self)->Option<&(Message,MessageMetadata)>{
        self.chat.last()
    }
}

pub(crate) struct ChatUsers{ // Order does NOT matter
    user1: NodeId,
    user2: NodeId,
}

pub(crate) struct MessageMetadata{
    pub id_sender: NodeId,
    pub timestamp: Datetime,
}

pub(crate) enum Message{
    Message(String), Media(MediaSC)
}

pub(crate) enum MediaSC {
    Image(/* An Recostructed Image File */), File(/* An Recostructed File */)
}

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

            data_chat: ChatCache::default(),
        }
    }
}