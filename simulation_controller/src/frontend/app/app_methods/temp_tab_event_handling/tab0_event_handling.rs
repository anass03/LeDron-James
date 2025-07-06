use ratatui::crossterm::event::KeyCode;
use crate::frontend::app::App;
use crate::frontend::app::models::main_types::AppMode;

impl App<'_> {
    pub(in super::super) fn handle_event_tab0(&mut self, key: KeyCode) {
        match key {
            KeyCode::Backspace => {
                // Backspace at main level does nothing (could be used for other purposes)
                // For now, just log that it was pressed
                self.simulation_controller_data.logs.push_sc_log(
                    crate::backend::sc_models::LogType::INFO,
                    "Backspace pressed at main level".to_string()
                );
            }
            KeyCode::Up => {
                if self.cache.cursor > 0 && self.cache.cursor < 3{
                    self.cache.cursor -= 1
                }
            }
            KeyCode::Down => {
                if self.cache.cursor < 2{
                    //println!("zio");
                    self.cache.cursor += 1
                }else if self.cache.cursor == 2{
                    self.cache.cursor = 0;
                }
            }
            KeyCode::Left => {
                if self.cache.cursor == 3 {
                    //println!("zio");
                    self.cache.cursor = 1
                }else{
                    self.cache.cursor = 3;
                }
            }
            KeyCode::Right => {
                if self.cache.cursor != 3 {
                    //println!("zio");
                    self.cache.cursor = 3
                }else{
                    self.cache.cursor = 1
                }
            }
            KeyCode::Enter => {
                match self.cache.cursor {
                    0 => {}
                    3 => {
                        self.app_mode = AppMode::Scrolling;
                    }
                    _ => (),
                }
            }
            _ => (),
        }
    }
}