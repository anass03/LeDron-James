use ratatui::crossterm::event::KeyCode;
use crate::frontend::app::App;

impl App<'_> {
    pub(crate) fn handle_event_tab3(&mut self, key: KeyCode) {
        match key{
            KeyCode::Up => {
                if self.cache.tab1_data.data_chat.first_menu_scroll_offset!= 0{
                    self.cache.tab1_data.data_chat.first_menu_scroll_offset -= 1;
                }
            }
            KeyCode::Down => {
                self.cache.tab1_data.data_chat.first_menu_scroll_offset += 1;
            }
            _ => (),
        }
    }
}