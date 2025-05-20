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
                    if self.simulation_controller_data.logs.log_scroll_offset > 1{
                        self.simulation_controller_data.logs.log_scroll_offset -= 2;
                    }
                }
                KeyCode::Down => {
                    self.simulation_controller_data.logs.log_scroll_offset += 2;
                }
                _ => {()}
            }
        }
    }
}