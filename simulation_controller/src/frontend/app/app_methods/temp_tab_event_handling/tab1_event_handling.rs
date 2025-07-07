use crate::backend::sc_models::MediaEntry;
use std::cmp::PartialEq;
use std::collections::HashMap;
use std::ops::DerefMut;
use std::sync::LockResult;
use dashmap::mapref::one::Ref;
use log::{error, info, warn, debug};
use host::{ClientId, FileId, MediaId, ServerId};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use wg_2024::network::NodeId;
use server::chat::Chat;
use crate::backend::sc_models::{ChatList, LogType, NetCacheFile, NetCacheMedia};
use crate::frontend::app::App;
use crate::frontend::app::app_methods::method_utils::open_file;
use crate::frontend::app::models::general_types::Node;
use crate::frontend::app::models::main_types::{AppMode, InputMode};
use crate::frontend::app::models::main_types::InputMode::Two;
use crate::frontend::app::models::tab1_types::{ChFirstPage, ChSecondPage, ChThirdPage, ChatThirdPageSelection, ClientChatPopup, ClientDataPopup, ClientNetPopup, ClientPopup, ClientSettingsCommandsPopup, ClientSettingsPopup, CommandPopup, DroneDataPopup, DronePopup, DroneSettingsCommandsPopup, DroneSettingsPopup, NetFirstPage, NetSecondPage, NetSecondPageItems, NodePopup, PopupSelection, RenderingImage, SelectedFileMedia, ServerDataPopup, ServerPopup, ServerSettingsCommandsPopup, ServerSettingsPopup, SettingsPopupStatus, ViewedTab1};
use crate::frontend::app::topology_algorithm::alg_models::TopologyAlgorithm;

