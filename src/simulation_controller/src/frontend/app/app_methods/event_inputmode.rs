/*
Each input-mode related method is found here
*/
use ratatui::crossterm::event::{KeyEvent, KeyEventKind, KeyCode, Event, KeyModifiers};
use tui_textarea::{Input, TextArea};
use initializer_utils::utils::parse_and_check_config;
use crate::frontend::app::model::{AppMode, LogType, TOMLmessages};
use super::super::App;
impl App<'_>{
    // Should be handled directly by the
    pub(crate) fn handle_event_inputmode(&mut self, event: Event){
        //self.update_text_buffer();
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
                                    self.cache.text_buffer.text_area.insert_char(c);
                                },
                                None => {
                                    if let Event::Key(KeyEvent { code, modifiers, .. }) = event {
                                        self.cache.tab0_data.push_log(LogType::DEBUG, format!("Pressed: {:?} + {:?}", code, modifiers).as_str());
                                    };
                                    self.cache.text_buffer.text_area.input(input);
                                }
                            }
                        }
                    }
                }
            },
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