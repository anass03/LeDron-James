use wg_2024::network::NodeId;
use std::collections::HashMap;
use host::{ClientId, ServerId};
use toml::value::{Datetime, Time};
use rand::distr::weighted::Weight;
use wg_2024::config::Server;

/// Used in multiple crates, needs to be pub
#[derive(Clone)]
pub enum LogType{
    WARN, ERROR, INFO, DEBUG, FATAL
}



/// Used in multiple crates, needs to be pub
#[derive(Clone)]
pub struct Logs{
    pub logs_memory: Vec<(LogType,String)>,
    pub log_scroll_offset: usize,
}

pub struct Statistics{
    pub total_packet_exchanged: usize,
    pub total_active_connections: usize,
    pub network_reliability: u8,
}

// Implementations

impl Logs{
    pub fn default() ->Self{
        Self{
            logs_memory: vec![],
            log_scroll_offset: 0,
        }
    }

    pub (crate) fn push_sc_log(&mut self, log_type: LogType, text: String) {
        self.logs_memory.push((log_type, format!("[Simulation Controller] - {text}")));
    }
    pub fn push_log(&mut self, log_type: LogType, text: &str){
        if self.logs_memory.len()>=1000{
            self.logs_memory.remove(0);
            self.logs_memory.push((log_type, text.to_string()));
        }else{
            self.logs_memory.push((log_type, text.to_string()));
            self.log_scroll_offset += 1;
        }
    }
}
impl Statistics{
    pub fn default()->Self{
        Self{
            total_packet_exchanged: 0,
            total_active_connections: 0,
            network_reliability: 0,
        }
    }
    pub fn received_packet(&mut self){
        self.total_packet_exchanged += 1;
    }
}



pub struct ChatCache{
    // Gets updated each time a chat-history gets modified via-server
    // Each server has its own ChatList saved
    pub local_chats: HashMap<ServerId, ChatList>,

    // Gets updated each time we receive the relative data-packet
    // Each client has its own list in each server.
    // First Key is ClientID, Value is all the contacts it has associated with the relative server.
    pub known_contacts: HashMap<ClientId, HashMap<ServerId, Vec<ClientId>>>,

    // Simply all the chat server the Client knows.
    pub known_chat_servers: HashMap<NodeId, Vec<NodeId>>,

    // First menu data
    // Starts to 0 and ends at known_chat_servers -> Vec<NodeId>.len()
    // Last number points to the button
    pub first_menu_scroll_offset: usize
}

impl ChatCache{
    pub fn default() ->Self{
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

    // CONVERTI IN HASHMAP E SISTEMA IN TAB1 RENDERING
    // FINISCI EVENT HANDLING CHAT
    // IMPORTA CRATE EXTERNAL FILE EXPLORER
    // DATA PAGE
    pub chats_stored: HashMap<(ClientId, ClientId), Chat>
}

impl ChatList{
    fn default() -> Self{
        Self{
            chats_stored: HashMap::new(),
        }
    }
    fn default_dummy() -> Self {
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
            chats_stored: {
                let mut hash_map = HashMap::default();
                hash_map.insert(users_pair, chat);
                hash_map
            },
        }
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