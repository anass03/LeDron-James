use std::fmt::Pointer;
use std::fs::FileType;
use log::error;
use ratatui::crossterm::event::{Event, KeyEventKind, KeyCode};
use ratatui_explorer::{File, Input};
use crate::backend::sc_models::LogType;
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
                            self.cache.tab1_data.file_exploring_data.file_request_source = 0;
                            // Reset file_explorer
                        },
                        KeyCode::Enter => {
                            // Check if he pressed a directory or a file
                            // If it is a file, then return the current file to the
                            if self.cache.file_explorer.current().is_dir(){
                                _ = self.cache.file_explorer.handle(input);
                                self.simulation_controller_data.logs.push_sc_log(LogType::ERROR, format!("Entered None 2 {:?}", self.cache.file_explorer.current()));

                            }else{
                                let selected_file = self.cache.file_explorer.current();
                                // Here I should send the request, depending if I am on Net Page and Chat Page
                                // I even should differentiate between files and media;
                                // If I press on upload media I can't upload a file!
                                match self.cache.tab1_data.file_exploring_data.file_request_source{
                                    1 => {
                                        // Chat - No need to check the type
                                        match selected_file.name().chars().position(|ch| ch=='.'){
                                            None => {
                                                // Impossible? Should not be a directory!
                                                self.simulation_controller_data.logs.push_sc_log(LogType::ERROR, format!("Entered None 2 {:?}", selected_file.name()));
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
                                                            self.simulation_controller_data.logs.push_sc_log(LogType::ERROR, format!("Successfully uploaded media dc!"));
                                                        }
                                                        Err(str) => {
                                                            //todo()! PUSH LOG
                                                            self.simulation_controller_data.logs.push_sc_log(LogType::ERROR, format!("Error sending media {:?}", str));
                                                        }
                                                    }
                                                    self.cache.tab1_data.file_exploring_data.file_request_source = 0;
                                                    self.app_mode = AppMode::Normal;

                                                }else{
                                                    // Send File
                                                    let sending_data = self.cache.tab1_data.file_exploring_data.clone();
                                                    match self.simulation_controller_data.file_for(
                                                        sending_data.last_selected_cl_sender,
                                                        sending_data.last_selected_cl_receiver,
                                                        sending_data.last_selected_sr,
                                                        selected_file.path().as_path())
                                                    {
                                                        Ok(_) => (),
                                                        Err(err) => {
                                                            error!("[LEDRON-CLI] Error sending file, {:?}", err);
                                                        }
                                                    }
                                                    self.cache.tab1_data.file_exploring_data.file_request_source = 0;
                                                    self.app_mode = AppMode::Normal;

                                                    self.simulation_controller_data.logs.push_sc_log(LogType::ERROR, format!("Entered Report Err 2 {:?}", selected_file.name()));
                                                }
                                            }
                                        }
                                    }
                                    2 => {
                                        self.simulation_controller_data.logs.push_sc_log(LogType::ERROR, format!("Entered 2 {:?}", selected_file.name()));
                                        // Net Media - Checking if it is a media
                                        match selected_file.name().chars().position(|ch| ch=='.'){
                                            None => {
                                                // Impossible? Should not be a directory!
                                                self.simulation_controller_data.logs.push_sc_log(LogType::ERROR, format!("Entered None 2 {:?}", selected_file.name()));
                                            }
                                            Some(pos) => {
                                                let ext = selected_file.name()[pos..].to_string();
                                                if ext == ".jpg" || ext == ".jpeg" || ext == ".png" {
                                                    // Send media
                                                    let sending_data = self.cache.tab1_data.file_exploring_data.clone();
                                                    match self.simulation_controller_data.new_media(
                                                        sending_data.last_selected_cl_sender,
                                                        sending_data.last_selected_sr,
                                                        selected_file.path().as_path(),
                                                    ){
                                                        Ok(_) => {
                                                            self.simulation_controller_data.logs.push_sc_log(LogType::ERROR, format!("Successfully uploaded media dc!"));
                                                        }
                                                        Err(str) => {
                                                            //todo()! PUSH LOG
                                                            self.simulation_controller_data.logs.push_sc_log(LogType::ERROR, format!("Error sending media {:?}", str));
                                                        }
                                                    }
                                                    self.cache.tab1_data.file_exploring_data.file_request_source = 0;
                                                    self.app_mode = AppMode::Normal;

                                                }else{
                                                    // Report error, I gotta check how to do it.
                                                    self.simulation_controller_data.logs.push_sc_log(LogType::ERROR, format!("Entered Report Err 2 {:?}", selected_file.name()));
                                                }
                                            }
                                        }
                                    }
                                    3 => {
                                        // Net File - Checking if it is a file
                                        match selected_file.name().chars().position(|ch| ch=='.'){
                                            None => {
                                                // Impossible? Should not be a directory!
                                                // self.simulation_controller_data.logs.push_sc_log();
                                            }
                                            Some(pos) => {
                                                let ext = selected_file.name()[pos..].to_string();
                                                if ext != ".jpg" && ext != ".jpeg" && ext != ".png" {
                                                    // Send media
                                                    let sending_data = self.cache.tab1_data.file_exploring_data.clone();
                                                    match self.simulation_controller_data.new_file(
                                                        sending_data.last_selected_cl_sender,
                                                        sending_data.last_selected_sr,
                                                        selected_file.path().as_path())
                                                    {
                                                        Ok(_) => (),
                                                        Err(err) => {
                                                            error!("[LEDRON-CLI] Error sending file, {:?}", err);
                                                        }
                                                    }
                                                    self.cache.tab1_data.file_exploring_data.file_request_source = 0;
                                                    self.app_mode = AppMode::Normal;
                                                }else{
                                                    // Report error, I gotta check how to do it.
                                                }
                                            }
                                        }
                                    }
                                    _ => {
                                        // It should never go here.
                                        self.simulation_controller_data.logs.push_sc_log(LogType::ERROR, format!("Entered Impossible {:?}", selected_file.name()));
                                    }
                                }
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