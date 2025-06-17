use ratatui::crossterm::event::{Event, KeyEventKind, KeyCode};
use ratatui_explorer::{File, Input};
use crate::frontend::app::App;
use crate::frontend::app::models::main_types::AppMode;

impl App<'_> {
    pub(crate) fn handle_event_file_exploringmode(&mut self, event: Event) {
        let input = Input::from(&event);
        match event.clone(){
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Esc => {
                            // Quit file exploring without nothing
                            self.app_mode = AppMode::Normal;
                        },
                        KeyCode::Enter => {
                            // Check if he pressed a directory or a file
                            // If it is a file, then return the current file to the
                            if self.cache.file_explorer.current().is_dir(){
                                self.cache.file_explorer.handle(input);
                            }
                            if self.cache.file_explorer.current().is_file(){
                                let selected_file = self.cache.file_explorer.current();
                            }
                        }
                        _ => _ = self.cache.file_explorer.handle(input),
                    }
                }
            },
            _ => (),
        }
    }
}