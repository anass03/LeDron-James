/*
Each input-mode related method is found here
*/
use log::{error, info, warn, debug};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use tui_textarea::{Input, TextArea};
use initializer_utils::utils::parse_and_check_config;
use crate::backend::sc_models::LogType;
use crate::frontend::app::models::general_types::TOMLMessages;
use crate::frontend::app::models::main_types::{AppMode, InputMode};
use crate::frontend::app::ui::tabs::SelectedTab;
use crate::frontend::app::models::tab3_types::{ViewedTab3, Tab3SettingsPage, NetworkSettingsCommandsPopup};
use super::super::App;
impl App<'_>{
    // Should be handled directly by the
    pub(crate) fn handle_event_inputmode(&mut self, event: Event){
        //self.update_text_buffer();
        match self.selected_tab{
            SelectedTab::Topology | SelectedTab::Settings=> {
                self.handle_topology_inputmode(event);
            }
            SelectedTab::TOMLEditor => {
                self.handle_toml_inputmode(event);
            }
            _ => {}
        }
    }
    fn handle_toml_inputmode(&mut self, event: Event){
        match event.clone(){
            // FIXA I SIMBOLI IN INPUT, COME PARENTESI QUADRE E VIRGOLE ,
            Event::Key(key) =>{
                if key.kind == KeyEventKind::Press {
                    match key.code{
                        KeyCode::Esc => {
                            // Salvo input
                            self.save_text_buffer();
                            self.app_mode = AppMode::Normal;
                            // Aggiorno il buffer_del_compiler
                            match parse_and_check_config(&self.settings.toml_path){
                                Ok(_) => {self.cache.tab2_data.compiler_buf = TOMLMessages::Correct},
                                Err(e) => {self.cache.tab2_data.compiler_buf = TOMLMessages::Error(e)}
                            }
                        },
                        KeyCode::F(1) => {
                            // Esco senza salvare
                            self.app_mode = AppMode::Normal;
                            // Aggiorno il buffer_del_compiler
                            match parse_and_check_config(&self.settings.toml_path){
                                Ok(_) => {self.cache.tab2_data.compiler_buf = TOMLMessages::Correct},
                                Err(e) => {self.cache.tab2_data.compiler_buf = TOMLMessages::Error(e)}
                            }
                        },
                        _ => {
                            let input = Input::from(event.clone());
                            match Self::map_italian_brackets(&event){
                                Some(c) => {
                                    self.cache.text_buffer.toml_text_area.insert_char(c);
                                },
                                None => {
                                    if let Event::Key(KeyEvent { code, modifiers, .. }) = event {
                                        debug!(target: "main", "[LEDRON-CLI] Pressed: {:?} + {:?}", code, modifiers);
                                    };
                                    self.cache.text_buffer.toml_text_area.input(input);
                                }
                            }
                        }
                    }
                }
            },
            _ => (),
        }
    }
    fn handle_topology_inputmode(&mut self, event: Event) {
        match &self.app_mode{
            AppMode::Input(mode) => {
                match mode{
                    InputMode::One => {
                        match event{
                            Event::Key(key) => {
                                if key.kind == KeyEventKind::Press {
                                    match key.code {
                                        KeyCode::Esc  => {
                                            // Exit input mode and return to previous state
                                            self.app_mode = AppMode::Normal;
                                        },
                                        KeyCode::F(1) => {
                                            self.cache.text_buffer.clean_topology_inputs_buffer();
                                            self.app_mode = AppMode::Normal;
                                        },
                                        _ => {
                                            let input = Input::from(event.clone());
                                            match Self::map_italian_brackets(&event){
                                                Some(c) => {
                                                    self.cache.text_buffer.input_first_text_area.insert_char(c);
                                                },
                                                None => {
                                                    self.cache.text_buffer.input_first_text_area.input(input);
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            _ => ()
                        }

                    }
                    InputMode::Two(id, chthird) => {
                        // Chat message input mode
                        match event {
                            Event::Key(key) => {
                                if key.kind == KeyEventKind::Press {
                                    match key.code {
                                        KeyCode::Esc => {
                                            // Exit chat input mode and return to chat view
                                            self.app_mode = AppMode::Normal;
                                        }
                                        KeyCode::Enter => {
                                            if key.modifiers.contains(KeyModifiers::ALT) {
                                                // Option + Enter: Add new line
                                                self.cache.text_buffer.chat_text_area.insert_char('\n');
                                            } else {
                                                // Enter: Send message
                                                let message_text = self.cache.text_buffer.chat_text_area.lines().join("\n");
                                                if !message_text.trim().is_empty() {
                                                    // Send the message
                                                    match self.parse_and_send_message(*id,chthird.selected_contact,chthird.selected_server){
                                                        Ok(_) => {
                                                            // Exit input mode and return to chat view
                                                            self.app_mode = AppMode::Normal;
                                                        }
                                                        Err(err) => {
                                                            // Log error and stay in input mode
                                                            error!(target: "main", "[LEDRON-CLI] Failed to send chat message: {}", err);
                                                        }
                                                    }
                                                } else {
                                                    // Empty message, just exit input mode
                                                    self.app_mode = AppMode::Normal;
                                                }
                                            }
                                        }
                                        _ => {
                                            let input = Input::from(event.clone());
                                            match Self::map_italian_brackets(&event) {
                                                Some(c) => {
                                                    self.cache.text_buffer.chat_text_area.insert_char(c);
                                                }
                                                None => {
                                                    self.cache.text_buffer.chat_text_area.input(input);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            _ => ()
                        }
                    }
                    _ => {
                        match event{
                            Event::Key(key) => {
                                if key.kind == KeyEventKind::Press {
                                    match key.code {
                                        KeyCode::Esc  => {
                                            // Salvo input
                                            self.app_mode = AppMode::Normal;
                                        },
                                        KeyCode::F(1) => {
                                            self.cache.text_buffer.clean_topology_inputs_buffer();
                                            self.app_mode = AppMode::Normal;
                                        },

                                        _ => {
                                            let input = Input::from(event.clone());
                                            match Self::map_italian_brackets(&event){
                                                Some(c) => {
                                                    self.cache.text_buffer.input_first_text_area.insert_char(c);
                                                },
                                                None => {
                                                    if let Event::Key(KeyEvent { code, modifiers, .. }) = event {
                                                        debug!(target: "main", "[LEDRON-CLI] Pressed: {:?} + {:?}", code, modifiers);
                                                    };
                                                    self.cache.text_buffer.input_first_text_area.input(input);
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            _ => ()
                        }
                    }
                }
            }
            _ => (),
        }
    }
    fn map_italian_brackets(event: &Event) -> Option<char> {
        if let Event::Key(KeyEvent { code, modifiers, .. }) = event {
            match (code, modifiers) {
                // AltGr is typically represented as Control + Alt
                (KeyCode::Char('è'), &KeyModifiers::CONTROL | &KeyModifiers::ALT) => Some('['),
                (KeyCode::Char('ȉ'), &KeyModifiers::ALT) => Some(']'),
                _ => None,
            }
        } else {
            None
        }
    }
}