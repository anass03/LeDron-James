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
                match self.cache.tab3_data.navigation_level {
                    NavigationLevel::MiddleBoxes => {
                        match self.cache.tab3_data.selected_middle_box {
                            0 => {
                                // Navigate left in targets
                                if self.cache.tab3_data.target_cursor > 0 {
                                    self.cache.tab3_data.target_cursor -= 1;
                                }
                            }
                            1 => {
                                // Navigate left in log levels
                                if self.cache.tab3_data.log_level_cursor > 0 {
                                    self.cache.tab3_data.log_level_cursor -= 1;
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {
                        // Original left navigation (move to categories)
                        self.cache.tab3_data.navigation_level = NavigationLevel::Categories;
                    }
                }
            }
            KeyCode::Right => {
                match self.cache.tab3_data.navigation_level {
                    NavigationLevel::MiddleBoxes => {
                        match self.cache.tab3_data.selected_middle_box {
                            0 => {
                                // Navigate right in targets
                                if self.cache.tab3_data.target_cursor < self.cache.tab3_data.available_targets.len() - 1 {
                                    self.cache.tab3_data.target_cursor += 1;
                                }
                            }
                            1 => {
                                // Navigate right in log levels
                                let log_levels = LogLevel::all_levels();
                                if self.cache.tab3_data.log_level_cursor < log_levels.len() - 1 {
                                    self.cache.tab3_data.log_level_cursor += 1;
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {
                        // Original right navigation (move to settings content)
                        self.cache.tab3_data.navigation_level = NavigationLevel::OuterBoxes;
                    }
                }
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
                                // Choose Target - toggle selection of current cursor target
                                let cursor_target = &self.cache.tab3_data.available_targets[self.cache.tab3_data.target_cursor].clone();

                                if self.cache.tab3_data.selected_targets.contains(cursor_target) {
                                    // Deselect: remove from selected list
                                    self.cache.tab3_data.selected_targets.retain(|t| t != cursor_target);
                                    self.simulation_controller_data.logs.push_sc_log(
                                        crate::backend::sc_models::LogType::INFO,
                                        format!("Deselected target: {}", cursor_target)
                                    );
                                } else {
                                    // Select: add to selected list
                                    self.cache.tab3_data.selected_targets.push(cursor_target.clone());
                                    self.simulation_controller_data.logs.push_sc_log(
                                        crate::backend::sc_models::LogType::INFO,
                                        format!("Selected target: {}", cursor_target)
                                    );
                                }
                            }
                            1 => {
                                // Choose Log Type - toggle selection of current cursor log level
                                let log_levels = LogLevel::all_levels();
                                let cursor_log_level = &log_levels[self.cache.tab3_data.log_level_cursor].clone();

                                if self.cache.tab3_data.selected_log_levels.contains(cursor_log_level) {
                                    // Deselect: remove from selected list
                                    self.cache.tab3_data.selected_log_levels.retain(|l| l != cursor_log_level);
                                    self.simulation_controller_data.logs.push_sc_log(
                                        crate::backend::sc_models::LogType::INFO,
                                        format!("Deselected log level: {}", cursor_log_level.to_string())
                                    );
                                } else {
                                    // Select: add to selected list
                                    self.cache.tab3_data.selected_log_levels.push(cursor_log_level.clone());
                                    self.simulation_controller_data.logs.push_sc_log(
                                        crate::backend::sc_models::LogType::INFO,
                                        format!("Selected log level: {}", cursor_log_level.to_string())
                                    );
                                }
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
                // Keep the old bracket navigation for backwards compatibility (if needed)
                if self.cache.tab3_data.navigation_level == NavigationLevel::MiddleBoxes
                    && self.cache.tab3_data.selected_middle_box == 1 {
                    // Navigate left in log levels (alternative to Left arrow)
                    if self.cache.tab3_data.log_level_cursor > 0 {
                        self.cache.tab3_data.log_level_cursor -= 1;
                    }
                }
            }
            KeyCode::Char(']') => {
                // Keep the old bracket navigation for backwards compatibility (if needed)
                if self.cache.tab3_data.navigation_level == NavigationLevel::MiddleBoxes
                    && self.cache.tab3_data.selected_middle_box == 1 {
                    // Navigate right in log levels (alternative to Right arrow)
                    let log_levels = LogLevel::all_levels();
                    if self.cache.tab3_data.log_level_cursor < log_levels.len() - 1 {
                        self.cache.tab3_data.log_level_cursor += 1;
                    }
                }
            }
            KeyCode::Tab => {
                // Toggle between left panel (categories) and right panel (settings)
                match self.cache.tab3_data.navigation_level {
                    NavigationLevel::Categories => {
                        self.cache.tab3_data.navigation_level = NavigationLevel::OuterBoxes;
                    }
                    _ => {
                        self.cache.tab3_data.navigation_level = NavigationLevel::Categories;
                    }
                }
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                // Clear all selections in current section (optional feature)
                if self.cache.tab3_data.navigation_level == NavigationLevel::MiddleBoxes {
                    match self.cache.tab3_data.selected_middle_box {
                        0 => {
                            // Clear all selected targets
                            let count = self.cache.tab3_data.selected_targets.len();
                            self.cache.tab3_data.selected_targets.clear();
                            self.simulation_controller_data.logs.push_sc_log(
                                crate::backend::sc_models::LogType::INFO,
                                format!("Cleared {} selected targets", count)
                            );
                        }
                        1 => {
                            // Clear all selected log levels
                            let count = self.cache.tab3_data.selected_log_levels.len();
                            self.cache.tab3_data.selected_log_levels.clear();
                            self.simulation_controller_data.logs.push_sc_log(
                                crate::backend::sc_models::LogType::INFO,
                                format!("Cleared {} selected log levels", count)
                            );
                        }
                        _ => {}
                    }
                }
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                // Select all in current section (optional feature)
                if self.cache.tab3_data.navigation_level == NavigationLevel::MiddleBoxes {
                    match self.cache.tab3_data.selected_middle_box {
                        0 => {
                            // Select all targets
                            self.cache.tab3_data.selected_targets = self.cache.tab3_data.available_targets.clone();
                            self.simulation_controller_data.logs.push_sc_log(
                                crate::backend::sc_models::LogType::INFO,
                                format!("Selected all {} targets", self.cache.tab3_data.selected_targets.len())
                            );
                        }
                        1 => {
                            // Select all log levels
                            self.cache.tab3_data.selected_log_levels = LogLevel::all_levels();
                            self.simulation_controller_data.logs.push_sc_log(
                                crate::backend::sc_models::LogType::INFO,
                                format!("Selected all {} log levels", self.cache.tab3_data.selected_log_levels.len())
                            );
                        }
                        _ => {}
                    }
                }
            }
            _ => {
                // Handle other keys if needed
            }
        }
    }
}