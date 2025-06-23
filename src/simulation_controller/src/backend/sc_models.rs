use std::collections::HashMap;
use host::{ClientId, FileId, Image, MediaId, ServerId};
use toml::value::{Datetime, Time};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::sync::mpsc::{Receiver, Sender};          // <-- concrete type
use server::chat2::{Chat, MessageMetaData, MessageSC::*, ChatUsers};
use ratatui_image::thread::{ResizeRequest, ResizeResponse, ThreadProtocol};
use server::MessageSC::Message;
use crate::backend::sc_models::FileRendererUpdate::Redraw;

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

pub enum RendererCommand{
    Stop, AddChannel((Sender<FileRendererUpdate>, Receiver<ResizeRequest>)), Encode((MediaId,Image))
}
pub enum RendererResponse{
    Outcome((LogType,String)), Encoded(ThreadProtocol)
}

pub struct NetCache{
    // Simply all the chat server the Client knows.
    pub known_content_servers: HashMap<ClientId, Vec<ServerId>>,
    
    pub known_file_list: HashMap<ClientId, HashMap<ServerId, Vec<FileId>>>,
    pub known_media_list: HashMap<ClientId, HashMap<ServerId, Vec<MediaId>>>,

    pub downloaded_medias: HashMap<ServerId,
        HashMap<MediaId,
            ((Option<std::sync::mpsc::Sender<ResizeRequest>>,Option<std::sync::mpsc::Receiver<FileRendererUpdate>>), NetCacheMedia)>>,
    pub downloaded_files: HashMap<ServerId, HashMap<FileId,((),NetCacheFile)>>
    // Each file has its own communication channel to identify it in a univocal way in the renderer thread.
    //
}

pub enum FileRendererUpdate {
    Redraw(ResizeResponse), Error(String)
}

impl NetCache{
    pub fn default() -> Self{
        Self{
            known_content_servers: Default::default(),
            known_file_list: Default::default(),
            known_media_list: Default::default(),
            downloaded_medias: Default::default(),
            downloaded_files: Default::default(),
        }
    }
}

pub enum NetCacheMedia {
    Downloading(f32), // f32 € [0,100] representing client loading status.
    Downloaded(Image), // ByteStream of the file the clients sends to the Sim. Controller
    Rendering(f32), // f32 € [0,100] representing Sim. Controller loading status.
    Rendered(ThreadProtocol), // Rendered file that has to be
    // re-rendered each time Terminal changes its size.
    FatalErrorRendering(), // Fatal error rendering, file has to be re-sent!
}
pub enum NetCacheFile{
    Downloading(f32), // f32 € [0,100] representing client loading status.
    Downloaded(Vec<u8>), // ByteStream of the file the clients sends to the Sim. Controller
    Rendering(f32),
    Rendered(File)
}


/*
Now, I want a single worker thread to render each of my files.
First idea could be:

A main crossbeam_channels / mpsc channels for direct important communications from Simulation Controller.
- Add file channels to render
- Render

Then each file will have its own channels to identify them in a univocal way.
 */

pub struct ChatCache{
    // Gets updated each time a chat-history gets modified via-server
    // Each server has its own ChatList saved
    pub local_chats: HashMap<ServerId, ChatList>,

    // Gets updated each time we receive the relative data-packet
    // Each client has its own list in each server.
    // First Key is ClientID, Value is all the contacts it has associated with the relative server.
    pub known_contacts: HashMap<ClientId, HashMap<ServerId, Vec<ClientId>>>,

    // Simply all the chat server the Client knows.
    pub known_chat_servers: HashMap<ClientId, Vec<ServerId>>,

    // First menu data
    // Starts to 0 and ends at known_chat_servers -> Vec<NodeId>.len()
    // Last number points to the button
    pub first_menu_scroll_offset: usize
}

impl ChatCache{
    pub fn default() ->Self{
        Self{
            local_chats: Default::default(), /*{
                let mut ret_hash = HashMap::new();
                ret_hash.insert(11, ChatList::default_dummy());
                ret_hash
            },*/
            known_contacts: Default::default(),
            known_chat_servers: Default::default(),
            first_menu_scroll_offset: 0,
        }
    }
}

#[derive(Debug)]
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
        let users_pair = (8, 9);
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
                Message(text),
                MessageMetaData { id: sender, timestamp },
            ));
        }
        let chat = Chat {
            chat: history,
            users: ChatUsers { user1: 8, user2: 9 },
        };
        ChatList {
            chats_stored: {
                let mut hash_map = HashMap::default();
                hash_map.insert(users_pair, chat);
                hash_map
            },
        }
    }
}

pub(crate) enum MediaSC {
    Image(/* A Reconstructed Image File */), File(/* A Reconstructed File */)
}

pub enum DroneType {
    RollingDrone,
    LockheedRustin,
    BagelBomber,
    FlyPath,
    RustDrone,
    RustasticDrone,
    BoberDrone,
    RustyDrone,
    RustaceansDrone,
    KrustyDrone,
    Unknown
}

impl Display for DroneType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            DroneType::RollingDrone => "RollingDrone",
            DroneType::LockheedRustin => "LockheedRustin",
            DroneType::BagelBomber => "BagelBomber",
            DroneType::FlyPath => "FlyPath",
            DroneType::RustDrone => "RustDrone",
            DroneType::RustasticDrone => "RustasticDrone",
            DroneType::BoberDrone => "BoberDrone",
            DroneType::RustyDrone => "RustyDrone",
            DroneType::RustaceansDrone => "RustaceansDrone",
            DroneType::KrustyDrone => "KrustyDrone",
            DroneType::Unknown => "Unknown"
        };
        write!(f, "{}", name)
    }
}