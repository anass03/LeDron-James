use ratatui::crossterm::event::{KeyCode, KeyEvent};
use wg_2024::network::NodeId;
use drone::Drone;
use crate::backend::sc_models::LogType;
use crate::frontend::app::App;
use crate::frontend::app::models::general_types::Node;
use crate::frontend::app::models::tab1_types::{ChFirstPage, ChSecondPage, ClientChatPopup, ClientPopup, DronePopup, NodePopup, PopupSelection, ServerPopup, ViewedTab1};

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
                self.cache.tab1_data.nodelist.select_prev();
            }
            KeyCode::Down => {
                self.cache.tab1_data.nodelist.select_next();
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
                        let selected_cursor_item = self.cache.tab1_data.nodelist.selected as usize;
                        match self.cache.tab1_data.nodelist.items.get(selected_cursor_item){
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
                                                    NodePopup::Drone(DronePopup::FirstPage())
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

    fn handle_popup(&mut self, key:KeyEvent,id:NodeId, node_popup: NodePopup) {
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
                                if self.cache.tab1_data.popup_cursor as isize -3 >=0{
                                    self.cache.tab1_data.popup_cursor-=3
                                }
                            }
                            KeyCode::Down => {
                                if self.cache.tab1_data.popup_cursor+3 < chat_servers_len  {
                                    self.cache.tab1_data.popup_cursor+=3
                                }
                            }
                            KeyCode::Left => {
                                if self.cache.tab1_data.popup_cursor>0{
                                    self.cache.tab1_data.popup_cursor-=1;
                                }else{
                                    // Return on Menu
                                    self.cache.tab1_data.popup_selection = PopupSelection::MenuArea;
                                    self.cache.tab1_data.popup_cursor = 0; // Re-selecting Chat Menu
                                }
                            }
                            KeyCode::Right => {
                                if self.cache.tab1_data.popup_cursor + 1 < chat_servers_len  {
                                    self.cache.tab1_data.popup_cursor+=1;
                                }
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
            }
            ClientChatPopup::ThirdPage(chat_data) => {

            }
        }
    }

    fn handle_server_popup(&mut self, key: KeyEvent,id:NodeId, server_popup: ServerPopup){

    }
    fn handle_drone_popup(&mut self, key: KeyEvent,id:NodeId, client_popup: DronePopup){

    }
}



