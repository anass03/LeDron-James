use super::super::ui;
use super::super::App;
use crate::frontend::app::models::main_types::AppMode;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use tui_logger::TuiWidgetEvent;
impl App<'_> {
    pub(crate) fn handle_event_scrollingmode(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Esc => {
                    self.app_mode = AppMode::Normal;
                    self.cache
                        .main_log_state
                        .1
                        .transition(TuiWidgetEvent::EscapeKey)
                }
                KeyCode::Up => self
                    .cache
                    .main_log_state
                    .1
                    .transition(TuiWidgetEvent::PrevPageKey),
                KeyCode::Down => self
                    .cache
                    .main_log_state
                    .1
                    .transition(TuiWidgetEvent::NextPageKey),
                _ => (),
            }
        }
    }
}
