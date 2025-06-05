/*
Each input-mode related method is found here
*/
use ratatui::crossterm::event::{KeyEvent, KeyEventKind, KeyCode, Event, KeyModifiers};
use tui_textarea::{Input, TextArea};
use initializer_utils::utils::parse_and_check_config;
use crate::backend::sc_models::LogType;
use crate::frontend::app::model::{AppMode, TOMLmessages};
use crate::frontend::app::ui::tabs::SelectedTab;
use super::super::App;
impl App<'_>{
    // Should be handled directly by the
    pub(crate) fn handle_event_inputmode(&mut self, event: Event){
        //self.update_text_buffer();
        match self.selected_tab{
            SelectedTab::Topology => {
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
                                Ok(_) => {self.cache.tab2_data.compiler_buf = TOMLmessages::Correct},
                                Err(e) => {self.cache.tab2_data.compiler_buf = TOMLmessages::Error(e)}
                            }
                        },
                        KeyCode::F(1) => {
                            // Esco senza salvare
                            self.app_mode = AppMode::Normal;
                            // Aggiorno il buffer_del_compiler
                            match parse_and_check_config(&self.settings.toml_path){
                                Ok(_) => {self.cache.tab2_data.compiler_buf = TOMLmessages::Correct},
                                Err(e) => {self.cache.tab2_data.compiler_buf = TOMLmessages::Error(e)}
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
                                        self.simulation_controller_data.logs.push_log(LogType::DEBUG, format!("Pressed: {:?} + {:?}", code, modifiers).as_str());
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
        // I gotta understand which field is currently selected
        match event{
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Esc | KeyCode::Enter => {
                            // Salvo input
                            self.app_mode = AppMode::Normal;
                        },
                        KeyCode::F(1) => {
                            self.clean_topology_inputs_buffer();
                            self.app_mode = AppMode::Normal;
                        },

                        _ => {
                            let input = Input::from(event.clone());
                            match Self::map_italian_brackets(&event){
                                Some(c) => {
                                    match self.cache.tab1_data.selected_button{
                                        1 => self.cache.text_buffer.inputfirst_text_area.insert_char(c),
                                        2 => self.cache.text_buffer.inputsecond_text_area.insert_char(c),
                                        3 => self.cache.text_buffer.inputthird_text_area.insert_char(c),
                                        _ => ()
                                    }
                                    self.cache.text_buffer.toml_text_area.insert_char(c);
                                },
                                None => {
                                    if let Event::Key(KeyEvent { code, modifiers, .. }) = event {
                                        self.simulation_controller_data.logs.push_log(LogType::DEBUG, format!("Pressed: {:?} + {:?}", code, modifiers).as_str());
                                    };
                                    match self.cache.tab1_data.selected_button{
                                        1 => {self.cache.text_buffer.inputfirst_text_area.input(input);},
                                        2 => {self.cache.text_buffer.inputsecond_text_area.input(input);},
                                        3 => {self.cache.text_buffer.inputthird_text_area.input(input);},
                                        _ => ()
                                    }
                                }
                            }
                        }
                    }
                }
            },
            _ => ()
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