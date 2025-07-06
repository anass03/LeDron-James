use ratatui::crossterm::event::KeyCode;
use crate::frontend::app::App;
use crate::frontend::app::models::tab3_types::{
    Tab3MenuSelection, ViewedTab3, Tab3SettingsPage, 
    DownloadSettingsItems, LoggingSettingsItems, NetworkSettingsItems, AppSettingsItems,
    LogLevel
};
use crate::frontend::app::models::main_types::AppMode;

impl App<'_> {
    pub(crate) fn handle_event_tab3(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up => {
                match self.cache.tab3_data.menu_selection {
                    Tab3MenuSelection::MenuArea => {
                        self.cache.tab3_data.menu_up();
                    }
                    Tab3MenuSelection::OptArea => {
                        self.cache.tab3_data.opt_up();
                    }
                }
            }
            KeyCode::Down => {
                match self.cache.tab3_data.menu_selection {
                    Tab3MenuSelection::MenuArea => {
                        self.cache.tab3_data.menu_down();
                    }
                    Tab3MenuSelection::OptArea => {
                        self.cache.tab3_data.opt_down();
                    }
                }
            }
            KeyCode::Left => {
                // Move from OptArea to MenuArea
                if self.cache.tab3_data.menu_selection == Tab3MenuSelection::OptArea {
                    self.cache.tab3_data.menu_selection = Tab3MenuSelection::MenuArea;
                }
            }
            KeyCode::Right => {
                // Move from MenuArea to OptArea (only if we're in a settings page)
                if self.cache.tab3_data.menu_selection == Tab3MenuSelection::MenuArea {
                    if let ViewedTab3::SettingsPage(_) = self.cache.tab3_data.viewing {
                        self.cache.tab3_data.menu_selection = Tab3MenuSelection::OptArea;
                    }
                }
            }
            KeyCode::Enter => {
                match self.cache.tab3_data.menu_selection {
                    Tab3MenuSelection::MenuArea => {
                        // Navigate to the selected settings page
                        match self.cache.tab3_data.menu_cursor {
                            0 => {
                                // Download Settings
                                self.cache.tab3_data.viewing = ViewedTab3::SettingsPage(
                                    Tab3SettingsPage::DownloadPage(Default::default())
                                );
                            }
                            1 => {
                                // Logging Settings
                                self.cache.tab3_data.viewing = ViewedTab3::SettingsPage(
                                    Tab3SettingsPage::LoggingPage(Default::default())
                                );
                            }
                            2 => {
                                // Network Settings
                                self.cache.tab3_data.viewing = ViewedTab3::SettingsPage(
                                    Tab3SettingsPage::NetworkPage(Default::default())
                                );
                            }
                            3 => {
                                // App Settings
                                self.cache.tab3_data.viewing = ViewedTab3::SettingsPage(
                                    Tab3SettingsPage::AppPage(Default::default())
                                );
                            }
                            _ => {}
                        }
                        // Move to OptArea when entering a settings page
                        self.cache.tab3_data.menu_selection = Tab3MenuSelection::OptArea;
                    }
                    Tab3MenuSelection::OptArea => {
                        // Handle specific settings actions
                        if let ViewedTab3::SettingsPage(page) = &mut self.cache.tab3_data.viewing {
                            match page {
                                Tab3SettingsPage::DownloadPage(download_page) => {
                                    match download_page.opt_cursor {
                                        0 => {
                                            // Download Folder - trigger folder browser
                                            self.simulation_controller_data.logs.push_sc_log(
                                                crate::backend::sc_models::LogType::INFO,
                                                "Opening folder browser for download folder...".to_string()
                                            );
                                            // Set the tab3 data so we know we're browsing for download folder
                                            self.cache.tab3_data.browsing_for_download_folder = true;
                                            // Enter file exploring mode
                                            self.app_mode = AppMode::FileExploring;
                                        }
                                        1 => {
                                            // AutoDownload toggle
                                            self.simulation_controller_data.logs.push_sc_log(
                                                crate::backend::sc_models::LogType::INFO,
                                                "Toggling auto download...".to_string()
                                            );
                                            // TODO: Implement auto download toggle
                                        }
                                        2 => {
                                            // Download Limit
                                            self.simulation_controller_data.logs.push_sc_log(
                                                crate::backend::sc_models::LogType::INFO,
                                                "Setting download limit...".to_string()
                                            );
                                            // TODO: Implement download limit setting
                                        }
                                        _ => {}
                                    }
                                }
                                Tab3SettingsPage::LoggingPage(logging_page) => {
                                    match logging_page.opt_cursor {
                                        0 => {
                                            // Log Targets - toggle selection
                                            let cursor_target = &logging_page.available_targets[logging_page.target_cursor].clone();
                                            if logging_page.selected_targets.contains(cursor_target) {
                                                logging_page.selected_targets.retain(|t| t != cursor_target);
                                                self.simulation_controller_data.logs.push_sc_log(
                                                    crate::backend::sc_models::LogType::INFO,
                                                    format!("Deselected log target: {}", cursor_target)
                                                );
                                            } else {
                                                logging_page.selected_targets.push(cursor_target.clone());
                                                self.simulation_controller_data.logs.push_sc_log(
                                                    crate::backend::sc_models::LogType::INFO,
                                                    format!("Selected log target: {}", cursor_target)
                                                );
                                            }
                                        }
                                        1 => {
                                            // Log Levels - toggle selection
                                            let log_levels = LogLevel::all_levels();
                                            let cursor_log_level = &log_levels[logging_page.log_level_cursor].clone();
                                            if logging_page.selected_log_levels.contains(cursor_log_level) {
                                                logging_page.selected_log_levels.retain(|l| l != cursor_log_level);
                                                self.simulation_controller_data.logs.push_sc_log(
                                                    crate::backend::sc_models::LogType::INFO,
                                                    format!("Deselected log level: {}", cursor_log_level.to_string())
                                                );
                                            } else {
                                                logging_page.selected_log_levels.push(cursor_log_level.clone());
                                                self.simulation_controller_data.logs.push_sc_log(
                                                    crate::backend::sc_models::LogType::INFO,
                                                    format!("Selected log level: {}", cursor_log_level.to_string())
                                                );
                                            }
                                        }
                                        2 => {
                                            // Log File
                                            self.simulation_controller_data.logs.push_sc_log(
                                                crate::backend::sc_models::LogType::INFO,
                                                "Setting log file path...".to_string()
                                            );
                                            // TODO: Implement log file setting
                                        }
                                        3 => {
                                            // Log Format
                                            self.simulation_controller_data.logs.push_sc_log(
                                                crate::backend::sc_models::LogType::INFO,
                                                "Setting log format...".to_string()
                                            );
                                            // TODO: Implement log format setting
                                        }
                                        _ => {}
                                    }
                                }
                                Tab3SettingsPage::NetworkPage(network_page) => {
                                    match network_page.opt_cursor {
                                        0 => {
                                            // Connection Timeout
                                            self.simulation_controller_data.logs.push_sc_log(
                                                crate::backend::sc_models::LogType::INFO,
                                                "Setting connection timeout...".to_string()
                                            );
                                            // TODO: Implement connection timeout setting
                                        }
                                        1 => {
                                            // Retry Attempts
                                            self.simulation_controller_data.logs.push_sc_log(
                                                crate::backend::sc_models::LogType::INFO,
                                                "Setting retry attempts...".to_string()
                                            );
                                            // TODO: Implement retry attempts setting
                                        }
                                        2 => {
                                            // Max Connections
                                            self.simulation_controller_data.logs.push_sc_log(
                                                crate::backend::sc_models::LogType::INFO,
                                                "Setting max connections...".to_string()
                                            );
                                            // TODO: Implement max connections setting
                                        }
                                        3 => {
                                            // Network Interface
                                            self.simulation_controller_data.logs.push_sc_log(
                                                crate::backend::sc_models::LogType::INFO,
                                                "Setting network interface...".to_string()
                                            );
                                            // TODO: Implement network interface setting
                                        }
                                        _ => {}
                                    }
                                }
                                Tab3SettingsPage::AppPage(app_page) => {
                                    match app_page.opt_cursor {
                                        0 => {
                                            // Theme
                                            self.simulation_controller_data.logs.push_sc_log(
                                                crate::backend::sc_models::LogType::INFO,
                                                "Setting theme...".to_string()
                                            );
                                            // TODO: Implement theme setting
                                        }
                                        1 => {
                                            // Language
                                            self.simulation_controller_data.logs.push_sc_log(
                                                crate::backend::sc_models::LogType::INFO,
                                                "Setting language...".to_string()
                                            );
                                            // TODO: Implement language setting
                                        }
                                        2 => {
                                            // Auto Save
                                            self.simulation_controller_data.logs.push_sc_log(
                                                crate::backend::sc_models::LogType::INFO,
                                                "Toggling auto save...".to_string()
                                            );
                                            // TODO: Implement auto save toggle
                                        }
                                        3 => {
                                            // Notifications
                                            self.simulation_controller_data.logs.push_sc_log(
                                                crate::backend::sc_models::LogType::INFO,
                                                "Toggling notifications...".to_string()
                                            );
                                            // TODO: Implement notifications toggle
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
            KeyCode::Esc => {
                // Go back one level
                match self.cache.tab3_data.viewing {
                    ViewedTab3::SettingsPage(_) => {
                        // Go back to main settings page
                        self.cache.tab3_data.viewing = ViewedTab3::Main;
                        self.cache.tab3_data.menu_selection = Tab3MenuSelection::MenuArea;
                    }
                    ViewedTab3::Main => {
                        // Already at main level, maybe show a message
                        self.simulation_controller_data.logs.push_sc_log(
                            crate::backend::sc_models::LogType::INFO,
                            "Already at main settings level".to_string()
                        );
                    }
                }
            }
            _ => {}
        }
    }
}