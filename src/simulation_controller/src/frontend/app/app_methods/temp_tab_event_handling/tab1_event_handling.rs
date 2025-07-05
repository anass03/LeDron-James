use std::cmp::PartialEq;
use std::collections::HashMap;
use std::ops::DerefMut;
use host::{ClientId, FileId, MediaId};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use wg_2024::network::NodeId;
use server::chat2::Chat;
use crate::backend::sc_models::{ChatList, LogType, NetCacheMedia};
use crate::backend::simulation_controller::NetNode;
use crate::frontend::app::App;
use crate::frontend::app::models::general_types::Node;
use crate::frontend::app::models::main_types::AppMode;
use crate::frontend::app::models::tab1_types::{ChFirstPage, ChSecondPage, ChThirdPage, ChatThirdPageSelection, ClientChatPopup, ClientDataPopup, ClientNetPopup, ClientPopup, ClientSettingsPopup, CommandPopup, DroneDataPopup, DronePopup, DroneSettingsCommandsPopup, DroneSettingsItems, DroneSettingsPopup, NetFirstPage, NetSecondPage, NetSecondPageItems, NodePopup, PopupSelection, RenderingImage, SelectedFileMedia, ServerPopup, SettingsPopupStatus, ViewedTab1};
use crate::frontend::app::topology_algorithm::alg_models::TopologyAlgorithm;

