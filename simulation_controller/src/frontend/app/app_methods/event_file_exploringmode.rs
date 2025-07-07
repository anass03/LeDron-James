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

                match file_name.chars().position(|ch| ch == '.') {
                    None => (),
                    Some(pos) => {
                        let ext = file_name[pos..].to_string();
                        info!(target: "main", "[LEDRON-CLI] File selected for chat: {}", file_name);

                        if ext == ".jpg" || ext == ".jpeg" || ext == ".png" {
                            // Send media
                            let sending_data = self.cache.tab1_data.file_exploring_data.clone();
                            match self.simulation_controller_data.media_for(
                                sending_data.last_selected_cl_sender,
                                sending_data.last_selected_cl_receiver,
                                sending_data.last_selected_sr,
                                path)
                            {
                                Ok(_) => {
                                    info!(target:"main", "[LEDRON-CLI] Sent media-for request")
                                }
                                Err(str) => {
                                    error!(target:"main", "[LEDRON-CLI] Error sending media-for request:\n{}", str)
                                }
                            }
                            self.cache.tab1_data.file_exploring_data.file_request_source = 0;
                            self.app_mode = AppMode::Normal;
                        } else {
                            // Send file
                            let sending_data = self.cache.tab1_data.file_exploring_data.clone();
                            match self.simulation_controller_data.file_for(
                                sending_data.last_selected_cl_sender,
                                sending_data.last_selected_cl_receiver,
                                sending_data.last_selected_sr,
                                path)
                            {
                                Ok(_) => {
                                    info!(target:"main", "[LEDRON-CLI] Sent file-for request")
                                }
                                Err(str) => {
                                    error!(target:"main", "[LEDRON-CLI] Error sending file-for request:\n{}", str)
                                }
                            }
                            self.cache.tab1_data.file_exploring_data.file_request_source = 0;
                            self.app_mode = AppMode::Normal;
                        }
                    }
                }



                
                // TODO: Implement file attachment logic for chat
                self.app_mode = AppMode::Normal;
                self.cache.tab1_data.file_exploring_data.file_request_source = 0;
            }
            2 => {
                // NetPage Media upload
                let selected_file = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown_file")
                    .to_string();

                info!(target: "main", "[LEDRON-CLI] Media file selected for upload: {}", selected_file);
                match selected_file.chars().position(|ch| ch == '.') {
                    None => (),
                    Some(pos) => {
                        let ext = selected_file[pos..].to_string();
                        if ext == ".jpg" || ext == ".jpeg" || ext == ".png" {
                            // Send media
                            let sending_data = self.cache.tab1_data.file_exploring_data.clone();
                            match self.simulation_controller_data.new_media(
                                sending_data.last_selected_cl_sender,
                                sending_data.last_selected_sr,
                                path,
                            ) {
                                Ok(_) => info!(target:"main", "[LEDRON-CLI] Sent new-media request"),
                                Err(str) => error!(target: "main", "{}", str)
                            }
                            self.cache.tab1_data.file_exploring_data.file_request_source = 0;
                            self.app_mode = AppMode::Normal;
                        } else {}
                    }
                }

                self.app_mode = AppMode::Normal;
                self.cache.tab1_data.file_exploring_data.file_request_source = 0;
            }
            3 => {
                // NetPage Media upload
                let selected_file = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown_file")
                    .to_string();

                info!(target: "main", "[LEDRON-CLI] Media file selected for upload: {}", selected_file);
                match selected_file.chars().position(|ch| ch == '.') {
                    None => (),
                    Some(pos) => {
                        let ext = selected_file[pos..].to_string();
                        if !(ext == ".jpg" || ext == ".jpeg" || ext == ".png") {
                            // Send file
                            let sending_data = self.cache.tab1_data.file_exploring_data.clone();
                            match self.simulation_controller_data.new_file(
                                sending_data.last_selected_cl_sender,
                                sending_data.last_selected_sr,
                                path,
                            ) {
                                Ok(_) => info!(target:"main", "[LEDRON-CLI] Sent new-file request"),
                                Err(str) => error!(target: "main", "{}", str)
                            }
                            self.cache.tab1_data.file_exploring_data.file_request_source = 0;
                            self.app_mode = AppMode::Normal;
                        } else {}
                    }
                }
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