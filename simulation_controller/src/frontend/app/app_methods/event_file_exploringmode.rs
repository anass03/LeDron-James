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
                                    let folder_path = selected_file.path().to_string_lossy().to_string();
                                    self.simulation_controller_data.download_folder = PathBuf::from(folder_path.clone());
                                    self.cache.tab3_data.browsing_for_download_folder = false;
                                    self.app_mode = AppMode::Normal;

                                    info!(target: "main", "[LEDRON-CLI] Download folder set to: {}", folder_path)
                                } else {
                                    // If they clicked on a file, navigate into its parent directory or show error
                                    _ = self.cache.file_explorer.handle(input);
                                }
                            } else {
                                // Original tab1 file exploring logic
                                if self.cache.file_explorer.current().is_dir() {
                                    _ = self.cache.file_explorer.handle(input);
                                    error!(target: "main", "[LEDRON-CLI] Entered directory: {:?}", self.cache.file_explorer.current());
                                } else {
                                    let selected_file = self.cache.file_explorer.current();
                                    // Here I should send the request, depending if I am on Net Page and Chat Page
                                    // I even should differentiate between files and media;
                                    // If I press on upload media I can't upload a file!
                                    match self.cache.tab1_data.file_exploring_data.file_request_source {
                                        1 => {
                                            // Chat - No need to check the type
                                            match selected_file.name().chars().position(|ch| ch == '.') {
                                                None => {
                                                    // Impossible? Should not be a directory!
                                                    error!(target: "main", "[LEDRON-CLI] Entered None 1 {:?}", selected_file.name())
                                                }
                                                Some(pos) => {
                                                    let ext = selected_file.name()[pos..].to_string();
                                                    if ext == ".jpg" || ext == ".jpeg" || ext == ".png" {
                                                        // Send media
                                                        let sending_data = self.cache.tab1_data.file_exploring_data.clone();
                                                        match self.simulation_controller_data.media_for(
                                                            sending_data.last_selected_cl_sender,
                                                            sending_data.last_selected_cl_receiver,
                                                            sending_data.last_selected_sr,
                                                            selected_file.path().as_path())
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
                                                            selected_file.path().as_path())
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
                                        }
                                        2 => {
                                            // Net Media - Checking if it is a media
                                            match selected_file.name().chars().position(|ch| ch == '.') {
                                                None => (),
                                                Some(pos) => {
                                                    let ext = selected_file.name()[pos..].to_string();
                                                    if ext == ".jpg" || ext == ".jpeg" || ext == ".png" {
                                                        // Send media
                                                        let sending_data = self.cache.tab1_data.file_exploring_data.clone();
                                                        match self.simulation_controller_data.new_media(
                                                            sending_data.last_selected_cl_sender,
                                                            sending_data.last_selected_sr,
                                                            selected_file.path().as_path(),
                                                        ) {
                                                            Ok(_) => {
                                                                info!(target: "main", "[LEDRON-CLI] Successfully uploaded media!");
                                                            }
                                                            Err(str) => {
                                                                error!(target: "main", "[LEDRON-CLI] Error sending media {:?}", str);
                                                            }
                                                        }
                                                        self.cache.tab1_data.file_exploring_data.file_request_source = 0;
                                                        self.app_mode = AppMode::Normal;
                                                    } else {
                                                        // Report error, I got to check how to do it.
                                                        error!(target: "main", "[LEDRON-CLI] Invalid media format. Only .jpg, .jpeg, .png are supported");
                                                    }
                                                }
                                            }
                                        }
                                        3 => {
                                            // Net File - Any file type is acceptable
                                            let sending_data = self.cache.tab1_data.file_exploring_data.clone();
                                            match self.simulation_controller_data.new_file(
                                                sending_data.last_selected_cl_sender,
                                                sending_data.last_selected_sr,
                                                selected_file.path().as_path(),
                                            ) {
                                                Ok(_) => {
                                                    info!(target: "main", "[LEDRON-CLI] Successfully uploaded file!");
                                                }
                                                Err(str) => {
                                                    error!(target: "main", "[LEDRON-CLI] Error sending file {:?}", str);
                                                }
                                            }
                                            self.cache.tab1_data.file_exploring_data.file_request_source = 0;
                                            self.app_mode = AppMode::Normal;
                                        }
                                        _ => {
                                            // It should never go here.
                                            error!(target: "main", "[LEDRON-CLI] Invalid file request source");
                                        }
                                    }
                                }
                            }
                        },
                        _ => _ = self.cache.file_explorer.handle(input),
                    }
                }
            }
             _ => {}
        }
    }
}