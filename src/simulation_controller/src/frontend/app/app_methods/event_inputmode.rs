/*
Each input-mode related method is found here
*/
use ratatui::crossterm::event::{KeyEvent, KeyEventKind, KeyCode, Event};
use tui_textarea::{Input, TextArea};
use initializer_utils::utils::{check_config, parse_config};
use crate::frontend::app::model::TOMLmessages;
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
                            self.input_mode = false;
                            // Aggiorno il buffer_del_compiler
                            match check_config(&parse_config(self.settings.toml_path.as_str())){
                                Ok(_) => {self.cache.tab2_data.compiler_buf = TOMLmessages::Correct},
                                Err(e) => {self.cache.tab2_data.compiler_buf = TOMLmessages::Error(e)}
                            }
                        },
                        KeyCode::F(1) => {
                            // Esco senza salvare
                            self.input_mode = false;
                            // Aggiorno il buffer_del_compiler
                            match check_config(&parse_config(self.settings.toml_path.as_str())){
                                Ok(_) => {self.cache.tab2_data.compiler_buf = TOMLmessages::Correct},
                                Err(e) => {self.cache.tab2_data.compiler_buf = TOMLmessages::Error(e)}
                            }
                        },
                        _ => {
                            let input = Input::from(event);
                            self.cache.text_buffer.text_area.input(input);
                        }
                    }
                }
            },
            _ => (),
        }
    }
}