impl App<'_> {
    
    pub(crate) fn handle_event_tab1(&mut self, key: KeyEvent) {
        // Handle backspace globally for tab1
        if key.code == KeyCode::Backspace {
            self.handle_backspace_tab1();
            return;
        }

        match self.cache.tab1_data.viewing.clone(){
            ViewedTab1::Main => {
                self.handle_main(key)
            }
            ViewedTab1::Popup((id,node_popup)) => {
                self.handle_popup(key,id,node_popup);
            }
        }
    }

    /// Handle backspace key according to navigation rules
    fn handle_backspace_tab1(&mut self) {
        match self.cache.tab1_data.viewing.clone() {
            ViewedTab1::Main => {
                // If we're at the main level, backspace should do nothing
                // (or could be used for other purposes like clearing selection)
                return;
            }
            ViewedTab1::Popup((id, node_popup)) => {
                // Check if we're inside a block (file list, media list, etc.)
                if self.is_inside_block(&node_popup) {
                    // Go back to navigate through blocks
                    self.go_back_to_block_navigation(&node_popup);
                } else {
                    // Go back one page in navigation history
                    self.go_back_one_page();
                }
            }
        }
    }

    /// Check if the user is currently inside a block (file list, media list, etc.)
    fn is_inside_block(&self, node_popup: &NodePopup) -> bool {
        match node_popup {
            NodePopup::Client(ClientPopup::ChatPage(ClientChatPopup::ThirdPage(_))) => {
                // Inside chat messages or bottom tab
                true
            }
            NodePopup::Client(ClientPopup::InternetPage(ClientNetPopup::SecondPage(second_page))) => {
                // Inside file list or media list
                matches!(second_page.selected_item, NetSecondPageItems::FileList | NetSecondPageItems::MediaList)
            }
            NodePopup::Client(ClientPopup::SettingsPage(settings)) => {
                // Inside a command popup
                !matches!(settings.command_popup, ClientSettingsCommandsPopup::None)
            }
            NodePopup::Server(ServerPopup::SettingsPage(settings)) => {
                // Inside a command popup
                !matches!(settings.command_popup, ServerSettingsCommandsPopup::None)
            }
            NodePopup::Drone(DronePopup::SettingsPage(settings)) => {
                // Inside a command popup
                !matches!(settings.command_popup, DroneSettingsCommandsPopup::None)
            }
            _ => false
        }
    }

    /// Go back to block navigation (exit from file/media lists, command popups, etc.)
    fn go_back_to_block_navigation(&mut self, node_popup: &NodePopup) {
        match node_popup {
            NodePopup::Client(ClientPopup::ChatPage(ClientChatPopup::ThirdPage(third_page))) => {
                // Go back from chat third page to second page
                if let Some(id) = self.get_current_popup_id() {
                    self.cache.tab1_data.viewing = ViewedTab1::Popup((
                        id,
                        NodePopup::Client(ClientPopup::ChatPage(ClientChatPopup::SecondPage(
                            ChSecondPage {
                                selected_server: third_page.selected_server,
                            }
                        )))
                    ));
                }
            }
            NodePopup::Client(ClientPopup::InternetPage(ClientNetPopup::SecondPage(second_page))) => {
                // Go back from file/media list to blocks
                if let Some(id) = self.get_current_popup_id() {
                    let mut new_second_page = second_page.clone();
                    new_second_page.selected_item = NetSecondPageItems::Blocks;
                    self.cache.tab1_data.viewing = ViewedTab1::Popup((
                        id,
                        NodePopup::Client(ClientPopup::InternetPage(ClientNetPopup::SecondPage(new_second_page)))
                    ));
                }
            }
            NodePopup::Client(ClientPopup::SettingsPage(settings)) => {
                // Exit command popup
                if let Some(id) = self.get_current_popup_id() {
                    let mut new_settings = settings.clone();
                    new_settings.command_popup = ClientSettingsCommandsPopup::None;
                    self.cache.tab1_data.viewing = ViewedTab1::Popup((
                        id,
                        NodePopup::Client(ClientPopup::SettingsPage(new_settings))
                    ));
                }
            }
            NodePopup::Server(ServerPopup::SettingsPage(settings)) => {
                // Exit command popup
                if let Some(id) = self.get_current_popup_id() {
                    let mut new_settings = settings.clone();
                    new_settings.command_popup = ServerSettingsCommandsPopup::None;
                    self.cache.tab1_data.viewing = ViewedTab1::Popup((
                        id,
                        NodePopup::Server(ServerPopup::SettingsPage(new_settings))
                    ));
                }
            }
            NodePopup::Drone(DronePopup::SettingsPage(settings)) => {
                // Exit command popup
                if let Some(id) = self.get_current_popup_id() {
                    let mut new_settings = settings.clone();
                    new_settings.command_popup = DroneSettingsCommandsPopup::None;
                    self.cache.tab1_data.viewing = ViewedTab1::Popup((
                        id,
                        NodePopup::Drone(DronePopup::SettingsPage(new_settings))
                    ));
                }
            }
            _ => {
                // Default: go back one page
                self.go_back_one_page();
            }
        }
    }

    /// Go back one page in navigation history
    fn go_back_one_page(&mut self) {
        if let Some(previous_state) = self.cache.tab1_data.pop_navigation_state() {
            self.cache.tab1_data.viewing = previous_state;
        } else {
            // If no history, go back to main
            self.cache.tab1_data.return_to_main();
        }
    }

    /// Get the current popup ID if we're in a popup
    fn get_current_popup_id(&self) -> Option<NodeId> {
        if let ViewedTab1::Popup((id, _)) = &self.cache.tab1_data.viewing {
            Some(*id)
        } else {
            None
        }
    }

    /// Save current state to navigation history before navigating
    fn save_navigation_state(&mut self) {
        self.cache.tab1_data.push_navigation_state(self.cache.tab1_data.viewing.clone());
    }

    fn handle_main(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => {
                self.cache.tab1_data.net_node_list.select_prev();
            }
            KeyCode::Down => {
                self.cache.tab1_data.net_node_list.select_next();
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
                        let selected_cursor_item = self.cache.tab1_data.net_node_list.selected as usize;
                        let node_info = self.cache.tab1_data.net_node_list.items.get(selected_cursor_item);
                        
                        match node_info {
                            None => {
                                // Error: selected item doesn't exist
                                error!(target: "main", "[LEDRON-CLI] Selected item not found in network list");
                            }
                            Some((node,is_active,_)) => {
                                if *is_active{
                                    // Clone the node data to avoid borrowing conflicts
                                    let node_clone = node.clone();
                                    
                                    // Save current state before navigating
                                    self.save_navigation_state();
                                    
                                    match node_clone{
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
                                                    NodePopup::Server(ServerPopup::DataPage(
                                                        ServerDataPopup::FirstPage()
                                                    ))
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
                                    error!(target: "main", "[LEDRON-CLI] Selected node is not active");
                                }
                            }
                        }
                    }
                    1 => {
                        // I am on Map
                        // I still don't know what could happen when I press map
                        info!(target: "main", "[LEDRON-CLI] Map functionality not yet implemented");
                    }
                    _ => ()
                }
            }
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
                            ClientSettingsPopup{
                                command_offset: 0,
                                command_popup: Default::default(),
                            }
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
                let mut pdr:f32 = 404f32;
                for dr in &self.simulation_controller_data.drones{
                    if dr.id == id {
                        pdr = dr.pdr;
                    }
                }
                self.cache.tab1_data.viewing =
                    ViewedTab1::Popup((
                        id,
                        NodePopup::Drone(
                            DronePopup::SettingsPage(
                                DroneSettingsPopup{ command_offset: 0,
                                    drone_pdr: pdr,
                                    command_popup: Default::default()
                                })
                            )
                        )
                    );
            }
            _ => {
                self.cache.tab1_data.popup_cursor = 0;
            }
        }
    }
    fn coordinate_server_popup_cursor_with_opt_menu(&mut self, id:NodeId, current_cursor: usize){
        match current_cursor{
            0 => {
                self.cache.tab1_data.viewing =
                    ViewedTab1::Popup((
                        id,
                        NodePopup::Server(
                            ServerPopup::DataPage(
                                ServerDataPopup::FirstPage()
                            )
                        )
                    ));
            }
            1 => {
                self.cache.tab1_data.viewing =
                    ViewedTab1::Popup((
                        id,
                        NodePopup::Server(
                            ServerPopup::SettingsPage(
                                ServerSettingsPopup{
                                    command_offset: 0,
                                    command_popup: Default::default()
                                })
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
                match key.code {
                    KeyCode::Esc => {
                        self.cache.tab1_data.popup_selection = PopupSelection::MenuArea;
                        self.cache.tab1_data.popup_cursor = 0;
                    }
                    _ => {
                        match client_popup {
                            ClientPopup::ChatPage(ch_page) => {
                                self.handle_chat_page(key,id,ch_page)
                            }
                            ClientPopup::InternetPage(net_page) => {
                                self.handle_net_client_page(key, id, net_page)
                            }
                            ClientPopup::DataPage(data_page) => {
                                self.handle_data_client_page(key, id, data_page)
                            }
                            ClientPopup::SettingsPage(sett_page) => {
                                match sett_page.command_popup.clone(){
                                    ClientSettingsCommandsPopup::None => {
                                        self.handle_sett_client_page(key, id, sett_page)
                                    }
                                    ClientSettingsCommandsPopup::AddSender(cmd) => {
                                        self.handle_client_add_sender_popup(key, id, sett_page, cmd);
                                    }
                                    ClientSettingsCommandsPopup::RemoveSender(cmd) => {
                                        self.handle_client_remove_sender_popup(key, id, sett_page, cmd);
                                    }
                                }
                            }
                        }
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
                                info!(target: "main", "[LEDRON-CLI] Trying to find new servers in the network!");
                            }
                            Err(str) => {
                                error!(target: "main", "[LEDRON-CLI] {}", str);
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
                                let mut request:bool = false;
                                let mut opt_server_id: ServerId = 0;
                                match known_chat_servers.get((self.cache.tab1_data.popup_cursor as usize)){
                                    None => {
                                        // Should be impossible
                                        // THROW FATAL
                                        error!(target: "main", "[LEDRON-CLI] Selected chat server not found");
                                    }
                                    Some(selected_server_id) => {
                                        request=true;
                                        opt_server_id = selected_server_id.clone();
                                        let server_id = *selected_server_id; // Clone the server ID
                                        self.cache.tab1_data.popup_cursor = 0;
                                        
                                        // Save current state before navigating
                                        self.save_navigation_state();
                                        
                                        self.cache.tab1_data.viewing =
                                            ViewedTab1::Popup((
                                                id,
                                                NodePopup::Client(ClientPopup::ChatPage(
                                                    ClientChatPopup::SecondPage(
                                                        ChSecondPage {
                                                            selected_server: server_id,
                                                        }
                                                    )
                                                ))
                                            ))
                                    }
                                }
                                if request{
                                    match self.simulation_controller_data.request_registration_to_chat(id,opt_server_id) {
                                        Ok(_) => (),
                                        Err(str) => {
                                            error!(target:"main", "[LEDRON-CLI] Couldn't registrate to chat:\n{:?}",str);
                                        }
                                    }
                                }

                            }
                            _ => (),
                        }
                    }
                }
            }
            ClientChatPopup::SecondPage(contacts_data) => {
                match key.code {
                    KeyCode::Esc => {
                        // Go back to first page
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
                    _ => {
                        match key.code.to_string().to_lowercase().as_str(){
                            "r" => {
                                // Send request
                                match self.simulation_controller_data.request_client_list(id,contacts_data.selected_server){
                                    Ok(_) => (),
                                    Err(str) => {
                                        error!(target: "main", "[LEDRON-CLI] {}", str);
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
                                                        error!(target: "main", "[LEDRON-CLI] Selected contact not found");
                                                    }
                                                    Some(contact) => {
                                                        self.cache.tab1_data.popup_cursor = 0;
                                                        let contact_id = *contact; // Clone the contact ID
                                                        
                                                        // Save current state before navigating
                                                        self.save_navigation_state();
                                                        
                                                        self.cache.tab1_data.viewing =
                                                            ViewedTab1::Popup((
                                                                id,
                                                                NodePopup::Client(ClientPopup::ChatPage(
                                                                    ClientChatPopup::ThirdPage(
                                                                        ChThirdPage {
                                                                            scroll_offset: 0,
                                                                            selected_item: Default::default(),
                                                                            selected_server: contacts_data.selected_server,
                                                                            selected_contact: contact_id,
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
                }
            }
            ClientChatPopup::ThirdPage(chat_data) => {
                match key.code {
                    KeyCode::Esc => {
                        // Go back to second page
                        self.cache.tab1_data.viewing =
                            ViewedTab1::Popup((
                                id,
                                NodePopup::Client(ClientPopup::ChatPage(
                                    ClientChatPopup::SecondPage(
                                        ChSecondPage {
                                            selected_server: chat_data.selected_server,
                                        }
                                    )
                                ))
                            ))
                    }
                    _ => {
                        self.handle_chat_third_page(key, id, chat_data);
                    }
                }
            }
        }
    }
    fn handle_chat_third_page(&mut self, key: KeyEvent, id: NodeId, mut third_page_data: ChThirdPage) {
        // Get the chat length to set proper boundaries
        let chat_length = match self.simulation_controller_data.chat_cache.local_chats.get(&third_page_data.selected_server) {
            Some(chat_list) => {
                match chat_list.chats_stored.get(&(id, third_page_data.selected_contact)) {
                    Some(chat) => match chat.read() {
                        Ok(ch) => ch.chat.len(),
                        Err(_) => 0
                    }
                    None => {
                        match chat_list.chats_stored.get(&(third_page_data.selected_contact, id)) {
                            Some(chat) => match chat.read() {
                                Ok(ch) => ch.chat.len(),
                                Err(_) => 0
                            },
                            None => 0
                        }
                    }
                }
            }
            None => 0
        };

        match third_page_data.selected_item {
            ChatThirdPageSelection::ScrollingMessages => {
                match key.code {
                    KeyCode::Up => {
                        // Increase scroll offset to go to older messages
                        // Don't exceed chat length - 1 (since we start from 0)
                        if (third_page_data.scroll_offset as usize ) < chat_length.saturating_sub(1) {
                            third_page_data.scroll_offset += 1;
                        }
                    }
                    KeyCode::Down => {
                        // Decrease scroll offset to go to newer messages
                        if third_page_data.scroll_offset > 0 {
                            third_page_data.scroll_offset -= 1;
                        } else {
                            // If we're at 0, switch to BottomTab selection
                            third_page_data.selected_item = ChatThirdPageSelection::BottomTab;
                            third_page_data.scroll_offset = 0; // Start with text input
                        }
                    }
                    KeyCode::Enter => {
                        // Get the currently selected message
                        let selected_message_index = if third_page_data.scroll_offset == 0 {
                            chat_length.saturating_sub(1) // Last message (most recent)
                        } else {
                            chat_length.saturating_sub(third_page_data.scroll_offset as usize + 1)
                        };

                        // Get the chat to access the selected message
                        if let Some(chat_list) = self.simulation_controller_data.chat_cache.local_chats.get(&third_page_data.selected_server) {
                            let chat_key = (id, third_page_data.selected_contact);
                            let alt_chat_key = (third_page_data.selected_contact, id);
                            
                            let chat_arc = chat_list.chats_stored.get(&chat_key)
                                .or_else(|| chat_list.chats_stored.get(&alt_chat_key));
                            
                            if let Some(chat_arc) = chat_arc {
                                if let Ok(chat) = chat_arc.read() {
                                    if let Some((message, _)) = chat.chat.get(selected_message_index) {
                                        // Check if the message is a MediaSC
                                        if let server::MessageSC::Media(media_sc) = message {
                                            match media_sc {
                                                server::MediaSC::Image(media_id, _) => {
                                                    // Check if this media is in downloaded_chat_files
                                                    if let Some(file_path) = self.simulation_controller_data.downloaded_chat_files.get(media_id) {
                                                        // Open the file using the open_file function
                                                        let path_str = file_path.to_string_lossy();
                                                        crate::frontend::app::app_methods::method_utils::open_file(&path_str);
                                                        info!(target: "main", "[LEDRON-CLI] Opening media file: {}", path_str);
                                                    } else {
                                                        warn!(target: "main", "[LEDRON-CLI] Media file {} not found in downloaded files", media_id);
                                                    }
                                                }
                                                server::MediaSC::File(file_id, _) => {
                                                    // Check if this file is in downloaded_chat_files
                                                    if let Some(file_path) = self.simulation_controller_data.downloaded_chat_files.get(file_id) {
                                                        // Open the file using the open_file function
                                                        let path_str = file_path.to_string_lossy();
                                                        crate::frontend::app::app_methods::method_utils::open_file(&path_str);
                                                        info!(target: "main", "[LEDRON-CLI] Opening file: {}", path_str);
                                                    } else {
                                                        warn!(target: "main", "[LEDRON-CLI] File {} not found in downloaded files", file_id);
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
            ChatThirdPageSelection::BottomTab => {
                match key.code {
                    KeyCode::Up => {
                        third_page_data.scroll_offset=0;
                        third_page_data.selected_item = ChatThirdPageSelection::ScrollingMessages;
                    }
                    KeyCode::Left => {
                        if third_page_data.scroll_offset > 0 { // 0: text input, 1: attach file
                            third_page_data.scroll_offset -= 1;
                        }
                    }
                    KeyCode::Right => {
                        if third_page_data.scroll_offset < 1 { // 0: text input, 1: attach file
                            third_page_data.scroll_offset += 1;
                        }
                    }
                    KeyCode::Enter => {
                        match third_page_data.scroll_offset {
                            0 => {
                                // Text input mode - switch to message input
                                self.app_mode = AppMode::Input(InputMode::Two(id, third_page_data.clone())); // Use mode 2 for chat message input
                            }
                            1 => {
                                // Attach file mode
                                self.cache.tab1_data.file_exploring_data.set_fields(id, third_page_data.selected_contact, third_page_data.selected_server, 1);
                                self.app_mode = AppMode::FileExploring;
                            }
                            _ => (),
                        }
                    }
                    _ => (),
                }
            }
        }

        // Update the viewing state with the modified third page data
        self.cache.tab1_data.viewing = ViewedTab1::Popup((
            id,
            NodePopup::Client(ClientPopup::ChatPage(ClientChatPopup::ThirdPage(third_page_data)))
        ));
    }

    fn handle_net_client_page(&mut self, key: KeyEvent, id:NodeId, net_page:ClientNetPopup){
        match net_page {
            ClientNetPopup::FirstPage(stpage) => {
                // I am outside because this is shared by the two known_chat_servers cases
                match key.code.to_string().to_lowercase().as_str(){
                    "r" => {
                        // Send request
                        match self.simulation_controller_data.request_server_type(id){
                            Ok(_) => {
                                info!(target: "main", "[LEDRON-CLI] Sent request server-type");
                            }
                            Err(str) => {
                                error!(target: "main", "[LEDRON-CLI] {}", str);
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
                            KeyCode::Left  => {
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
                                        error!(target: "main", "[LEDRON-CLI] Selected content server not found");
                                    }
                                    Some(selected_server_id) => {
                                        let server_id = *selected_server_id;
                                        match self.simulation_controller_data.request_files_list(id, server_id){
                                            Ok(_) => {
                                                debug!(target: "main", "[LEDRON-CLI] Successfully sent request files list");
                                            }
                                            Err(str) => {
                                                error!(target: "main", "[LEDRON-CLI] {}", str);
                                            }
                                        }
                                        match self.simulation_controller_data.request_media_list(id, server_id){
                                            Ok(_) => {
                                                debug!(target: "main", "[LEDRON-CLI] Successfully sent request media list");
                                            }
                                            Err(str) => {
                                                error!(target: "main", "[LEDRON-CLI] {}", str);
                                            }
                                        }
                                        self.cache.tab1_data.popup_cursor = 0;
                                        
                                        // Save current state before navigating
                                        self.save_navigation_state();
                                        
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
                                        debug!(target: "main", "[LEDRON-CLI] Successfully sent request files list");
                                    }
                                    Err(str) => {
                                        error!(target: "main", "[LEDRON-CLI] {}", str);
                                    }
                                }
                                match self.simulation_controller_data.request_media_list(id, server_id){
                                    Ok(_) => {
                                        debug!(target: "main", "[LEDRON-CLI] Successfully sent request media list");
                                    }
                                    Err(str) => {
                                        error!(target: "main", "[LEDRON-CLI] {}", str);
                                    }
                                }
                            },
                            _ => (),
                        }
                        match key.code{
                            KeyCode::Backspace => {
                                match ndpage.selected_item{
                                    NetSecondPageItems::Blocks => {
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
                                    NetSecondPageItems::FileList => {
                                        self.cache.tab1_data.popup_cursor = 2;
                                        ndpage.selected_item = NetSecondPageItems::Blocks;
                                        self.cache.tab1_data.viewing = ViewedTab1::Popup((id, NodePopup::Client(
                                            ClientPopup::InternetPage(ClientNetPopup::SecondPage(ndpage))
                                        )));
                                    }
                                    NetSecondPageItems::MediaList => {
                                        self.cache.tab1_data.popup_cursor = 0;
                                        ndpage.selected_item = NetSecondPageItems::Blocks;
                                        self.cache.tab1_data.viewing = ViewedTab1::Popup((id, NodePopup::Client(
                                            ClientPopup::InternetPage(ClientNetPopup::SecondPage(ndpage))
                                        )));
                                    }
                                }
                            }
                            KeyCode::Up => {
                                match ndpage.selected_item{
                                    NetSecondPageItems::Blocks => {
                                        if self.cache.tab1_data.popup_cursor>0 && self.cache.tab1_data.popup_cursor<4{
                                            self.cache.tab1_data.popup_cursor-=1;
                                        }else{
                                            if ndpage.scroll_offset>0{
                                                ndpage.scroll_offset-=1;
                                                ndpage.selected_item = NetSecondPageItems::Blocks;
                                                self.cache.tab1_data.viewing = ViewedTab1::Popup((id, NodePopup::Client(
                                                    ClientPopup::InternetPage(ClientNetPopup::SecondPage(ndpage))
                                                )));
                                            }
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
                                        }else{
                                            ndpage.scroll_offset+=1;
                                            ndpage.selected_item = NetSecondPageItems::Blocks;
                                            self.cache.tab1_data.viewing = ViewedTab1::Popup((id, NodePopup::Client(
                                                ClientPopup::InternetPage(ClientNetPopup::SecondPage(ndpage))
                                            )));
                                        }
                                    }
                                    NetSecondPageItems::FileList => {
                                        match self.simulation_controller_data.net_cache.known_file_list.get(&id){
                                            None => (), // Do nothing
                                            Some(hash) => {
                                                match hash.get(&ndpage.selected_server){
                                                    None => (), // Do nothing
                                                    Some(vecfiles) => {
                                                        // todo()!
                                                        if (self.cache.tab1_data.file_list_render_utils.selected as usize) + 1 < vecfiles.len(){
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
                                                        if (self.cache.tab1_data.media_list_render_utils.selected as usize) + 1  < vecfiles.len() {
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
                                                self.cache.tab1_data.image_rendering_data = RenderingImage::False;
                                                self.cache.tab1_data.file_exploring_data.set_fields(id, 255, ndpage.selected_server , 2);
                                                self.app_mode = AppMode::FileExploring;
                                                ndpage.selected_file = SelectedFileMedia::None;
                                                ndpage.image_rendering_data = RenderingImage::False;
                                                self.cache.tab1_data.viewing = ViewedTab1::Popup((id, NodePopup::Client(
                                                    ClientPopup::InternetPage(ClientNetPopup::SecondPage(ndpage))
                                                )));
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
                                                ndpage.selected_file = SelectedFileMedia::None;
                                                self.cache.tab1_data.image_rendering_data = RenderingImage::False;
                                                ndpage.image_rendering_data = RenderingImage::False;

                                                self.cache.tab1_data.file_exploring_data.set_fields(id, 255, ndpage.selected_server , 3);
                                                self.app_mode = AppMode::FileExploring;
                                                self.cache.tab1_data.viewing = ViewedTab1::Popup((id, NodePopup::Client(
                                                    ClientPopup::InternetPage(ClientNetPopup::SecondPage(ndpage))
                                                )));
                                            }
                                            4 => {
                                                match ndpage.selected_file{
                                                    SelectedFileMedia::None => {}
                                                    SelectedFileMedia::File(cy) => {
                                                        match self.simulation_controller_data.download_folder.join(cy).to_str(){
                                                            None => (),
                                                            Some(str) => {
                                                                open_file(str);
                                                            }
                                                        }
                                                    }
                                                    SelectedFileMedia::Media(cy) => {
                                                        match self.simulation_controller_data.download_folder.join(cy).to_str(){
                                                            None => (),
                                                            Some(str) => {
                                                                open_file(str);

                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            _ => (),
                                        }
                                    }
                                    NetSecondPageItems::FileList => {
                                        match self.simulation_controller_data.net_cache.known_file_list.clone().get(&id){
                                            None => (), // Do nothing
                                            Some(hash) => {
                                                match hash.get(&ndpage.selected_server){
                                                    None => (), // Do nothing
                                                    Some(vecfiles) => {
                                                        match vecfiles.get(self.cache.tab1_data.file_list_render_utils.selected as usize){
                                                            None => error!(target: "main", "[LEDRON-CLI] Error selecting the file, file-index out of bounds!"),
                                                            Some(file_id) => {
                                                                // Request the file
                                                                if !self.simulation_controller_data.net_cache.downloaded_files.contains_key(file_id){
                                                                    match self.simulation_controller_data.request_file(id, ndpage.selected_server, file_id.clone()){
                                                                        Ok(_) => {
                                                                            self.simulation_controller_data.net_cache.downloaded_files.insert(
                                                                                file_id.clone(), NetCacheFile::Downloading(0f32)
                                                                            );
                                                                            ndpage.selected_file = SelectedFileMedia::File(file_id.clone());
                                                                            self.cache.tab1_data.image_rendering_data = RenderingImage::False;
                                                                            ndpage.image_rendering_data = RenderingImage::False;
                                                                            self.cache.tab1_data.viewing = ViewedTab1::Popup((id, NodePopup::Client(
                                                                                ClientPopup::InternetPage(ClientNetPopup::SecondPage(ndpage))
                                                                            )));
                                                                        }
                                                                        Err(str) => {
                                                                            error!(target:"main", "[LEDRON-CLI] Error sending request file to client:\n{}",str);
                                                                        }
                                                                    }
                                                                }else{
                                                                    self.cache.tab1_data.image_rendering_data = RenderingImage::False;
                                                                    ndpage.selected_file = SelectedFileMedia::File(file_id.clone());
                                                                    ndpage.image_rendering_data = RenderingImage::False;
                                                                    self.cache.tab1_data.viewing = ViewedTab1::Popup((id, NodePopup::Client(
                                                                        ClientPopup::InternetPage(ClientNetPopup::SecondPage(ndpage))
                                                                    )));
                                                                }

                                                                //}
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    NetSecondPageItems::MediaList => {
                                        let opt = match self.simulation_controller_data.net_cache.known_media_list.get(&id){
                                            None => None, // Do nothing
                                            Some(hash) => {
                                                match hash.get(&ndpage.selected_server){
                                                    None => None, // Do nothing
                                                    Some(vecfiles) => {
                                                        vecfiles.get(self.cache.tab1_data.media_list_render_utils.selected as usize).cloned()
                                                    }
                                                }
                                            }
                                        };
                                        match opt{
                                                None => error!(target:"main","Error selecting the media, media-index out of bounds!"),
                                                Some(media_id) => {
                                                    // Request the media
                                                    if !self.simulation_controller_data.net_cache.downloaded_medias.contains_key(&media_id){
                                                        match self.simulation_controller_data.request_media(id, ndpage.selected_server, media_id.clone()){
                                                            Ok(_) => {
                                                                // Create or update the media entry in new_downloaded_medias
                                                                let media_entry = MediaEntry::new(NetCacheMedia::Downloading(0f32));
                                                                self.simulation_controller_data.net_cache.downloaded_medias.insert(media_id.clone(), media_entry);
                                                                ndpage.selected_file = SelectedFileMedia::Media(media_id.clone());
                                                                self.cache.tab1_data.viewing = ViewedTab1::Popup((id, NodePopup::Client(
                                                                    ClientPopup::InternetPage(ClientNetPopup::SecondPage(ndpage))
                                                                )));
                                                            }
                                                            Err(str) => {
                                                                error!(target:"main", "[LEDRON-CLI] Error sending request media to client:\n{}",str);
                                                            }
                                                    }
                                                    }else{
                                                        ndpage.selected_file = SelectedFileMedia::Media(media_id.clone());
                                                        self.cache.tab1_data.viewing = ViewedTab1::Popup((id, NodePopup::Client(
                                                            ClientPopup::InternetPage(ClientNetPopup::SecondPage(ndpage))
                                                        )));
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
    fn handle_data_client_page(&mut self, key: KeyEvent, id:NodeId, data_page: ClientDataPopup){
        match key.code {
            KeyCode::Up => {
                if self.cache.tab1_data.popup_cursor == 3 {
                    // From log box, go to rightmost upper box (data_3)
                    self.cache.tab1_data.popup_cursor = 2;
                } else if self.cache.tab1_data.popup_cursor > 0 {
                    self.cache.tab1_data.popup_cursor -= 1;
                } else {
                    // Wrap around to the log box
                    self.cache.tab1_data.popup_cursor = 3;
                }
            }
            KeyCode::Down => {
                if self.cache.tab1_data.popup_cursor < 3 {
                    // From any upper box (0, 1, 2) go directly to log box (3)
                    self.cache.tab1_data.popup_cursor = 3;
                } else {
                    // From log box, wrap around to first element
                    self.cache.tab1_data.popup_cursor = 0;
                }
            }
            KeyCode::Left => {
                // Move left if possible within the horizontal layout (data_1, data_2, data_3)
                if self.cache.tab1_data.popup_cursor == 1 {
                    self.cache.tab1_data.popup_cursor = 0; // From data_2 to data_1
                } else if self.cache.tab1_data.popup_cursor == 2 {
                    self.cache.tab1_data.popup_cursor = 1; // From data_3 to data_2
                } else if self.cache.tab1_data.popup_cursor == 0 {
                    // At leftmost position in top row, go back to menu selection
                    self.cache.tab1_data.popup_selection = PopupSelection::MenuArea;
                    self.cache.tab1_data.popup_cursor = 2; // Set to Data menu item
                } else {
                    // From filtered logging (bottom area), go back to menu area
                    self.cache.tab1_data.popup_selection = PopupSelection::MenuArea;
                    self.cache.tab1_data.popup_cursor = 2;
                }
            }
            KeyCode::Right => {
                // Move right if possible within the horizontal layout
                if self.cache.tab1_data.popup_cursor == 0 {
                    self.cache.tab1_data.popup_cursor = 1; // From data_1 to data_2
                } else if self.cache.tab1_data.popup_cursor == 1 {
                    self.cache.tab1_data.popup_cursor = 2; // From data_2 to data_3
                }
                // If at rightmost or in filtered logging, no right movement
            }
            KeyCode::Enter => {
                // Enter key behavior in log field - enter scrolling mode like tab0
                if self.cache.tab1_data.popup_cursor == 3 {
                    // Enter scrolling mode for the log field (same as tab0)
                    self.app_mode = AppMode::Scrolling;
                    info!(target: "main", "[LEDRON-CLI] Entered log scroll mode for node {}", id);
                }
                // For data charts (0, 1, 2), Enter could open detailed views
                else {
                    info!(target: "main", "[LEDRON-CLI] Data chart {} selected for node {}",
                          self.cache.tab1_data.popup_cursor, id);
                }
            }
            KeyCode::Esc => {
                // ESC closes the popup completely and returns to main
                self.cache.tab1_data.return_to_main();
            }
            _ => {
                // Other keys are ignored in data page
            }
        }

        // Update the viewing state to maintain the data page
        self.cache.tab1_data.viewing = ViewedTab1::Popup((
            id,
            NodePopup::Client(ClientPopup::DataPage(data_page))
        ));
    }



    fn handle_sett_client_page(&mut self, key:KeyEvent, id:NodeId, mut sett_page:ClientSettingsPopup){
        match self.cache.tab1_data.popup_cursor{
            0 => {
                // Commands
                match key.code{
                    KeyCode::Up => {
                        sett_page.command_up();
                    },
                    KeyCode::Down => {
                        sett_page.command_down();
                    }
                    KeyCode::Right => {
                        if self.cache.tab1_data.popup_cursor < 1{
                            self.cache.tab1_data.popup_cursor += 1;
                        }
                    }
                    KeyCode::Left => {
                        if self.cache.tab1_data.popup_cursor > 0{
                            self.cache.tab1_data.popup_cursor -= 1;
                        }else{
                            self.cache.tab1_data.popup_selection = PopupSelection::MenuArea;
                            self.cache.tab1_data.popup_cursor = 1;
                        }
                    }
                    KeyCode::Enter => {
                        // Which command I've selected?
                        match sett_page.command_offset {
                            0 => {
                                // Add Sender
                                // Save current state before entering command popup
                                self.save_navigation_state();
                                sett_page.command_popup = ClientSettingsCommandsPopup::AddSender(CommandPopup {
                                    status: SettingsPopupStatus::Idle,
                                    cursor: 0,
                                });
                            }
                            1 => {
                                // Remove Sender
                                // Save current state before entering command popup
                                self.save_navigation_state();
                                sett_page.command_popup = ClientSettingsCommandsPopup::RemoveSender(CommandPopup {
                                    status: SettingsPopupStatus::Idle,
                                    cursor: 0,
                                });
                            }
                            2 => {
                                // Recompute Topology Map
                                self.initialize_node_pop_map(id);
                            }
                            _ => (),
                        }
                    }
                    _ => (),
                }
            }
            1 => {
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
                        // Re-Compute Map
                        self.initialize_node_pop_map(id);
                    }
                    _ => (),
                }
            }
            _ => {

            }
        }
        self.cache.tab1_data.viewing =
            ViewedTab1::Popup((
                id,
                NodePopup::Client(
                    ClientPopup::SettingsPage(sett_page)
                )
            ))
    }
    fn handle_client_add_sender_popup(&mut self, key:KeyEvent, id:NodeId, mut sett_page:ClientSettingsPopup, mut command_popup: CommandPopup){
        match key.code{
            KeyCode::Esc => {
                // Go back
                sett_page.command_popup = ClientSettingsCommandsPopup::None;
            }
            KeyCode::Up => {
                if command_popup.cursor>0{
                    command_popup.cursor -= 1;
                    sett_page.command_popup = ClientSettingsCommandsPopup::AddSender(command_popup);
                }
            }
            KeyCode::Down => {
                if command_popup.cursor<1{
                    command_popup.cursor += 1;
                    sett_page.command_popup = ClientSettingsCommandsPopup::AddSender(command_popup);
                }
            }
            KeyCode::Enter => {
                match command_popup.cursor{
                    0 => {
                        // text editor, first field
                        self.app_mode = AppMode::Input(InputMode::One);
                    }
                    1 => {
                        // parse and send
                        match self.cache.text_buffer.first_field_parse_u8(true){
                            Ok(id_target) => {
                                match self.simulation_controller_data.add_sender(id,id_target){
                                    Ok(_) => {
                                        command_popup.status = SettingsPopupStatus::CorrectlySent;
                                        let new_links = self.populate_topology_links();
                                        self.simulation_controller_data.topology_algorithm_data.initialize(new_links);
                                        self.initialize_node_pop_map(id);
                                    }
                                    Err(err) => {
                                        command_popup.status = SettingsPopupStatus::Error(err);
                                    }
                                }
                            }
                            Err(err) => {
                                command_popup.status = SettingsPopupStatus::Error(err);
                            }
                        }
                    }
                    _ => (),
                }
                sett_page.command_popup = ClientSettingsCommandsPopup::AddSender(command_popup);
            }
            _ => (),
        }
        self.cache.tab1_data.viewing =
            ViewedTab1::Popup((
                id,
                NodePopup::Client(
                    ClientPopup::SettingsPage(sett_page)
                )
            ))
    }
    fn handle_client_remove_sender_popup(&mut self, key:KeyEvent, id:NodeId, mut sett_page:ClientSettingsPopup, mut command_popup: CommandPopup){
        match key.code{
            KeyCode::Esc => {
                // Go back
                sett_page.command_popup = ClientSettingsCommandsPopup::None;
            }
            KeyCode::Up => {
                if command_popup.cursor>0{
                    command_popup.cursor -= 1;
                    sett_page.command_popup = ClientSettingsCommandsPopup::RemoveSender(command_popup);
                }
            }
            KeyCode::Down => {
                if command_popup.cursor<1{
                    command_popup.cursor += 1;
                    sett_page.command_popup = ClientSettingsCommandsPopup::RemoveSender(command_popup);
                }
            }
            KeyCode::Enter => {
                match command_popup.cursor{
                    0 => {
                        // text editor, first field
                        self.app_mode = AppMode::Input(InputMode::One);
                    }
                    1 => {
                        // parse and send
                        match self.cache.text_buffer.first_field_parse_u8(true){
                            Ok(id_target) => {
                                match self.simulation_controller_data.remove_sender(id,id_target){
                                    Ok(_) => {
                                        command_popup.status = SettingsPopupStatus::CorrectlySent;
                                        let new_links = self.populate_topology_links();
                                        self.simulation_controller_data.topology_algorithm_data.initialize(new_links);
                                        self.initialize_node_pop_map(id);
                                    }
                                    Err(err) => {
                                        command_popup.status = SettingsPopupStatus::Error(err);
                                    }
                                }
                            }
                            Err(err) => {
                                command_popup.status = SettingsPopupStatus::Error(err);
                            }
                        }
                    }
                    _ => (),
                }
                sett_page.command_popup = ClientSettingsCommandsPopup::RemoveSender(command_popup);
            }
            _ => (),
        }
        self.cache.tab1_data.viewing =
            ViewedTab1::Popup((
                id,
                NodePopup::Client(
                    ClientPopup::SettingsPage(sett_page)
                )
            ))
    }
    fn handle_data_server_page(&mut self, key: KeyEvent, id: NodeId, data_page: ServerDataPopup) {
        match key.code {
            KeyCode::Up => {
                if self.cache.tab1_data.popup_cursor == 3 {
                    // From log box, go to rightmost upper box (data_3)
                    self.cache.tab1_data.popup_cursor = 2;
                } else if self.cache.tab1_data.popup_cursor > 0 {
                    self.cache.tab1_data.popup_cursor -= 1;
                } else {
                    // Wrap around to the log box
                    self.cache.tab1_data.popup_cursor = 3;
                }
            }
            KeyCode::Down => {
                if self.cache.tab1_data.popup_cursor < 3 {
                    // From any upper box (0, 1, 2) go directly to log box (3)
                    self.cache.tab1_data.popup_cursor = 3;
                } else {
                    // From log box, wrap around to first element
                    self.cache.tab1_data.popup_cursor = 0;
                }
            }
            KeyCode::Left => {
                // Move left if possible within the horizontal layout
                if self.cache.tab1_data.popup_cursor == 1 {
                    self.cache.tab1_data.popup_cursor = 0;
                } else if self.cache.tab1_data.popup_cursor == 2 {
                    self.cache.tab1_data.popup_cursor = 1;
                } else if self.cache.tab1_data.popup_cursor == 0 {
                    // Go back to menu selection
                    self.cache.tab1_data.popup_selection = PopupSelection::MenuArea;
                    self.cache.tab1_data.popup_cursor = 0; // Set to Data menu item (servers only have Data and Settings)
                } else {
                    // From filtered logging area
                    self.cache.tab1_data.popup_selection = PopupSelection::MenuArea;
                    self.cache.tab1_data.popup_cursor = 0;
                }
            }
            KeyCode::Right => {
                // Move right if possible within the horizontal layout
                if self.cache.tab1_data.popup_cursor == 0 {
                    self.cache.tab1_data.popup_cursor = 1;
                } else if self.cache.tab1_data.popup_cursor == 1 {
                    self.cache.tab1_data.popup_cursor = 2;
                }
            }
            KeyCode::Enter => {
                // Enter key behavior
                if self.cache.tab1_data.popup_cursor == 3 {
                    // Enter scrolling mode for the log field (same as tab0)
                    self.app_mode = AppMode::Scrolling;
                    info!(target: "main", "[LEDRON-CLI] Entered log scroll mode for server node {}", id);
                } else {
                    info!(target: "main", "[LEDRON-CLI] Server data chart {} selected for node {}",
                          self.cache.tab1_data.popup_cursor, id);
                }
            }
            KeyCode::Esc => {
                // ESC closes the popup completely
                self.cache.tab1_data.return_to_main();
            }
            _ => {}
        }

        // Update the viewing state
        self.cache.tab1_data.viewing = ViewedTab1::Popup((
            id,
            NodePopup::Server(ServerPopup::DataPage(data_page))
        ));
    }



    fn handle_server_popup(&mut self, key: KeyEvent,id:NodeId, server_popup: ServerPopup){
        match self.cache.tab1_data.popup_selection {
            PopupSelection::MenuArea => {
                match key.code{
                    KeyCode::Up => {
                        if self.cache.tab1_data.popup_cursor > 0{
                            self.cache.tab1_data.popup_cursor -= 1;
                        }else {
                            self.cache.tab1_data.popup_cursor = 1;
                        }
                        self.coordinate_server_popup_cursor_with_opt_menu(id, self.cache.tab1_data.popup_cursor);
                    }
                    KeyCode::Down => {
                        if self.cache.tab1_data.popup_cursor < 1{
                            self.cache.tab1_data.popup_cursor += 1;
                        }else{
                            self.cache.tab1_data.popup_cursor = 0;
                        }
                        self.coordinate_server_popup_cursor_with_opt_menu(id, self.cache.tab1_data.popup_cursor);
                    }
                    KeyCode::Right | KeyCode::Enter => {
                        self.cache.tab1_data.popup_selection = PopupSelection::OptArea;
                        self.coordinate_server_popup_cursor_with_opt_menu(id, self.cache.tab1_data.popup_cursor);
                        self.cache.tab1_data.popup_cursor = 0;
                    }
                    _ => (),
                }
            }
            PopupSelection::OptArea => {
                match key.code {
                    KeyCode::Esc => {
                        self.cache.tab1_data.popup_selection = PopupSelection::MenuArea;
                        self.cache.tab1_data.popup_cursor = 0;
                    }
                    _ => {
                        match server_popup{
                            ServerPopup::DataPage(datapage) => {
                                self.handle_data_server_page(key, id, datapage)
                            }
                            ServerPopup::SettingsPage(settpage) => {
                                match settpage.command_popup.clone(){
                                    ServerSettingsCommandsPopup::None => {
                                        self.handle_sett_server_page(key,id,settpage);
                                    }
                                    ServerSettingsCommandsPopup::AddSender(addsend) => {
                                        self.handle_server_add_sender_popup(key, id, settpage, addsend);
                                    }
                                    ServerSettingsCommandsPopup::RemoveSender(remsend) => {
                                        self.handle_server_remove_sender_popup(key, id, settpage, remsend);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn handle_sett_server_page(&mut self, key:KeyEvent, id:NodeId, mut sett_page: ServerSettingsPopup){
        match self.cache.tab1_data.popup_cursor{
            0 => {
                // Commands
                match key.code{
                    KeyCode::Up => {
                        sett_page.command_up();
                    },
                    KeyCode::Down => {
                        sett_page.command_down();
                    }
                    KeyCode::Right => {
                        if self.cache.tab1_data.popup_cursor < 1{
                            self.cache.tab1_data.popup_cursor += 1;
                        }
                    }
                    KeyCode::Left => {
                        if self.cache.tab1_data.popup_cursor > 0{
                            self.cache.tab1_data.popup_cursor -= 1;
                        }else{
                            self.cache.tab1_data.popup_selection = PopupSelection::MenuArea;
                            self.cache.tab1_data.popup_cursor = 1;
                        }
                    }
                    KeyCode::Enter => {
                        // Which command I've selected?
                        match sett_page.command_offset {
                            0 => {
                                // Add Sender
                                sett_page.command_popup = ServerSettingsCommandsPopup::AddSender(CommandPopup {
                                    status: SettingsPopupStatus::Idle,
                                    cursor: 0,
                                });
                            }
                            1 => {
                                // Remove Sender
                                sett_page.command_popup = ServerSettingsCommandsPopup::RemoveSender(CommandPopup {
                                    status: SettingsPopupStatus::Idle,
                                    cursor: 0,
                                });
                            }
                            2 => {
                                // Recompute Topology Map
                                self.initialize_node_pop_map(id);
                            }
                            _ => (),
                        }
                    }
                    _ => (),
                }
            }
            1 => {
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
                        // Re-Compute Map
                        self.initialize_node_pop_map(id);
                    }
                    _ => (),
                }
            }
            _ => {

            }
        }
        self.cache.tab1_data.viewing =
            ViewedTab1::Popup((
                id,
                NodePopup::Server(
                    ServerPopup::SettingsPage(sett_page)
                )
            ))
    }
    fn handle_server_add_sender_popup(&mut self, key:KeyEvent, id:NodeId, mut sett_page:ServerSettingsPopup, mut command_popup: CommandPopup){
        match key.code{
            KeyCode::Esc => {
                // Go back
                sett_page.command_popup = ServerSettingsCommandsPopup::None;
            }
            KeyCode::Up => {
                if command_popup.cursor>0{
                    command_popup.cursor -= 1;
                    sett_page.command_popup = ServerSettingsCommandsPopup::AddSender(command_popup);
                }
            }
            KeyCode::Down => {
                if command_popup.cursor<1{
                    command_popup.cursor += 1;
                    sett_page.command_popup = ServerSettingsCommandsPopup::AddSender(command_popup);
                }
            }
            KeyCode::Enter => {
                match command_popup.cursor{
                    0 => {
                        // text editor, first field
                        self.app_mode = AppMode::Input(InputMode::One);
                    }
                    1 => {
                        // parse and send
                        match self.cache.text_buffer.first_field_parse_u8(true){
                            Ok(id_target) => {
                                match self.simulation_controller_data.add_sender(id,id_target){
                                    Ok(_) => {
                                        command_popup.status = SettingsPopupStatus::CorrectlySent;
                                        let new_links = self.populate_topology_links();
                                        self.simulation_controller_data.topology_algorithm_data.initialize(new_links);
                                        self.initialize_node_pop_map(id);
                                    }
                                    Err(err) => {
                                        command_popup.status = SettingsPopupStatus::Error(err);
                                    }
                                }
                            }
                            Err(err) => {
                                command_popup.status = SettingsPopupStatus::Error(err);
                            }
                        }
                    }
                    _ => (),
                }
                sett_page.command_popup = ServerSettingsCommandsPopup::AddSender(command_popup);
            }
            _ => (),
        }
        self.cache.tab1_data.viewing =
            ViewedTab1::Popup((
                id,
                NodePopup::Server(
                    ServerPopup::SettingsPage(sett_page)
                )
            ))
    }
    fn handle_server_remove_sender_popup(&mut self, key:KeyEvent, id:NodeId, mut sett_page:ServerSettingsPopup, mut command_popup: CommandPopup){
        match key.code{
            KeyCode::Esc => {
                // Go back
                sett_page.command_popup = ServerSettingsCommandsPopup::None;
            }
            KeyCode::Up => {
                if command_popup.cursor>0{
                    command_popup.cursor -= 1;
                    sett_page.command_popup = ServerSettingsCommandsPopup::RemoveSender(command_popup);
                }
            }
            KeyCode::Down => {
                if command_popup.cursor<1{
                    command_popup.cursor += 1;
                    sett_page.command_popup = ServerSettingsCommandsPopup::RemoveSender(command_popup);
                }
            }
            KeyCode::Enter => {
                match command_popup.cursor{
                    0 => {
                        // text editor, first field
                        self.app_mode = AppMode::Input(InputMode::One);
                    }
                    1 => {
                        // parse and send
                        match self.cache.text_buffer.first_field_parse_u8(true){
                            Ok(id_target) => {
                                match self.simulation_controller_data.remove_sender(id,id_target){
                                    Ok(_) => {
                                        command_popup.status = SettingsPopupStatus::CorrectlySent;
                                        let new_links = self.populate_topology_links();
                                        self.simulation_controller_data.topology_algorithm_data.initialize(new_links);
                                        self.initialize_node_pop_map(id);
                                    }
                                    Err(err) => {
                                        command_popup.status = SettingsPopupStatus::Error(err);
                                    }
                                }
                            }
                            Err(err) => {
                                command_popup.status = SettingsPopupStatus::Error(err);
                            }
                        }
                    }
                    _ => (),
                }
                sett_page.command_popup = ServerSettingsCommandsPopup::RemoveSender(command_popup);
            }
            _ => (),
        }
        self.cache.tab1_data.viewing =
            ViewedTab1::Popup((
                id,
                NodePopup::Server(
                    ServerPopup::SettingsPage(sett_page)
                )
            ))
    }


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
                match key.code {
                    KeyCode::Esc => {
                        self.cache.tab1_data.popup_selection = PopupSelection::MenuArea;
                        self.cache.tab1_data.popup_cursor = 0;
                    }
                    _ => {
                        match client_popup{
                            DronePopup::DataPage(datapage) => {
                                self.handle_data_drone_page(key, id, datapage)
                            }
                            DronePopup::SettingsPage(settpage) => {
                                match settpage.command_popup.clone(){
                                    DroneSettingsCommandsPopup::None => {
                                        self.handle_sett_drone_page(key,id,settpage)
                                    }
                                    DroneSettingsCommandsPopup::SetPDR(commpop) => {
                                        self.handle_drone_set_pdr_popup(key, id, settpage, commpop);
                                    }
                                    DroneSettingsCommandsPopup::AddSender(addsend) => {
                                        self.handle_drone_add_sender_popup(key, id, settpage, addsend);
                                    }
                                    DroneSettingsCommandsPopup::RemoveSender(remsend) => {
                                        self.handle_drone_remove_sender_popup(key, id, settpage, remsend);
                                    }
                                    DroneSettingsCommandsPopup::Crash(crapop) => {
                                        self.handle_drone_crash_drone_popup(key, id, settpage, crapop);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    fn handle_data_drone_page(&mut self, key:KeyEvent,id:NodeId, data_page: DroneDataPopup){
        match key.code {
            KeyCode::Up => {
                if self.cache.tab1_data.popup_cursor == 3 {
                    // From log box, go to rightmost upper box (data_3)
                    self.cache.tab1_data.popup_cursor = 2;
                } else if self.cache.tab1_data.popup_cursor > 0 {
                    self.cache.tab1_data.popup_cursor -= 1;
                } else {
                    // Wrap around to the log box
                    self.cache.tab1_data.popup_cursor = 3;
                }
            }
            KeyCode::Down => {
                if self.cache.tab1_data.popup_cursor < 3 {
                    // From any upper box (0, 1, 2) go directly to log box (3)
                    self.cache.tab1_data.popup_cursor = 3;
                } else {
                    // From log box, wrap around to first element
                    self.cache.tab1_data.popup_cursor = 0;
                }
            }
            KeyCode::Left => {
                // Move left within horizontal layout
                if self.cache.tab1_data.popup_cursor == 1 {
                    self.cache.tab1_data.popup_cursor = 0;
                } else if self.cache.tab1_data.popup_cursor == 2 {
                    self.cache.tab1_data.popup_cursor = 1;
                } else if self.cache.tab1_data.popup_cursor == 0 {
                    // Go back to menu selection
                    self.cache.tab1_data.popup_selection = PopupSelection::MenuArea;
                    self.cache.tab1_data.popup_cursor = 0; // Set to Data menu item (drones have Data and Settings)
                } else {
                    // From bottom area
                    self.cache.tab1_data.popup_selection = PopupSelection::MenuArea;
                    self.cache.tab1_data.popup_cursor = 0;
                }
            }
            KeyCode::Right => {
                // Move right within horizontal layout
                if self.cache.tab1_data.popup_cursor == 0 {
                    self.cache.tab1_data.popup_cursor = 1;
                } else if self.cache.tab1_data.popup_cursor == 1 {
                    self.cache.tab1_data.popup_cursor = 2;
                }
            }
            KeyCode::Enter => {
                // Enter key behavior
                if self.cache.tab1_data.popup_cursor == 3 {
                    // Enter scrolling mode for the log field (same as tab0)
                    self.app_mode = AppMode::Scrolling;
                    info!(target: "main", "[LEDRON-CLI] Entered log scroll mode for drone node {}", id);
                } else {
                    info!(target: "main", "[LEDRON-CLI] Drone data chart {} selected for node {}",
                          self.cache.tab1_data.popup_cursor, id);
                }
            }
            KeyCode::Esc => {
                // ESC closes the popup completely
                self.cache.tab1_data.return_to_main();
            }
            _ => {}
        }

        // Update the viewing state
        self.cache.tab1_data.viewing = ViewedTab1::Popup((
            id,
            NodePopup::Drone(DronePopup::DataPage(data_page))
        ));
    }
    fn handle_sett_drone_page(&mut self, key:KeyEvent, id:NodeId, mut sett_page:DroneSettingsPopup){
        match self.cache.tab1_data.popup_cursor{
            0 => {
                // Commands
                match key.code{
                    KeyCode::Up => {
                        sett_page.command_up();
                    },
                    KeyCode::Down => {
                        sett_page.command_down();
                    }
                    KeyCode::Right => {
                        if self.cache.tab1_data.popup_cursor < 1{
                            self.cache.tab1_data.popup_cursor += 1;
                        }
                    }
                    KeyCode::Left => {
                        if self.cache.tab1_data.popup_cursor > 0{
                            self.cache.tab1_data.popup_cursor -= 1;
                        }else{
                            self.cache.tab1_data.popup_selection = PopupSelection::MenuArea;
                            self.cache.tab1_data.popup_cursor = 1;
                        }
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
            1 => {
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
                        // Re-Compute Map
                        self.initialize_node_pop_map(id);
                    }
                    _ => (),
                }
            }
            _ => {

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

    fn handle_drone_set_pdr_popup(&mut self, key:KeyEvent, id:NodeId, mut sett_page:DroneSettingsPopup, mut command_popup: CommandPopup){
        match key.code{
            KeyCode::Esc => {
                // Go back
                sett_page.command_popup = DroneSettingsCommandsPopup::None;
            }
            KeyCode::Up => {
                if command_popup.cursor>0{
                    command_popup.cursor -= 1;
                    sett_page.command_popup = DroneSettingsCommandsPopup::SetPDR(command_popup);
                }
            }
            KeyCode::Down => {
                if command_popup.cursor<1{
                    command_popup.cursor += 1;
                    sett_page.command_popup = DroneSettingsCommandsPopup::SetPDR(command_popup);
                }
            }
            KeyCode::Enter => {
                match command_popup.cursor{
                    0 => {
                        // text editor, first field
                        self.app_mode = AppMode::Input(InputMode::One);
                    }
                    1 => {
                        // parse and send
                        match self.cache.text_buffer.first_field_parse_f32(true){
                            Ok(new_pdr) => {
                                match self.simulation_controller_data.set_pdr(id,new_pdr){
                                    Ok(_) => {
                                        command_popup.status = SettingsPopupStatus::CorrectlySent;
                                        sett_page.drone_pdr = new_pdr;
                                    }
                                    Err(err) => {
                                        command_popup.status = SettingsPopupStatus::Error(err);
                                    }
                                }
                            }
                            Err(err) => {
                                command_popup.status = SettingsPopupStatus::Error(err);
                            }
                        }
                    }
                    _ => (),
                }
                sett_page.command_popup = DroneSettingsCommandsPopup::SetPDR(command_popup);
            }
            _ => (),
        }
        self.cache.tab1_data.viewing =
            ViewedTab1::Popup((
                id,
                NodePopup::Drone(
                    DronePopup::SettingsPage(sett_page)
                )
            ))
    }
    fn handle_drone_add_sender_popup(&mut self, key:KeyEvent, id:NodeId, mut sett_page:DroneSettingsPopup, mut command_popup: CommandPopup){
        match key.code{
            KeyCode::Esc => {
                // Go back
                sett_page.command_popup = DroneSettingsCommandsPopup::None;
            }
            KeyCode::Up => {
                if command_popup.cursor>0{
                    command_popup.cursor -= 1;
                    sett_page.command_popup = DroneSettingsCommandsPopup::AddSender(command_popup);
                }
            }
            KeyCode::Down => {
                if command_popup.cursor<1{
                    command_popup.cursor += 1;
                    sett_page.command_popup = DroneSettingsCommandsPopup::AddSender(command_popup);
                }
            }
            KeyCode::Enter => {
                match command_popup.cursor{
                    0 => {
                        // text editor, first field
                        self.app_mode = AppMode::Input(InputMode::One);
                    }
                    1 => {
                        // parse and send
                        match self.cache.text_buffer.first_field_parse_u8(true){
                            Ok(id_target) => {
                                match self.simulation_controller_data.add_sender(id,id_target){
                                    Ok(_) => {
                                        command_popup.status = SettingsPopupStatus::CorrectlySent;
                                        let new_links = self.populate_topology_links();
                                        self.simulation_controller_data.topology_algorithm_data.initialize(new_links);
                                        self.initialize_node_pop_map(id);
                                    }
                                    Err(err) => {
                                        command_popup.status = SettingsPopupStatus::Error(err);
                                    }
                                }
                            }
                            Err(err) => {
                                command_popup.status = SettingsPopupStatus::Error(err);
                            }
                        }
                    }
                    _ => (),
                }
                sett_page.command_popup = DroneSettingsCommandsPopup::AddSender(command_popup);
            }
            _ => (),
        }
        self.cache.tab1_data.viewing =
            ViewedTab1::Popup((
                id,
                NodePopup::Drone(
                    DronePopup::SettingsPage(sett_page)
                )
            ))
    }
    fn handle_drone_remove_sender_popup(&mut self, key:KeyEvent, id:NodeId, mut sett_page:DroneSettingsPopup, mut command_popup: CommandPopup){
        match key.code{
            KeyCode::Esc => {
                // Go back
                sett_page.command_popup = DroneSettingsCommandsPopup::None;
            }
            KeyCode::Up => {
                if command_popup.cursor>0{
                    command_popup.cursor -= 1;
                    sett_page.command_popup = DroneSettingsCommandsPopup::RemoveSender(command_popup);
                }
            }
            KeyCode::Down => {
                if command_popup.cursor<1{
                    command_popup.cursor += 1;
                    sett_page.command_popup = DroneSettingsCommandsPopup::RemoveSender(command_popup);
                }
            }
            KeyCode::Enter => {
                match command_popup.cursor{
                    0 => {
                        // text editor, first field
                        self.app_mode = AppMode::Input(InputMode::One);
                    }
                    1 => {
                        // parse and send
                        match self.cache.text_buffer.first_field_parse_u8(true){
                            Ok(id_target) => {
                                match self.simulation_controller_data.remove_sender(id,id_target){
                                    Ok(_) => {
                                        command_popup.status = SettingsPopupStatus::CorrectlySent;
                                        let new_links = self.populate_topology_links();
                                        self.simulation_controller_data.topology_algorithm_data.initialize(new_links);
                                        self.initialize_node_pop_map(id);
                                    }
                                    Err(err) => {
                                        command_popup.status = SettingsPopupStatus::Error(err);
                                    }
                                }
                            }
                            Err(err) => {
                                command_popup.status = SettingsPopupStatus::Error(err);
                            }
                        }
                    }
                    _ => (),
                }
                sett_page.command_popup = DroneSettingsCommandsPopup::RemoveSender(command_popup);
            }
            _ => (),
        }
        self.cache.tab1_data.viewing =
            ViewedTab1::Popup((
                id,
                NodePopup::Drone(
                    DronePopup::SettingsPage(sett_page)
                )
            ))
    }
    fn handle_drone_crash_drone_popup(&mut self, key:KeyEvent, id:NodeId, mut sett_page:DroneSettingsPopup, mut command_popup: CommandPopup){
        match key.code{
            KeyCode::Esc => {
                // Go back
                sett_page.command_popup = DroneSettingsCommandsPopup::None;
            }
            KeyCode::Enter => {
                match self.simulation_controller_data.crash(id){
                    Ok(_) => {
                        command_popup.status = SettingsPopupStatus::CorrectlySent;
                    }
                    Err(err) => {
                        command_popup.status = SettingsPopupStatus::Error(err);
                    }
                }
                sett_page.command_popup = DroneSettingsCommandsPopup::Crash(command_popup);
            }
            _ => (),
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
