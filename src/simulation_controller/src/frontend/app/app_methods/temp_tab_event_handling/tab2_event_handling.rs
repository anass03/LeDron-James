use crate::frontend::app::App;
use ratatui::crossterm::event::KeyCode;
use crate::frontend::app::models::main_types::AppMode;

impl App<'_> {
    pub(crate) fn handle_event_tab2(&mut self, key: KeyCode) {
        match key {
            KeyCode::Left => {
                if self.cache.cursor > 0 {
                    //println!("zio");
                    self.cache.cursor -= 1
                }
            }
            KeyCode::Right => {
                if self.cache.cursor + 1 < 2 {
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
}