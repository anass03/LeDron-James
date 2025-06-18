use std::ops::DerefMut;
use host::ClientId;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use wg_2024::network::NodeId;
use drone::Drone;
use server::chat2::Chat;
use crate::backend::sc_models::{ChatList, LogType};
use crate::frontend::app::App;
use crate::frontend::app::models::general_types::Node;
use crate::frontend::app::models::main_types::AppMode;
use crate::frontend::app::models::tab1_types::{ChFirstPage, ChSecondPage, ChThirdPage, ChatThirdPageSelection, ClientChatPopup, ClientNetPopup, ClientPopup, DronePopup, NodePopup, PopupSelection, ServerPopup, ViewedTab1};

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
                            Some((node,is_active)) => {
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
                                                    NodePopup::Drone(DronePopup::DataPage())
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
    fn coordinate_popup_cursor_with_opt_menu(&mut self,id:NodeId, current_cursor: usize){
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
                /*self.cache.tab1_data.viewing =
                    ViewedTab1::Popup((
                        id,
                        NodePopup::Client(ClientPopup::InternetPage(

                        ))
                    ))*/
            }
            2 => {
                /*self.cache.tab1_data.viewing =
                    ViewedTab1::Popup((
                        id,
                        NodePopup::Client(ClientPopup::DataPage(

                        ))
                    ))*/
            }
            3 => {
                /*self.cache.tab1_data.viewing =
                    ViewedTab1::Popup((
                        id,
                        NodePopup::Client(ClientPopup::SettingsPage(

                        ))
                    ))*/
            }
            _ => (),
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
                        self.coordinate_popup_cursor_with_opt_menu(id,self.cache.tab1_data.popup_cursor);
                    }
                    KeyCode::Down => {
                        if self.cache.tab1_data.popup_cursor < 3{
                            self.cache.tab1_data.popup_cursor += 1;
                        }else{
                            self.cache.tab1_data.popup_cursor = 0;
                        }
                        self.coordinate_popup_cursor_with_opt_menu(id,self.cache.tab1_data.popup_cursor);
                    }
                    KeyCode::Right | KeyCode::Enter => {
                        self.cache.tab1_data.popup_selection = PopupSelection::OptArea;
                        self.coordinate_popup_cursor_with_opt_menu(id,self.cache.tab1_data.popup_cursor);
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
                    "i" => {
                        self.simulation_controller_data.logs.push_sc_log(LogType::INFO, format!("{:?}\n{:?}", self.simulation_controller_data.chat_cache.known_contacts, self.simulation_controller_data.chat_cache.known_chat_servers))
                    }
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
                                        self.cache.tab1_data.file_exploring_data.set_fields(id, chat_data.selected_contact, chat_data.selected_server);
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
    fn handle_net_page(&mut self, key: KeyEvent,id:NodeId, net_page: ClientNetPopup){}
    fn handle_data_page(&mut self, key: KeyEvent,id:NodeId, net_page: ClientNetPopup){}
    fn handle_sett_page(&mut self, key:KeyEvent,id:NodeId){}



    fn handle_server_popup(&mut self, key: KeyEvent,id:NodeId, server_popup: ServerPopup){}
    fn handle_drone_popup(&mut self, key: KeyEvent,id:NodeId, client_popup: DronePopup){}

}