impl App<'_> {
    pub(crate) fn handle_event_tab1(&mut self, key: KeyEvent) {
        match self.cache.tab1_data.viewing.clone(){
            ViewedTab1::Main => {
                self.handle_main(key)
            }
            ViewedTab1::Popup((id,node_popup)) => {
                self.handle_popup(key,id,node_popup);
            }
        }
    }
    fn handle_main(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => {
                self.cache.tab1_data.netnode_list.select_prev();
            }
            KeyCode::Down => {
                self.cache.tab1_data.netnode_list.select_next();
            }
            KeyCode::Right | KeyCode::Left=>{
                // Depending on where we I handle the right key
                match self.cache.cursor{
                    0 => {
                        // I am on List
                        self.cache.cursor += 1;
                    }
                    1 => {
                        // I am on Map
                        self.cache.cursor -= 1;
                    }
                    _ => ()
                }
            }
            KeyCode::Enter => {
                match self.cache.cursor{
                    0 => {
                        // I am on List
                        // Which item I've selected?
                        let selected_cursor_item = self.cache.tab1_data.netnode_list.selected as usize;
                        match self.cache.tab1_data.netnode_list.items.get(selected_cursor_item){
                            None => {

                            }
                            Some((node,is_active,_)) => {
                                if *is_active{
                                    match node{
                                        Node::Client(cl) => {
                                            self.cache.tab1_data.viewing =
                                                ViewedTab1::Popup((
                                                    cl.id,
                                                    NodePopup::Client(ClientPopup::ChatPage(
                                                        ClientChatPopup::FirstPage(
                                                            ChFirstPage::default()
                                                        )
                                                    ))
                                                ))
                                        }
                                        Node::Server(sr) => {
                                            self.cache.tab1_data.viewing =
                                                ViewedTab1::Popup((
                                                    sr.id,
                                                    NodePopup::Server(ServerPopup::FirstPage())
                                                ))
                                        }
                                        Node::Drone(dr) => {
                                            self.cache.tab1_data.viewing =
                                                ViewedTab1::Popup((
                                                    dr.id,
                                                    NodePopup::Drone(DronePopup::DataPage(DroneDataPopup::FirstPage()))
                                                ))
                                        }
                                    }
                                }else{
                                    // Throw a warning POPUP saying the item is not ACTIVE
                                }
                            }
                        }
                    }
                    1 => {
                        // I am on Map
                        // I still don't know what could happen when I press map
                    }
                    _ => ()
                }
            }

            // Missing KeyCodes to view the first one / last one, still gotta decide!
            _ => (), // Do nothing
        }
    }

    fn handle_popup(&mut self, key:KeyEvent, id: NodeId, node_popup: NodePopup) {
        match key.code{
            KeyCode::Esc => {
                self.cache.tab1_data.return_to_main()
            }
            _ => (),
        }
        match node_popup {
            NodePopup::Client(cl) => {
                self.handle_client_popup(key,id,cl)
            }
            NodePopup::Server(sr) => {
                self.handle_server_popup(key,id,sr)
            }
            NodePopup::Drone(dr) => {
                self.handle_drone_popup(key,id,dr)
            }
        }
    }

    // We load cursor and the selected client to fn for easier tasking.
    fn coordinate_client_popup_cursor_with_opt_menu(&mut self, id:NodeId, current_cursor: usize){
        match current_cursor{
            0 => {
                self.cache.tab1_data.viewing =
                    ViewedTab1::Popup((
                        id,
                        NodePopup::Client(ClientPopup::ChatPage(
                            ClientChatPopup::FirstPage(
                                ChFirstPage::default()
                            )
                        ))
                    ))
            }
            1 => {
                self.cache.tab1_data.viewing =
                    ViewedTab1::Popup((
                        id,
                        NodePopup::Client(ClientPopup::InternetPage(
                            ClientNetPopup::FirstPage(
                                NetFirstPage::default()
                            )
                        ))
                    ))
            }
            2 => {
                self.cache.tab1_data.viewing =
                    ViewedTab1::Popup((
                        id,
                        NodePopup::Client(ClientPopup::DataPage(
                            ClientDataPopup::FirstPage()
                        ))
                    ))
            }
            3 => {
                self.cache.tab1_data.viewing =
                    ViewedTab1::Popup((
                        id,
                        NodePopup::Client(ClientPopup::SettingsPage(
                            ClientSettingsPopup::FirstPage()
                        ))
                    ))
            }
            _ => (),
        }
    }
    fn coordinate_drone_popup_cursor_with_opt_menu(&mut self, id:NodeId, current_cursor: usize){
        match current_cursor{
            0 => {
               self.cache.tab1_data.viewing =
                   ViewedTab1::Popup((
                       id,
                       NodePopup::Drone(
                           DronePopup::DataPage(
                               DroneDataPopup::FirstPage()
                           )
                       )
                   ));
            }
            1 => {
                self.cache.tab1_data.viewing =
                    ViewedTab1::Popup((
                        id,
                        NodePopup::Drone(
                            DronePopup::SettingsPage(
                                DroneSettingsPopup{ command_list_state: Default::default(), command_offset: 0, selected_item: DroneSettingsItems::Blocks, drone_pdr: 0, command_popup: Default::default() })
                            )
                        )
                    );
            }
            _ => {
                self.cache.tab1_data.popup_cursor = 0;
            }
        }
    }
    fn handle_client_popup(&mut self, key: KeyEvent,id:NodeId, client_popup: ClientPopup){
        match self.cache.tab1_data.popup_selection {
            PopupSelection::MenuArea => {
                match key.code{
                    KeyCode::Up => {
                        if self.cache.tab1_data.popup_cursor > 0{
                            self.cache.tab1_data.popup_cursor -= 1;
                        }else{
                            self.cache.tab1_data.popup_cursor = 3;
                        }
                        self.coordinate_client_popup_cursor_with_opt_menu(id, self.cache.tab1_data.popup_cursor);
                    }
                    KeyCode::Down => {
                        if self.cache.tab1_data.popup_cursor < 3{
                            self.cache.tab1_data.popup_cursor += 1;
                        }else{
                            self.cache.tab1_data.popup_cursor = 0;
                        }
                        self.coordinate_client_popup_cursor_with_opt_menu(id, self.cache.tab1_data.popup_cursor);
                    }
                    KeyCode::Right | KeyCode::Enter => {
                        self.cache.tab1_data.popup_selection = PopupSelection::OptArea;
                        self.coordinate_client_popup_cursor_with_opt_menu(id, self.cache.tab1_data.popup_cursor);
                        self.cache.tab1_data.popup_cursor = 0;
                    }
                    _ => (),
                }
            }
            PopupSelection::OptArea => {
                match client_popup {
                    ClientPopup::ChatPage(ch_page) => {
                        self.handle_chat_page(key,id,ch_page)
                    }
                    ClientPopup::InternetPage(net_page) => {
                        self.handle_net_page(key,id,net_page)
                    }
                    ClientPopup::DataPage(data_page) => {

                    }
                    ClientPopup::SettingsPage(_) => {

                    }
                }
            }
        }
    }
    fn handle_chat_page(&mut self, key: KeyEvent,id:NodeId, chat_page: ClientChatPopup){
        match chat_page {
            ClientChatPopup::FirstPage(landing_data) => {
                // I am outside because this is shared by the two known_chat_servers cases
                match key.code.to_string().to_lowercase().as_str(){
                    "r" => {
                        // Send request
                        match self.simulation_controller_data.request_server_type(id){
                            Ok(_) => {
                                self.simulation_controller_data.logs.push_sc_log(LogType::INFO,"Trying to find new servers in the network!".to_string());
                            }
                            Err(str) => {
                                self.simulation_controller_data.logs.push_sc_log(LogType::ERROR,str);
                            }
                        }
                    },
                    _ => (),
                }

                match &self.simulation_controller_data.chat_cache.known_chat_servers.get(&id){
                    None => {
                        match key.code {
                            KeyCode::Left => {
                                self.cache.tab1_data.popup_selection = PopupSelection::MenuArea;
                                self.cache.tab1_data.popup_cursor = 0; // Re-selecting Chat Menu
                            }
                            _ => ()
                        }
                    },
                    Some(known_chat_servers) => {
                        let chat_servers_len = known_chat_servers.len();
                        match key.code {
                            KeyCode::Up => {
                                if self.cache.tab1_data.popup_cursor>0{
                                    self.cache.tab1_data.popup_cursor-=1;
                                }
                            }
                            KeyCode::Down => {
                                if self.cache.tab1_data.popup_cursor + 1 < chat_servers_len  {
                                    self.cache.tab1_data.popup_cursor+=1;
                                }
                            }
                            KeyCode::Left => {
                                    // Return on Menu
                                self.cache.tab1_data.popup_selection = PopupSelection::MenuArea;
                                self.cache.tab1_data.popup_cursor = 0; // Re-selecting Chat Menu
                            }
                            KeyCode::Enter => {
                                // I get the selected item and pass it to the second page struct
                                match known_chat_servers.get((self.cache.tab1_data.popup_cursor as usize)){
                                    None => {
                                        // Should be impossible
                                        // THROW FATAL
                                    }
                                    Some(selected_server_id) => {
                                        self.cache.tab1_data.popup_cursor = 0;
                                        self.cache.tab1_data.viewing =
                                            ViewedTab1::Popup((
                                                id,
                                                NodePopup::Client(ClientPopup::ChatPage(
                                                    ClientChatPopup::SecondPage(
                                                        ChSecondPage {
                                                            selected_server: *selected_server_id,
                                                        }
                                                    )
                                                ))
                                            ))
                                    }
                                }
                            }
                            _ => (),
                        }
                    }
                }
            }
            ClientChatPopup::SecondPage(contacts_data) => {
                match key.code.to_string().to_lowercase().as_str(){
                    "r" => {
                        // Send request
                        match self.simulation_controller_data.request_client_list(id,contacts_data.selected_server){
                            Ok(_) => (),
                            Err(str) => {
                                self.simulation_controller_data.logs.push_sc_log(LogType::ERROR,str);
                            }
                        }
                    },
                    _ => (),
                }
                // Devo capire dove mi trovo
                match self.simulation_controller_data.chat_cache.known_contacts.get(&id){
                    None => {
                        // Pagina vuota
                    },
                    Some(known_contacts) => {
                        match known_contacts.get(&contacts_data.selected_server){
                            None => {
                                // Pagina vuota
                            }
                            Some(contacts) => {
                                let contacts_len = contacts.len();
                                match key.code {
                                    KeyCode::Left => {
                                        self.cache.tab1_data.popup_selection = PopupSelection::MenuArea;
                                        self.cache.tab1_data.popup_cursor = 0; // Re-selecting Chat Menu
                                    }
                                    KeyCode::Up => {
                                        if self.cache.tab1_data.popup_cursor as isize -1 >=0{
                                            self.cache.tab1_data.popup_cursor-=1
                                        }
                                    }
                                    KeyCode::Down => {
                                        if self.cache.tab1_data.popup_cursor+1 < contacts_len  {
                                            self.cache.tab1_data.popup_cursor+=1
                                        }
                                    }
                                    KeyCode::Enter => {
                                        match contacts.get(self.cache.tab1_data.popup_cursor){
                                            None => {
                                                // Bug, shouldn't arrive here.
                                                // Means I can select non existing items
                                            }
                                            Some(contact) => {
                                                self.cache.tab1_data.popup_cursor = 0;
                                                self.cache.tab1_data.viewing =
                                                    ViewedTab1::Popup((
                                                        id,
                                                        NodePopup::Client(ClientPopup::ChatPage(
                                                            ClientChatPopup::ThirdPage(
                                                                ChThirdPage {
                                                                    selected_server: contacts_data.selected_server,
                                                                    selected_contact: *contact,
                                                                }
                                                            )
                                                        ))
                                                    ))
                                            }
                                        }
                                    }
                                    _ => (),
                                }
                            }
                        }
                    }
                }
            }
            ClientChatPopup::ThirdPage(chat_data) => {
                // Fundamental Matching
                match key.code.to_string().to_lowercase().as_str(){
                    "s" => {
                        self.parse_and_send_message(id, chat_data.selected_contact, chat_data.selected_server);
                    },
                    _ => {

                    }
                }
                // Scrolling messages or bottom tab
                match self.cache.tab1_data.chat_third_page_selection.clone(){
                    ChatThirdPageSelection::ScrollingMessages => {
                        match self.simulation_controller_data.chat_cache.local_chats.get(&chat_data.selected_server){
                            None => {
                                // Empty chat, do NUN but go down
                                match key.code {
                                    KeyCode::Down => {
                                        self.cache.tab1_data.chat_third_page_selection = ChatThirdPageSelection::BottomTab;
                                        self.cache.tab1_data.popup_cursor = 0;
                                    },
                                    _ => ()
                                }
                            }
                            Some(chat_list) => {
                                let found:Option<&Chat> = match chat_list.chats_stored.get(&(id, chat_data.selected_contact)){
                                    None => {
                                        match chat_list.chats_stored.get(&(id, chat_data.selected_contact)){
                                            None => None,
                                            Some(chat_list) => Some(chat_list),
                                        }
                                    }
                                    Some(chat_list) => Some(chat_list),
                                };
                                match found {
                                    None => {
                                        // Empty chat, do NUN but go down
                                        match key.code {
                                            KeyCode::Down => {
                                                self.cache.tab1_data.chat_third_page_selection = ChatThirdPageSelection::BottomTab;
                                                self.cache.tab1_data.popup_cursor = 0;
                                            },
                                            _ => ()
                                        }
                                    }
                                    Some(ch) => {
                                        let chat_len = ch.chat.len();
                                        match key.code {
                                            KeyCode::Up => {
                                                if self.cache.tab1_data.popup_cursor + 1 < chat_len {
                                                    self.cache.tab1_data.popup_cursor+=1;
                                                }
                                            }
                                            KeyCode::Down => {
                                                if self.cache.tab1_data.popup_cursor > 0 {
                                                    self.cache.tab1_data.popup_cursor-=1;
                                                }else{
                                                    self.cache.tab1_data.chat_third_page_selection = ChatThirdPageSelection::BottomTab;
                                                    self.cache.tab1_data.popup_cursor = 0;
                                                }
                                            }
                                            KeyCode::Enter => {
                                                // In base al messaggio che ho selezionato devo scegliere cosa fare
                                                // todo!()
                                            }
                                            _ => ()
                                        }
                                    }
                                }
                            }
                        }
                    }
                    ChatThirdPageSelection::BottomTab => {
                        match key.code {
                            KeyCode::Right => {
                                match self.cache.tab1_data.popup_cursor{
                                    0 => {
                                        self.cache.tab1_data.popup_cursor += 1;
                                    }
                                    _ => (),
                                }
                                },
                            KeyCode::Left => {
                                match self.cache.tab1_data.popup_cursor{
                                    0 => {
                                        self.cache.tab1_data.popup_selection = PopupSelection::MenuArea;
                                    }
                                    1 => {
                                        self.cache.tab1_data.popup_cursor -= 1;
                                    }
                                    _ => (),
                                }
                            }
                            KeyCode::Up => {
                                self.cache.tab1_data.chat_third_page_selection = ChatThirdPageSelection::ScrollingMessages;
                                self.cache.tab1_data.popup_cursor = 0;
                            }
                            KeyCode::Enter => {
                                match self.cache.tab1_data.popup_cursor{
                                    0 => {
                                        self.app_mode = AppMode::Input;
                                    }
                                    1 => {
                                        // Attach a file, il restituito andrà direttamente inviato
                                        self.cache.tab1_data.file_exploring_data.set_fields(id, chat_data.selected_contact, chat_data.selected_server, 1);
                                        self.app_mode = AppMode::FileExploring;
                                    }
                                    _ => ()
                                }
                            },
                            _ => (),
                        }
                    }
                }
            }
        }
    }
    fn handle_net_page(&mut self, key: KeyEvent,id:NodeId, net_page:ClientNetPopup){
        match net_page {
            ClientNetPopup::FirstPage(stpage) => {
                // I am outside because this is shared by the two known_chat_servers cases
                match key.code.to_string().to_lowercase().as_str(){
                    "r" => {
                        // Send request
                        match self.simulation_controller_data.request_server_type(id){
                            Ok(_) => {
                                self.simulation_controller_data.logs.push_sc_log(LogType::INFO,"Sent request server-type".to_string());
                            }
                            Err(str) => {
                                self.simulation_controller_data.logs.push_sc_log(LogType::ERROR,str);
                            }
                        }
                    },
                    _ => (),
                }
                match self.simulation_controller_data.net_cache.known_content_servers.clone().get(&id){
                    None => {
                        match key.code {
                            KeyCode::Left => {
                                self.cache.tab1_data.popup_selection = PopupSelection::MenuArea;
                                self.cache.tab1_data.popup_cursor = 1; // Re-selecting Net Menu
                            }
                            _ => ()
                        }
                    },
                    Some(known_content_servers) => {
                        let chat_servers_len = known_content_servers.len();
                        match key.code {
                            KeyCode::Up => {
                                if self.cache.tab1_data.popup_cursor>0{
                                    self.cache.tab1_data.popup_cursor-=1;
                                }
                            }
                            KeyCode::Down => {
                                if self.cache.tab1_data.popup_cursor + 1 < chat_servers_len  {
                                    self.cache.tab1_data.popup_cursor+=1;
                                }
                            }
                            KeyCode::Left => {
                                // Return on Menu
                                self.cache.tab1_data.popup_selection = PopupSelection::MenuArea;
                                self.cache.tab1_data.popup_cursor = 1; // Re-selecting Net Menu
                            }
                            KeyCode::Enter => {
                                // I get the selected item and pass it to the second page struct
                                match known_content_servers.get((self.cache.tab1_data.popup_cursor as usize)){
                                    None => {
                                        // Should be impossible
                                        // THROW FATAL
                                    }
                                    Some(selected_server_id) => {
                                        let server_id = *selected_server_id;
                                        match self.simulation_controller_data.request_files_list(id, server_id){
                                            Ok(_) => {
                                                self.simulation_controller_data.logs.push_sc_log(LogType::DEBUG,"Successfully sent request files list".to_string());
                                            }
                                            Err(str) => {
                                                self.simulation_controller_data.logs.push_sc_log(LogType::ERROR,str);
                                            }
                                        }
                                        match self.simulation_controller_data.request_media_list(id, server_id){
                                            Ok(_) => {
                                                self.simulation_controller_data.logs.push_sc_log(LogType::DEBUG,"Successfully sent request media list".to_string());
                                            }
                                            Err(str) => {
                                                self.simulation_controller_data.logs.push_sc_log(LogType::ERROR,str);
                                            }
                                        }
                                        self.cache.tab1_data.popup_cursor = 0;
                                        self.cache.tab1_data.viewing =
                                            ViewedTab1::Popup((
                                                id,
                                                NodePopup::Client(ClientPopup::InternetPage(
                                                    ClientNetPopup::SecondPage(
                                                        NetSecondPage{
                                                            selected_server: *selected_server_id,
                                                            selected_file: SelectedFileMedia::None,
                                                            image_rendering_data: RenderingImage::False,
                                                            selected_item: Default::default(),
                                                            scroll_offset: 0,
                                                        }
                                                    )
                                                ))
                                            ))
                                    }
                                }
                            }
                            _ => (),
                        }
                    }
                }
            }
            ClientNetPopup::SecondPage(mut ndpage) => {
                match key.code.to_string().to_lowercase().as_str(){
                    "r" => {
                        // Send request
                        let server_id = ndpage.selected_server;
                        match self.simulation_controller_data.request_files_list(id, server_id){
                            Ok(_) => {
                                self.simulation_controller_data.logs.push_sc_log(LogType::DEBUG,"Successfully sent request files list".to_string());
                            }
                            Err(str) => {
                                self.simulation_controller_data.logs.push_sc_log(LogType::ERROR,str);
                            }
                        }
                        match self.simulation_controller_data.request_media_list(id, server_id){
                            Ok(_) => {
                                self.simulation_controller_data.logs.push_sc_log(LogType::DEBUG,"Successfully sent request media list".to_string());
                            }
                            Err(str) => {
                                self.simulation_controller_data.logs.push_sc_log(LogType::ERROR,str);
                            }
                        }
                    },
                    _ => (),
                }
                match key.code{
                    KeyCode::Up => {
                        match ndpage.selected_item{
                            NetSecondPageItems::Blocks => {
                                if self.cache.tab1_data.popup_cursor>0 && self.cache.tab1_data.popup_cursor<4{
                                    self.cache.tab1_data.popup_cursor-=1;
                                }
                            }
                            NetSecondPageItems::FileList => {
                                self.cache.tab1_data.file_list_render_utils.decrease();
                            }
                            NetSecondPageItems::MediaList => {
                                self.cache.tab1_data.media_list_render_utils.decrease();
                            }
                        }
                    }
                    KeyCode::Down => {
                        match ndpage.selected_item{
                            NetSecondPageItems::Blocks => {
                                if self.cache.tab1_data.popup_cursor<3{
                                    self.cache.tab1_data.popup_cursor+=1;
                                }
                            }
                            NetSecondPageItems::FileList => {
                                match self.simulation_controller_data.net_cache.known_file_list.get(&id){
                                    None => (), // Do nothing
                                    Some(hash) => {
                                        match hash.get(&ndpage.selected_server){
                                            None => (), // Do nothing
                                            Some(vecfiles) => {
                                                if (self.cache.tab1_data.file_list_render_utils.selected as usize)  < vecfiles.len() {
                                                    self.cache.tab1_data.file_list_render_utils.increase();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            NetSecondPageItems::MediaList => {
                                match self.simulation_controller_data.net_cache.known_media_list.get(&id){
                                    None => (), // Do nothing
                                    Some(hash) => {
                                        match hash.get(&ndpage.selected_server){
                                            None => (), // Do nothing
                                            Some(vecfiles) => {
                                                if (self.cache.tab1_data.media_list_render_utils.selected as usize)  < vecfiles.len() {
                                                    self.cache.tab1_data.media_list_render_utils.increase();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Left => {
                        match self.cache.tab1_data.popup_cursor{
                            0 | 1 | 2 | 3 => {
                                // Back on selecting menu tab
                                self.cache.tab1_data.popup_selection = PopupSelection::MenuArea;
                                self.cache.tab1_data.popup_cursor = 1;
                            }
                            4 => {
                                self.cache.tab1_data.popup_cursor= 0;
                                ndpage.selected_item = NetSecondPageItems::Blocks;
                                self.cache.tab1_data.viewing = ViewedTab1::Popup((id, NodePopup::Client(
                                    ClientPopup::InternetPage(ClientNetPopup::SecondPage(ndpage))
                                )));
                            }
                            _ => (),
                        }
                    }
                    KeyCode::Right => {
                        if ndpage.selected_file != SelectedFileMedia::None {
                            self.cache.tab1_data.popup_cursor = 4;
                            ndpage.selected_item = NetSecondPageItems::Blocks;
                            self.cache.tab1_data.viewing = ViewedTab1::Popup((id, NodePopup::Client(
                                ClientPopup::InternetPage(ClientNetPopup::SecondPage(ndpage))
                            )));
                        }
                    }
                    KeyCode::Enter => {
                        match ndpage.selected_item {
                            NetSecondPageItems::Blocks => {
                                match self.cache.tab1_data.popup_cursor{
                                    0 => {
                                        // Enter media list only if it is not empty
                                        match self.simulation_controller_data.net_cache.known_media_list.get(&id){
                                            None => (), // Do nothing
                                            Some(hash) => {
                                                match hash.get(&ndpage.selected_server){
                                                    None => (), // Do nothing
                                                    Some(vecfiles) => {
                                                        if !vecfiles.is_empty(){
                                                            ndpage.selected_item = NetSecondPageItems::MediaList;
                                                            self.cache.tab1_data.viewing = ViewedTab1::Popup((id, NodePopup::Client(
                                                                ClientPopup::InternetPage(ClientNetPopup::SecondPage(ndpage))
                                                            )));
                                                        }else{

                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    1 => {
                                        // Upload your media
                                        // Attach a file, il restituito andrà direttamente inviato
                                        // Devo settare i fields in modo corretto
                                        // todo()!
                                        self.cache.tab1_data.file_exploring_data.set_fields(id, 255, ndpage.selected_server , 2);
                                        self.app_mode = AppMode::FileExploring;
                                    }
                                    2 => {
                                        match self.simulation_controller_data.net_cache.known_media_list.get(&id){
                                            None => (), // Do nothing
                                            Some(hash) => {
                                                match hash.get(&ndpage.selected_server){
                                                    None => (), // Do nothing
                                                    Some(vecfiles) => {
                                                        if !vecfiles.is_empty(){
                                                            ndpage.selected_item = NetSecondPageItems::FileList;
                                                            self.cache.tab1_data.viewing = ViewedTab1::Popup((id, NodePopup::Client(
                                                                ClientPopup::InternetPage(ClientNetPopup::SecondPage(ndpage))
                                                            )));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    3 => {
                                        // Upload your file
                                        // Upload your media
                                        // Attach a file, il restituito andrà direttamente inviato
                                        // Devo settare i fields in modo corretto
                                        // todo()!
                                        self.cache.tab1_data.file_exploring_data.set_fields(id, 255, ndpage.selected_server , 3);
                                        self.simulation_controller_data.logs.push_sc_log(LogType::ERROR, format!("{:?}",self.cache.tab1_data.file_exploring_data));
                                        self.app_mode = AppMode::FileExploring;
                                    }
                                    4 => {
                                        // Open the media if it is selected!

                                    }
                                    _ => (),
                                }
                            }
                            NetSecondPageItems::FileList => {

                            }
                            NetSecondPageItems::MediaList => {
                                match self.simulation_controller_data.net_cache.known_media_list.clone().get(&id){
                                    None => (), // Do nothing
                                    Some(hash) => {
                                        match hash.get(&ndpage.selected_server){
                                            None => (), // Do nothing
                                            Some(vecfiles) => {
                                                match vecfiles.get(self.cache.tab1_data.media_list_render_utils.selected as usize){
                                                    None => self.simulation_controller_data.logs.push_sc_log(LogType::ERROR, "Error selecting the media, media-index out of bounds!".to_string()),
                                                    Some(media_id) => {
                                                        // Request the media
                                                        match self.simulation_controller_data.request_media(id, ndpage.selected_server, media_id.clone()){
                                                            Ok(_) => {
                                                                let ret_values = self.simulation_controller_data.net_cache.downloaded_medias.get_mut(&ndpage.selected_server);
                                                                match ret_values{
                                                                    None => {
                                                                        self.simulation_controller_data.net_cache.downloaded_medias.insert(
                                                                            ndpage.selected_server,
                                                                            {
                                                                                let mut ret_hash = HashMap::new();
                                                                                ret_hash.insert(
                                                                                    media_id.clone(),
                                                                                    ((None,None), NetCacheMedia::Downloading(0f32))
                                                                                );
                                                                                ret_hash
                                                                            }
                                                                        );
                                                                    }
                                                                    Some(hash_mut) => {
                                                                        if !hash_mut.contains_key(&media_id.clone()){
                                                                            hash_mut.insert(
                                                                                media_id.clone(),
                                                                                ((None,None), NetCacheMedia::Downloading(0f32))
                                                                            );
                                                                        }
                                                                    }
                                                                }
                                                                ndpage.selected_file = SelectedFileMedia::Media(media_id.clone());
                                                                self.cache.tab1_data.viewing = ViewedTab1::Popup((id, NodePopup::Client(
                                                                    ClientPopup::InternetPage(ClientNetPopup::SecondPage(ndpage))
                                                                )));
                                                            }
                                                            Err(str) => {
                                                                self.simulation_controller_data.logs.push_sc_log(LogType::ERROR,str);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
    }
    fn handle_data_page(&mut self, key: KeyEvent,id:NodeId, net_page: ClientNetPopup){}
    fn handle_sett_page(&mut self, key:KeyEvent,id:NodeId){}



    fn handle_server_popup(&mut self, key: KeyEvent,id:NodeId, server_popup: ServerPopup){}
    fn handle_drone_popup(&mut self, key: KeyEvent,id:NodeId, client_popup: DronePopup){
        match self.cache.tab1_data.popup_selection {
            PopupSelection::MenuArea => {
                match key.code{
                    KeyCode::Up => {
                        if self.cache.tab1_data.popup_cursor > 0{
                            self.cache.tab1_data.popup_cursor -= 1;
                        }else{
                            self.cache.tab1_data.popup_cursor = 1;
                        }
                        // Need same for drone
                        self.coordinate_drone_popup_cursor_with_opt_menu(id, self.cache.tab1_data.popup_cursor);
                    }
                    KeyCode::Down => {
                        if self.cache.tab1_data.popup_cursor < 1{
                            self.cache.tab1_data.popup_cursor += 1;
                        }else{
                            self.cache.tab1_data.popup_cursor = 0;
                        }
                        self.coordinate_drone_popup_cursor_with_opt_menu(id, self.cache.tab1_data.popup_cursor);
                    }
                    KeyCode::Right | KeyCode::Enter => {
                        self.cache.tab1_data.popup_selection = PopupSelection::OptArea;
                        self.coordinate_drone_popup_cursor_with_opt_menu(id, self.cache.tab1_data.popup_cursor);
                        self.cache.tab1_data.popup_cursor = 0;
                    }
                    _ => (),
                }
            }
            PopupSelection::OptArea => {
                match client_popup{
                    DronePopup::DataPage(datapage) => {

                    }
                    DronePopup::SettingsPage(settpage) => {
                        match settpage.command_popup{
                            DroneSettingsCommandsPopup::None => {
                                self.handle_sett_drone_page(key,id,settpage)
                            }
                            DroneSettingsCommandsPopup::SetPDR(commpop) => {
                                match key {
                                    _ => (),
                                }

                            }
                            DroneSettingsCommandsPopup::AddSender(addsend) => {

                            }
                            DroneSettingsCommandsPopup::RemoveSender(remsend) => {

                            }
                            DroneSettingsCommandsPopup::Crash(crapop) => {

                            }
                        }
                    }
                }
            }
        }
    }
    fn handle_data_drone_page(&mut self, key:KeyEvent,id:NodeId, data_page: DroneDataPopup){

    }
    fn handle_sett_drone_page(&mut self, key:KeyEvent, id:NodeId, mut sett_page:DroneSettingsPopup){
        match sett_page.selected_item{
            DroneSettingsItems::Blocks => {
                match key.code{
                    KeyCode::Right => {
                        if self.cache.tab1_data.popup_cursor < 1{
                            self.cache.tab1_data.popup_cursor += 1;
                        }
                    }
                    KeyCode::Left => {
                        if self.cache.tab1_data.popup_cursor > 0{
                            self.cache.tab1_data.popup_cursor -= 1;
                        }
                    }
                    KeyCode::Enter => {
                        match self.cache.tab1_data.popup_cursor{
                            0 => {
                                sett_page.selected_item=DroneSettingsItems::Commands;
                            }
                            1 => {
                                // Recompute the map?
                            }
                            _ => (),
                        }
                    }
                    _ => (),
                }
            }
            DroneSettingsItems::Commands => {
                match key.code {
                    KeyCode::Char('i') | KeyCode::Char('I') => {
                        // todo!()
                    }
                    KeyCode::Up => {
                        sett_page.command_up();
                    },
                    KeyCode::Down => {
                        sett_page.command_down();
                    }
                    KeyCode::Enter => {
                        // Which command I've selected?
                        match sett_page.command_offset {
                            0 => {
                                // SetPDR
                                sett_page.command_popup = DroneSettingsCommandsPopup::SetPDR(CommandPopup {
                                    status: SettingsPopupStatus::Idle,
                                    cursor: 0,
                                });
                            }
                            1 => {
                                // Add Sender
                                sett_page.command_popup = DroneSettingsCommandsPopup::AddSender(CommandPopup {
                                    status: SettingsPopupStatus::Idle,
                                    cursor: 0,
                                });
                            }
                            2 => {
                                // Remove Sender
                                sett_page.command_popup = DroneSettingsCommandsPopup::RemoveSender(CommandPopup {
                                    status: SettingsPopupStatus::Idle,
                                    cursor: 0,
                                });
                            }
                            3 => {
                                // Crash
                                sett_page.command_popup = DroneSettingsCommandsPopup::Crash(CommandPopup {
                                    status: SettingsPopupStatus::Idle,
                                    cursor: 0,
                                });
                            }
                            4 => {
                                // Recompute Topology Map
                                self.initialize_node_pop_map(id);
                            }
                            _ => (),
                        }
                    }
                    _ => (),
                }
            }
        }
    self.cache.tab1_data.viewing =
    ViewedTab1::Popup((
                id,
                NodePopup::Drone(
                    DronePopup::SettingsPage(sett_page)
                )
            ))
    }

}



