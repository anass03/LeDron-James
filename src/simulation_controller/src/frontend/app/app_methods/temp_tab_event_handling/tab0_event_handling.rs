use ratatui::crossterm::event::KeyCode;
use crate::frontend::app::App;
use crate::frontend::app::models::main_types::AppMode;

impl App<'_> {
    pub(in super::super) fn handle_event_tab0(&mut self, key: KeyCode) {
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
}