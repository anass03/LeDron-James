use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use crate::frontend::app::model::AppMode;
use super::super::App;
use super::super::ui;
impl App<'_>{
    pub(crate) fn handle_event_scrollingmode(&mut self, key: KeyEvent){
        if key.kind == KeyEventKind::Press{
            match key.code{
                KeyCode::Esc => {
                    self.app_mode = AppMode::Normal;
                },
                KeyCode::Up => {
                    if self.cache.tab0_data.log_scroll_offset > 0{
                        self.cache.tab0_data.log_scroll_offset -= 1;
                    }
                }
                KeyCode::Down => {
                    if self.cache.tab0_data.log_scroll_offset < self.cache.tab0_data.logs_memory.len(){
                        self.cache.tab0_data.log_scroll_offset += 1;
                    }
                }
                _ => {()}
            }
        }
    }
}