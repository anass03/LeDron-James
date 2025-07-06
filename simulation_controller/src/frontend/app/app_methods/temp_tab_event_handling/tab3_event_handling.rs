use ratatui::crossterm::event::KeyCode;
use crate::frontend::app::App;
use crate::frontend::app::models::tab3_types::{LogLevel, NavigationLevel};
use crate::frontend::app::models::main_types::AppMode;

impl App<'_> {
    pub(crate) fn handle_event_tab3(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up => {
                match self.cache.tab3_data.navigation_level {
                    NavigationLevel::OuterBoxes => {
                        // Navigate between outer boxes (Download Folder <-> Logging Preferences)
                        if self.cache.tab3_data.selected_outer_box > 0 {
                            self.cache.tab3_data.selected_outer_box -= 1;
                        }
                    }
                    NavigationLevel::MiddleBoxes => {
                        // Navigate between middle boxes (Target <-> Log Type)
                        if self.cache.tab3_data.selected_middle_box > 0 {
                            self.cache.tab3_data.selected_middle_box -= 1;
                        }
                    }
                    NavigationLevel::Categories => {
                        // Navigate categories (right panel)
                        if self.cache.tab3_data.selected_category > 0 {
                            self.cache.tab3_data.selected_category -= 1;
                        }
                    }
                }
            }
            KeyCode::Down => {
                match self.cache.tab3_data.navigation_level {
                    NavigationLevel::OuterBoxes => {
                        // Navigate between outer boxes (max 1: 0=Download Folder, 1=Logging Preferences)
                        if self.cache.tab3_data.selected_outer_box < 1 {
                            self.cache.tab3_data.selected_outer_box += 1;
                        }
                    }
                    NavigationLevel::MiddleBoxes => {
                        // Navigate between middle boxes (max 1: 0=Target, 1=Log Type)
                        if self.cache.tab3_data.selected_middle_box < 1 {
                            self.cache.tab3_data.selected_middle_box += 1;
                        }
                    }
                    NavigationLevel::Categories => {
                        // Navigate categories (max 6: 0-6)
                        if self.cache.tab3_data.selected_category < 6 {
                            self.cache.tab3_data.selected_category += 1;
                        }
                    }
                }
            }
            KeyCode::Left => {
                // Move to left panel (settings content) - always go to outer boxes level
                self.cache.tab3_data.navigation_level = NavigationLevel::OuterBoxes;
            }
            KeyCode::Right => {
                // Move to right panel (categories)
                self.cache.tab3_data.navigation_level = NavigationLevel::Categories;
            }
            KeyCode::Enter => {
                match self.cache.tab3_data.navigation_level {
                    NavigationLevel::OuterBoxes => {
                        match self.cache.tab3_data.selected_outer_box {
                            0 => {
                                // Download Folder selected - trigger folder browser
                                self.simulation_controller_data.logs.push_sc_log(
                                    crate::backend::sc_models::LogType::INFO,
                                    "Opening folder browser...".to_string()
                                );
                                // Set the tab3 data so we know we're browsing for download folder
                                self.cache.tab3_data.browsing_for_download_folder = true;
                                // Enter file exploring mode
                                self.app_mode = AppMode::FileExploring;
                            }
                            1 => {
                                // Logging Preferences selected - enter middle boxes level
                                self.cache.tab3_data.navigation_level = NavigationLevel::MiddleBoxes;
                                self.cache.tab3_data.selected_middle_box = 0; // Start at first middle box
                            }
                            _ => {}
                        }
                    }
                    NavigationLevel::MiddleBoxes => {
                        match self.cache.tab3_data.selected_middle_box {
                            0 => {
                                // Choose Target - cycle through available targets
                                let current_index = self.cache.tab3_data.available_targets
                                    .iter()
                                    .position(|target| target == &self.cache.tab3_data.current_log_target)
                                    .unwrap_or(0);

                                let next_index = (current_index + 1) % self.cache.tab3_data.available_targets.len();
                                self.cache.tab3_data.current_log_target =
                                    self.cache.tab3_data.available_targets[next_index].clone();

                                self.simulation_controller_data.logs.push_sc_log(
                                    crate::backend::sc_models::LogType::INFO,
                                    format!("Log target changed to: {}", self.cache.tab3_data.current_log_target)
                                );
                            }
                            1 => {
                                // Choose Log Type - cycle through log levels
                                self.cache.tab3_data.current_log_level =
                                    self.cache.tab3_data.current_log_level.next();

                                self.simulation_controller_data.logs.push_sc_log(
                                    crate::backend::sc_models::LogType::INFO,
                                    format!("Log level changed to: {}", self.cache.tab3_data.current_log_level.to_string())
                                );
                            }
                            _ => {}
                        }
                    }
                    NavigationLevel::Categories => {
                        // Categories selected - could implement category-specific actions
                        self.simulation_controller_data.logs.push_sc_log(
                            crate::backend::sc_models::LogType::INFO,
                            format!("Category {} selected", self.cache.tab3_data.selected_category)
                        );
                    }
                }
            }
            KeyCode::Esc => {
                // Go back one level
                match self.cache.tab3_data.navigation_level {
                    NavigationLevel::MiddleBoxes => {
                        // Go back to outer boxes level
                        self.cache.tab3_data.navigation_level = NavigationLevel::OuterBoxes;
                    }
                    NavigationLevel::Categories => {
                        // Go back to outer boxes level (or could go to middle if we were there before)
                        self.cache.tab3_data.navigation_level = NavigationLevel::OuterBoxes;
                    }
                    NavigationLevel::OuterBoxes => {
                        // Already at top level, maybe show a message or do nothing
                        self.simulation_controller_data.logs.push_sc_log(
                            crate::backend::sc_models::LogType::INFO,
                            "Already at top navigation level".to_string()
                        );
                    }
                }
            }
            KeyCode::Char('b') | KeyCode::Char('B') => {
                if self.cache.tab3_data.navigation_level == NavigationLevel::OuterBoxes
                    && self.cache.tab3_data.selected_outer_box == 0 {
                    // Browse for folder when Download Folder is selected
                    self.simulation_controller_data.logs.push_sc_log(
                        crate::backend::sc_models::LogType::INFO,
                        "Opening folder browser...".to_string()
                    );
                    // Set the tab3 data so we know we're browsing for download folder
                    self.cache.tab3_data.browsing_for_download_folder = true;
                    // Enter file exploring mode
                    self.app_mode = AppMode::FileExploring;
                }
            }
            KeyCode::Char('[') => {
                if self.cache.tab3_data.navigation_level == NavigationLevel::MiddleBoxes
                    && self.cache.tab3_data.selected_middle_box == 1 {
                    // Previous log level when Log Type is selected
                    self.cache.tab3_data.current_log_level =
                        self.cache.tab3_data.current_log_level.prev();

                    self.simulation_controller_data.logs.push_sc_log(
                        crate::backend::sc_models::LogType::INFO,
                        format!("Log level changed to: {}", self.cache.tab3_data.current_log_level.to_string())
                    );
                }
            }
            KeyCode::Char(']') => {
                if self.cache.tab3_data.navigation_level == NavigationLevel::MiddleBoxes
                    && self.cache.tab3_data.selected_middle_box == 1 {
                    // Next log level when Log Type is selected
                    self.cache.tab3_data.current_log_level =
                        self.cache.tab3_data.current_log_level.next();

                    self.simulation_controller_data.logs.push_sc_log(
                        crate::backend::sc_models::LogType::INFO,
                        format!("Log level changed to: {}", self.cache.tab3_data.current_log_level.to_string())
                    );
                }
            }
            KeyCode::Tab => {
                // Toggle between left panel (settings) and right panel (categories)
                match self.cache.tab3_data.navigation_level {
                    NavigationLevel::Categories => {
                        self.cache.tab3_data.navigation_level = NavigationLevel::OuterBoxes;
                    }
                    _ => {
                        self.cache.tab3_data.navigation_level = NavigationLevel::Categories;
                    }
                }
            }
            _ => {
                // Handle other keys if needed
            }
        }
    }
}