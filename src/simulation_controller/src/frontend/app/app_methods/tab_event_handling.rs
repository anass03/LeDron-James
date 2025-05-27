/*
In this module we will handle the specific events related to single tabs.
Our event handler from main App file, will delegate these methods those said events.
 */
use super::super::App;
use crate::backend::simulation_controller::LogType;
use crate::frontend::app::model::{AppMode, Node};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use wg_2024::network::NodeId;
impl App<'_> {
    pub(super) fn handle_event_tab0(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up => {
                if self.cache.cursor != 0 {
                    //println!("zio");
                    self.cache.cursor -= 1
                }
            }
            KeyCode::Down => {
                if self.cache.cursor != 2 {
                    //println!("zio");
                    self.cache.cursor += 1
                }
            }
            KeyCode::Enter => {
                match self.cache.cursor {
                    0 => {}
                    1 => {
                        // TODO! Log scroll mode
                        self.app_mode = AppMode::Scrolling;
                        self.simulation_controller_data.logs.log_scroll_offset += 1;
                    }
                    _ => (),
                }
            }
            _ => (),
        }
    }
    pub(super) fn handle_event_tab1(&mut self, key: KeyCode) {
        match self.cache.cursor {
            0 => {
                match key {
                    KeyCode::Up => {
                        self.cache.tab1_data.nodelist.select_prev();
                        self.cache.tab1_data.selected_fn.select_first();
                        self.cache.tab1_data.selected_button = 0;
                    }
                    KeyCode::Down => {
                        self.cache.tab1_data.nodelist.select_next();
                        self.cache.tab1_data.selected_fn.select_first();
                        self.cache.tab1_data.selected_button = 0;
                    }
                    KeyCode::Right | KeyCode::Enter => {
                        self.cache.cursor += 1;
                    }

                    // Missing KeyCodes to view the first one / last one, still gotta decide!
                    _ => (), // Do nothing
                }
            }
            1 => {
                let selected_n = self.cache.tab1_data.nodelist.selected;
                match self.cache.tab1_data.nodelist.items.get(selected_n as usize) {
                    None => {
                        self.handle_unknown(key);
                    }
                    Some((n, b)) => {
                        if *b {
                            match n {
                                Node::Client(_) => {
                                    // We have 9 Options
                                    /*
                                    "[1] Server Type",
                                    "[2] FilesList",
                                    "[3] File",
                                    "[4] MediaList",
                                    "[5] Media",
                                    "[7] Registration to chat",
                                    "[8] ClientList",
                                    "[9] Message For",
                                                             */
                                    match key {
                                        KeyCode::Up => {
                                            if self.cache.tab1_data.selected_fn.selected > 0 {
                                                self.cache.tab1_data.selected_fn.select_prev();
                                                self.cache.tab1_data.selected_button = 0;
                                            }
                                        }
                                        KeyCode::Down => {
                                            if self.cache.tab1_data.selected_fn.selected + 1 < 8 {
                                                self.cache.tab1_data.selected_fn.select_next();
                                                self.cache.tab1_data.selected_button = 0;
                                            }
                                        }
                                        KeyCode::Left => {
                                            self.cache.cursor -= 1;
                                        }
                                        KeyCode::Right | KeyCode::Enter => {
                                            self.cache.cursor += 1;
                                        }
                                        // Missing KeyCodes to view the first one / last one, still gotta decide!
                                        _ => (), // Do nothing
                                    }
                                }
                                Node::Server(_) => {
                                    // O options
                                    match key {
                                        KeyCode::Left => {
                                            self.cache.cursor -= 1;
                                        }
                                            KeyCode::Enter => {
                                                //
                                            },

                                        // Missing KeyCodes to view the first one / last one, still gotta decide!
                                        _ => (), // Do nothing
                                    }
                                }
                                Node::Drone(_) => {
                                    // 4 Options
                                    /*
                                    [1] Remove Sender", "[2] Add Sender", "[3] Set PDR", "[4] Crash"];
                                     */
                                    match key {
                                        KeyCode::Up => {
                                            if self.cache.tab1_data.selected_fn.selected > 0 {
                                                self.cache.tab1_data.selected_fn.select_prev();
                                                self.cache.tab1_data.selected_button = 0;
                                            }
                                        }
                                        KeyCode::Down => {
                                            if self.cache.tab1_data.selected_fn.selected + 1 < 4 {
                                                self.cache.tab1_data.selected_fn.select_next();
                                                self.cache.tab1_data.selected_button = 0;
                                            }
                                        }
                                        KeyCode::Left => {
                                            self.cache.cursor -= 1;
                                        }
                                        KeyCode::Right | KeyCode::Enter => {
                                            self.cache.cursor += 1;
                                        }
                                        // Missing KeyCodes to view the first one / last one, still gotta decide!
                                        _ => (), // Do nothing
                                    }
                                }
                            }
                        }
                    }
                }
            }
            2 => {
                // Each specific fn case cover different event handling

                // Mi trovo in una tab a 1 o a 2 bottoni?
                let selected_n = self.cache.tab1_data.nodelist.selected;
                match self.cache.tab1_data.nodelist.items.get(selected_n as usize) {
                    None => {
                        // unreachable
                    }
                    Some((n, _)) => {
                        match n {
                            Node::Client(c) => {
                                let id = c.id;
                                /*
                                "[1] Server Type",
                                "[2] FilesList",
                                "[3] File [ 1 input ]",
                                "[4] MediaList",
                                "[5] Media [ 1 input ]",
                                "[6] Registration to chat",
                                "[7] ClientList",
                                "[8] Message For [ 2 input ]",
                                     */
                                match self.cache.tab1_data.selected_fn.selected {
                                    0 | 1 | 3 | 5 | 6 => self.tab1_0input_handle(
                                        key,
                                        id,
                                        SelectedNode::Client(self.cache.tab1_data.selected_fn.selected),
                                    ),
                                    2 | 4 => self.tab1_1input_handle(
                                        key,
                                        id,
                                        SelectedNode::Client(self.cache.tab1_data.selected_fn.selected),
                                    ),
                                    7 => self.tab1_2input_handle(
                                        key,
                                        id,
                                        SelectedNode::Client(self.cache.tab1_data.selected_fn.selected),
                                    ),
                                    _ => self.handle_unknown(key),
                                }
                            }
                            Node::Server(_) => self.handle_unknown(key),
                            Node::Drone(d) => {
                                let id = d.id;
                                match self.cache.tab1_data.selected_fn.selected {
                                    0 | 1 | 2=> self.tab1_1input_handle(
                                        key,
                                        id,
                                        SelectedNode::Drone(self.cache.tab1_data.selected_fn.selected),
                                    ),
                                    3  => self.tab1_0input_handle(
                                        key,
                                        id,
                                        SelectedNode::Drone(self.cache.tab1_data.selected_fn.selected),
                                    ),
                                    _ => self.handle_unknown(key),
                                }
                            },
                        }
                    }
                }
            }
            _ => {}
        }
    }
    fn tab1_0input_handle(&mut self, key: KeyCode, id_origin: NodeId, node_fn:SelectedNode) {
        match key {
            KeyCode::Left => {
                self.cache.cursor -= 1;
            }
            KeyCode::Enter => {
                // Which fn I've selected?
                match node_fn{
                    SelectedNode::Drone(drone_fn) => {
                        match drone_fn{
                            3 => {
                                match self.simulation_controller_data.crash(id_origin){
                                    Ok(_) => (),
                                    Err(err) => {
                                        self.simulation_controller_data.logs.push_log(LogType::ERROR, err.as_str())
                                    }
                                }
                            },
                            _ => (),
                        }
                    }
                    SelectedNode::Client(client_fn) => {
                        match client_fn {
                            0 => {
                                // Server Type
                                // Can't find command in the backend
                                // match self.simulation_controller_data.
                                self.simulation_controller_data
                                    .logs
                                    .push_log(LogType::ERROR, "Function does NOT exist")
                            }
                            1 => {
                                // Files List
                                match self
                                    .simulation_controller_data
                                    .request_files_list(id_origin)
                                {
                                    Ok(_) => self.simulation_controller_data.logs.push_log(
                                        LogType::DEBUG,
                                        "Simulation Controller Sent Request Files List",
                                    ),
                                    Err(str) => self
                                        .simulation_controller_data
                                        .logs
                                        .push_log(LogType::ERROR, str.as_str()),
                                }
                            }
                            3 => {
                                // Media List
                                match self
                                    .simulation_controller_data
                                    .request_media_list(id_origin)
                                {
                                    Ok(_) => self.simulation_controller_data.logs.push_log(
                                        LogType::DEBUG,
                                        "Simulation Controller Sent Request Media List",
                                    ),
                                    Err(str) => self
                                        .simulation_controller_data
                                        .logs
                                        .push_log(LogType::ERROR, str.as_str()),
                                }
                            }
                            5 => {
                                // Reg to Chat
                                match self
                                    .simulation_controller_data
                                    .request_registration_to_chat(id_origin)
                                {
                                    Ok(_) => self.simulation_controller_data.logs.push_log(
                                        LogType::DEBUG,
                                        "Simulation Controller Sent Request Registration to Chat",
                                    ),
                                    Err(str) => self
                                        .simulation_controller_data
                                        .logs
                                        .push_log(LogType::ERROR, str.as_str()),
                                }
                            }
                            6 => {
                                // Client List
                                match self
                                    .simulation_controller_data
                                    .request_client_list(id_origin)
                                {
                                    Ok(_) => self.simulation_controller_data.logs.push_log(
                                        LogType::DEBUG,
                                        "Simulation Controller Sent Request Client List",
                                    ),
                                    Err(str) => self
                                        .simulation_controller_data
                                        .logs
                                        .push_log(LogType::ERROR, str.as_str()),
                                }
                            }
                            _ => (),
                        }
                    }
                    SelectedNode::Server(_) => {}
                }
            }
            _ => (),
        }
    }
    fn tab1_1input_handle(&mut self, key: KeyCode, id_origin: NodeId, node_fn: SelectedNode) {
        match key {
            KeyCode::Left => {
                self.cache.cursor -= 1;
            }
            KeyCode::Up => {
                if self.cache.tab1_data.selected_button > 0 {
                    self.cache.tab1_data.selected_button -= 1;
                }
            }
            KeyCode::Down => {
                if self.cache.tab1_data.selected_button < 1 {
                    self.cache.tab1_data.selected_button += 1;
                }
            }
            KeyCode::Enter => {
                // Se 1 -> INPUT MODE e inseriamo nel first buffer
                // Se 0 -> parse dei dati e a seconda della fn selezionata inviamo
                match self.cache.tab1_data.selected_button {
                    0 => {
                        // ENTER
                        match node_fn{
                            SelectedNode::Drone(drone_fn) => {
                                match drone_fn{
                                    0 => {
                                        // Remove Sender
                                        self.drone_remove_sender(id_origin)
                                    },
                                    1 => {
                                        // Add Sender
                                        self.drone_add_sender(id_origin)
                                    },
                                    2 => {
                                        //  Set PDR
                                        self.drone_set_pdr(id_origin)
                                    },
                                    _ => (),
                                }
                            }
                            SelectedNode::Client(client_fn) => {
                                match client_fn {
                                    2 => {
                                        // File ID requested
                                        self.request_file(id_origin);
                                    }
                                    4 => {
                                        // Media ID requested
                                        self.request_media(id_origin);
                                    }
                                    _ => (),
                                }
                            }
                            SelectedNode::Server(_) => {}
                        }

                    }
                    1 => {
                        self.app_mode = AppMode::Input;
                    }
                    _ => (),
                }
            }
            _ => (),
        }
    }
    fn tab1_2input_handle(&mut self, key: KeyCode, id_origin: NodeId, node_fn: SelectedNode) {
        match key {
            KeyCode::Left => {
                self.cache.cursor -= 1;
            }
            KeyCode::Up => {
                if self.cache.tab1_data.selected_button > 0 {
                    self.cache.tab1_data.selected_button -= 1;
                }
            }
            KeyCode::Down => {
                if self.cache.tab1_data.selected_button < 2 {
                    self.cache.tab1_data.selected_button += 1;
                }
            }
            KeyCode::Enter => {
                // Se 1 -> INPUT MODE e inseriamo nel first buffer
                // Se 2 -> INPUT MODE e inseriamo nel second buffer
                // Se 0 -> parse dei dati e a seconda della fn selezionata inviamo
                match self.cache.tab1_data.selected_button {
                    // ENTER
                    0 => {
                        match node_fn {
                            SelectedNode::Drone(drone_fn) => {}
                            SelectedNode::Client(client_fn) => {
                                match client_fn {
                                    8 => {
                                        self.parse_and_send_message(id_origin)
                                    }
                                    _ => (),
                                }
                            }
                            SelectedNode::Server(server_fn) => {}
                        }
                    }
                    1 => {
                        self.app_mode = AppMode::Input;
                    }
                    2 => {
                        self.app_mode = AppMode::Input;
                    }
                    _ => (),
                }
            }
            _ => (),
        }
    }
    fn tab1_send_output(&mut self) {
        // If for any reason parsing doesn't go through, it will just print the error on log
        // Now we have to understand which fn is currently selected
    }
    fn handle_unknown(&mut self, key: KeyCode) {
        match key {
            KeyCode::Left => {
                self.cache.cursor -= 1;
            }
            _ => (),
        }
    }
    pub(super) fn handle_event_tab2(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up => {
                if self.cache.cursor != 0 {
                    //println!("zio");
                    self.cache.cursor -= 1
                }
            }
            KeyCode::Down => {
                if self.cache.cursor != 2 {
                    //println!("zio");
                    self.cache.cursor += 1
                }
            }
            KeyCode::Enter => match self.cache.cursor {
                0 => {
                    self.app_mode = AppMode::Input;
                }
                _ => (),
            },
            _ => (),
        }
    }
    pub(super) fn handle_event_tab3(&mut self, key: KeyEvent) {}
}
enum SelectedNode { // Type of node selected, plus function id.
    Drone(u8), Client(u8), Server(u8),
}
