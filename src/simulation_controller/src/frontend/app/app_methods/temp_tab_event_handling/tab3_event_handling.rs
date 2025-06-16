use ratatui::crossterm::event::KeyCode;
use crate::frontend::app::App;

impl App<'_> {
    pub(crate) fn handle_event_tab3(&mut self, key: KeyCode) {
        match key{
            KeyCode::Up => {
                if self.simulation_controller_data.chat_cache.first_menu_scroll_offset!= 0{
                    self.simulation_controller_data.chat_cache.first_menu_scroll_offset -= 1;
                }
            }
            KeyCode::Down => {
                self.simulation_controller_data.chat_cache.first_menu_scroll_offset += 1;
            }
            _ => (),
        }
    }
}