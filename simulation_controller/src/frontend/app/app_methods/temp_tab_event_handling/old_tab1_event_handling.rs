use ratatui::crossterm::event::KeyCode;
use wg_2024::network::NodeId;
use crate::backend::sc_models::LogType;
use crate::frontend::app::App;
use crate::frontend::app::models::general_types::Node;
use crate::frontend::app::models::main_types::AppMode;

impl App<'_> {
    /*pub(crate) fn handle_event_old_tab1(&mut self, key: KeyCode) {
        match self.cache.cursor {
            0 => {
                match key {
                    KeyCode::Up => {
                        self.cache.tab1_data.netnode_list.select_prev();
                        self.cache.tab1_data.selected_fn.select_first();
                        self.cache.tab1_data.popup_cursor = 0;
                    }
                    KeyCode::Down => {
                        self.cache.tab1_data.netnode_list.select_next();
                        self.cache.tab1_data.selected_fn.select_first();
                        self.cache.tab1_data.popup_cursor = 0;
                    }
                    KeyCode::Right | KeyCode::Enter => {
                        self.cache.cursor += 1;
                    }

                    // Missing KeyCodes to view the first one / last one, still gotta decide!
                    _ => (), // Do nothing
                }
            }
            1 => {
                let selected_n = self.cache.tab1_data.netnode_list.selected;
                match self.cache.tab1_data.netnode_list.items.get(selected_n as usize) {
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
                                                self.cache.tab1_data.popup_cursor = 0;
                                            }
                                        }
                                        KeyCode::Down => {
                                            if self.cache.tab1_data.selected_fn.selected + 1 < 8 {
                                                self.cache.tab1_data.selected_fn.select_next();
                                                self.cache.tab1_data.popup_cursor = 0;
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
                                                self.cache.tab1_data.popup_cursor = 0;
                                            }
                                        }
                                        KeyCode::Down => {
                                            if self.cache.tab1_data.selected_fn.selected + 1 < 4 {
                                                self.cache.tab1_data.selected_fn.select_next();
                                                self.cache.tab1_data.popup_cursor = 0;
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
                let selected_n = self.cache.tab1_data.netnode_list.selected;
                match self.cache.tab1_data.netnode_list.items.get(selected_n as usize) {
                    None => {
                        // unreachable
                    }
                    Some((n, _)) => {
                        match n {
                            Node::Client(c) => {
                                let id = c.id;
                                /*
                                                        0 => {
                            render_no_input_fn(app, area, buf);
                        }
                        1 | 3 | 5 | 6 => {
                            render_one_input_fn(app, area, buf);
                        }
                        2 | 4 => {
                            render_two_input_fn(app, area, buf);
                        }
                        7 => {
                            render_three_input_fn(app, area, buf);
                        }
                                     */
                                match self.cache.tab1_data.selected_fn.selected {
                                    0 => self.tab1_0input_handle(
                                        key,
                                        id,
                                        SelectedNode::Client(self.cache.tab1_data.selected_fn.selected),
                                    ),
                                    1 | 3 | 5 | 6 => self.tab1_1input_handle(
                                        key,
                                        id,
                                        SelectedNode::Client(self.cache.tab1_data.selected_fn.selected),
                                    ),
                                    2 | 4 => self.tab1_2input_handle(
                                        key,
                                        id,
                                        SelectedNode::Client(self.cache.tab1_data.selected_fn.selected),
                                    ),
                                    7 => self.tab1_3input_handle(
                                        key,
                                        id,
                                        SelectedNode::Client(self.cache.tab1_data.selected_fn.selected)
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
    }*/
    /*fn tab1_0input_handle(&mut self, key: KeyCode, id_origin: NodeId, node_fn:SelectedNode) {
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
                                    Ok(_) => self.simulation_controller_data.logs.push_sc_log(LogType::DEBUG, format!("Sent Crash command to {:?}", id_origin)),
                                    Err(err) => {
                                        self.simulation_controller_data.logs.push_sc_log(LogType::ERROR, err)
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
                                match self.simulation_controller_data.request_server_type(id_origin){
                                    Ok(_) => self.simulation_controller_data.logs.push_sc_log(
                                        LogType::DEBUG,
                                        "Sent Request Server Type".to_string(),
                                    ),
                                    Err(str) => self
                                        .simulation_controller_data
                                        .logs
                                        .push_log(LogType::ERROR, str.as_str()),
                                }
                            }
                            /*0 => {
                            render_no_input_fn(app, area, buf);
                        }
                        1 | 3 | 5 | 6 => {
                            render_one_input_fn(app, area, buf);
                        }
                        2 | 4 => {
                            render_two_input_fn(app, area, buf);
                        }
                        7 => {
                            render_three_input_fn(app, area, buf);
                        }
                                     */
                            /*    1 => {
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
                                }*/
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
                if self.cache.tab1_data.popup_cursor > 0 {
                    self.cache.tab1_data.popup_cursor -= 1;
                }
            }
            KeyCode::Down => {
                if self.cache.tab1_data.popup_cursor < 1 {
                    self.cache.tab1_data.popup_cursor += 1;
                }
            }
            KeyCode::Enter => {
                // Se 1 -> INPUT MODE e inseriamo nel first buffer
                // Se 0 -> parse dei dati e a seconda della fn selezionata inviamo
                match self.cache.tab1_data.popup_cursor {
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
                                    1 =>{
                                        self.request_file_list(id_origin)
                                    }
                                    3 => {
                                        self.request_media_list(id_origin)
                                    }
                                    5 => {
                                        // Media ID requested
                                        self.request_reg_to_chat(id_origin);
                                    }
                                    6 => {
                                        self.request_client_list(id_origin);
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
    }*/
    /*fn tab1_2input_handle(&mut self, key: KeyCode, id_origin: NodeId, node_fn: SelectedNode) {
        match key {
            KeyCode::Left => {
                self.cache.cursor -= 1;
            }
            KeyCode::Up => {
                if self.cache.tab1_data.popup_cursor > 0 {
                    self.cache.tab1_data.popup_cursor -= 1;
                }
            }
            KeyCode::Down => {
                if self.cache.tab1_data.popup_cursor < 2 {
                    self.cache.tab1_data.popup_cursor += 1;
                }
            }
            KeyCode::Enter => {
                // Se 1 -> INPUT MODE e inseriamo nel first buffer
                // Se 2 -> INPUT MODE e inseriamo nel second buffer
                // Se 0 -> parse dei dati e a seconda della fn selezionata inviamo
                match self.cache.tab1_data.popup_cursor {
                    // ENTER
                    0 => {
                        match node_fn {
                            SelectedNode::Drone(drone_fn) => {}
                            SelectedNode::Client(client_fn) => {
                                match client_fn {
                                    2=> {
                                        self.request_file(id_origin);
                                    }
                                    4 => {
                                        // Media ID requested
                                        self.request_media(id_origin);
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
    fn tab1_3input_handle(&mut self, key: KeyCode, id_origin: NodeId, node_fn: SelectedNode) {
        match key {
            KeyCode::Left => {
                self.cache.cursor -= 1;
            }
            KeyCode::Up => {
                if self.cache.tab1_data.popup_cursor > 0 {
                    self.cache.tab1_data.popup_cursor -= 1;
                }
            }
            KeyCode::Down => {
                if self.cache.tab1_data.popup_cursor < 3 {
                    self.cache.tab1_data.popup_cursor += 1;
                }
            }
            KeyCode::Enter => {
                // Se 1 -> INPUT MODE e inseriamo nel first buffer
                // Se 2 -> INPUT MODE e inseriamo nel second buffer
                // Se 3 -> INPUT MODE e inseriamo nel terzo buffer
                // Se 0 -> parse dei dati e a seconda della fn selezionata inviamo
                match self.cache.tab1_data.popup_cursor {
                    // ENTER
                    0 => {
                        match node_fn {
                            SelectedNode::Drone(drone_fn) => {}
                            SelectedNode::Client(client_fn) => {
                                match client_fn {
                                    7 => {
                                        self.parse_and_send_message_old(id_origin)
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
                    3 => {
                        self.app_mode = AppMode::Input;
                    }
                    _ => (),
                }
            }
            _ => (),
        }
    }*/
    fn handle_unknown(&mut self, key: KeyCode) {
        match key {
            KeyCode::Left => {
                self.cache.cursor -= 1;
            }
            _ => (),
        }
    }

}
pub(super) enum SelectedNode { // Type of node selected, plus function id.
    Drone(u8), Client(u8), Server(u8),
}