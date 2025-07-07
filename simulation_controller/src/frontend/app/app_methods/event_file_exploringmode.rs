use std::fmt::Pointer;
use std::fs::FileType;
use std::path::PathBuf;
use log::{error, info};
use ratatui::crossterm::event::{Event, KeyEventKind, KeyCode};
use ratatui_explorer::{File, Input};
use crate::backend::sc_models::LogType;
use crate::frontend::app::App;
use crate::frontend::app::models::main_types::AppMode;

impl App<'_> {
    pub(crate) fn handle_event_file_exploringmode(&mut self, event: Event) {
        let input = Input::from(&event);
        match event.clone() {
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Esc => {
                            // Quit file exploring without nothing
                            self.app_mode = AppMode::Normal;
                            self.cache.tab1_data.file_exploring_data.file_request_source = 0;
                            // Reset tab3 browsing flag
                            self.cache.tab3_data.browsing_for_download_folder = false;
                        },
                        KeyCode::Enter => {
                            // Check if we're browsing for tab3 download folder
                            if self.cache.tab3_data.browsing_for_download_folder {
                                let selected_file = self.cache.file_explorer.current();

                                if selected_file.is_dir() {
                                    // If it's a directory and we're browsing for download folder, select it
                                    let folder_path = selected_file.path().to_path_buf();
                                    self.handle_download_folder_selected(folder_path);
                                } else {
                                    // If they clicked on a file, navigate into its parent directory or show error
                                    _ = self.cache.file_explorer.handle(input);
                                }
                            } else {
                                // Handle regular file explorer for tab1
                                let selected_file = self.cache.file_explorer.current();
                                if selected_file.is_file() {
                                    // Handle file selection for tab1
                                    self.handle_file_selection_for_tab1(selected_file.path().to_path_buf());
                                } else if selected_file.is_dir() {
                                    // Navigate into directory
                                    _ = self.cache.file_explorer.handle(input);
                                }
                            }
                        },
                        _ => {
                            // Handle other navigation keys
                            _ = self.cache.file_explorer.handle(input);
                        }
                    }
                }
            },
            _ => {
                // Handle other events (mouse, etc.)
                _ = self.cache.file_explorer.handle(input);
            }
        }
    }

    fn handle_file_selection_for_tab1(&mut self, path: PathBuf) {
        // Handle file selection for tab1 (existing logic)
        // This is the original file selection logic for tab1
        match self.cache.tab1_data.file_exploring_data.file_request_source {
            0 => {
                // No specific source, just navigate
                _ = self.cache.file_explorer.handle(Input::from(&Event::Key(ratatui::crossterm::event::KeyEvent::new(
                    KeyCode::Enter,
                    ratatui::crossterm::event::KeyModifiers::empty(),
                ))));
            }
            1 => {
                // File attachment for chat
                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown_file")
                    .to_string();
                
                info!(target: "main", "[LEDRON-CLI] File selected for chat: {}", file_name);
                
                // TODO: Implement file attachment logic for chat
                self.app_mode = AppMode::Normal;
                self.cache.tab1_data.file_exploring_data.file_request_source = 0;
            }
            _ => {
                // Unknown source, just navigate
                _ = self.cache.file_explorer.handle(Input::from(&Event::Key(ratatui::crossterm::event::KeyEvent::new(
                    KeyCode::Enter,
                    ratatui::crossterm::event::KeyModifiers::empty(),
                ))));
            }
        }
    }
}