use ratatui::crossterm::event::KeyCode;
use crate::frontend::app::App;
use crate::frontend::app::models::tab3_types::{
    Tab3MenuSelection, ViewedTab3, Tab3SettingsPage, 
    DownloadSettingsItems, LoggingSettingsItems, NetworkSettingsItems, AppSettingsItems,
    LogLevel, DownloadSettingsPage, LoggingSettingsPage, NetworkSettingsPage, AppSettingsPage,
    DownloadSettingsCommandsPopup, LoggingSettingsCommandsPopup, NetworkSettingsCommandsPopup, AppSettingsCommandsPopup,
    CommandPopup, SettingsPopupStatus, RefreshTimeLevel
};
use crate::frontend::app::models::main_types::{AppMode, InputMode};
use crate::backend::sc_models::RendererCommand;
use std::path::PathBuf;
use log::{error, info, warn, debug, LevelFilter};
use tui_logger::TuiWidgetState;
use crate::frontend::app::models::general_types::Node;

impl App<'_> {
    pub(crate) fn handle_event_tab3(&mut self, key: KeyCode) {
        // First check if we're in a command popup
        if self.is_in_tab3_command_popup() {
            self.handle_tab3_command_popup(key);
            return;
        }

        match key {
            KeyCode::Backspace => {
                // Handle backspace based on current state
                match self.cache.tab3_data.menu_selection {
                    Tab3MenuSelection::OptArea => {
                        // If in settings area, go back to menu area
                        self.cache.tab3_data.menu_selection = Tab3MenuSelection::MenuArea;
                        info!(target: "main", "[LEDRON-CLI] Backspace: Returned to menu area");
                    }
                    Tab3MenuSelection::MenuArea => {
                        // If in menu area, backspace does nothing (could be used for other purposes)
                        info!(target: "main", "[LEDRON-CLI] Backspace pressed at settings menu level");
                    }
                }
            }
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
                                // Logging Settings - preserve existing state if available
                                let logging_page = if let ViewedTab3::SettingsPage(Tab3SettingsPage::LoggingPage(existing_page)) = &self.cache.tab3_data.viewing {
                                    existing_page.clone()
                                } else {
                                    // Create new logging page with current state from main_log_state
                                    let mut new_page = LoggingSettingsPage::default();
                                    
                                    // Get all available targets from the system
                                    let mut available_targets = vec![
                                        "main".to_string(),
                                        "simulation_controller".to_string(),
                                        "renderer".to_string(),
                                        "network".to_string(),
                                        "topology".to_string(),
                                    ];
                                    
                                    // Add targets from host_log_state
                                    for (_, (target, _)) in &self.simulation_controller_data.host_log_state {
                                        if !available_targets.contains(target) {
                                            available_targets.push(target.clone());
                                        }
                                    }
                                    available_targets.push("All targets".to_string());
                                    new_page.available_targets = available_targets;
                                    
                                    // Use cached logging state
                                    new_page.selected_targets = self.cache.logging_selected_targets.clone();
                                    new_page.selected_log_levels = self.cache.logging_selected_levels.clone();
                                    new_page
                                };
                                self.cache.tab3_data.viewing = ViewedTab3::SettingsPage(
                                    Tab3SettingsPage::LoggingPage(logging_page)
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
                        match &mut self.cache.tab3_data.viewing {
                            ViewedTab3::SettingsPage(Tab3SettingsPage::DownloadPage(download_page)) => {
                                match download_page.opt_cursor {
                                    0 => {
                                        info!(target: "main", "[LEDRON-CLI] Opening folder browser for download folder...");
                                        self.cache.tab3_data.browsing_for_download_folder = true;
                                        self.app_mode = AppMode::FileExploring;
                                    }
                                    _ => {}
                                }
                            }
                            ViewedTab3::SettingsPage(Tab3SettingsPage::LoggingPage(logging_page)) => {
                                match logging_page.opt_cursor {
                                    0 => {
                                        logging_page.command_popup = LoggingSettingsCommandsPopup::SetLogTargets(CommandPopup {
                                            status: SettingsPopupStatus::Idle,
                                            cursor: 0,
                                        });
                                    }
                                    1 => {
                                        logging_page.command_popup = LoggingSettingsCommandsPopup::SetLogLevels(CommandPopup {
                                            status: SettingsPopupStatus::Idle,
                                            cursor: 0,
                                        });
                                    }
                                    2 | 3 => {
                                        // Current Filter and Available Targets are read-only info
                                        // No action needed
                                    }
                                    _ => {}
                                }
                            }
                            ViewedTab3::SettingsPage(Tab3SettingsPage::NetworkPage(network_page)) => {
                                match network_page.opt_cursor {
                                    0 => {
                                        network_page.command_popup = NetworkSettingsCommandsPopup::SetAllPDR(CommandPopup {
                                            status: SettingsPopupStatus::Idle,
                                            cursor: 0,
                                        });
                                    }
                                    _ => {}
                                }
                            }
                            ViewedTab3::SettingsPage(Tab3SettingsPage::AppPage(app_page)) => {
                                match app_page.opt_cursor {
                                    0 => {
                                        app_page.command_popup = AppSettingsCommandsPopup::SetRefreshTime(CommandPopup {
                                            status: SettingsPopupStatus::Idle,
                                            cursor: 0,
                                        });
                                    }
                                    1 => {
                                        app_page.command_popup = AppSettingsCommandsPopup::ToggleTopologyAnimation(CommandPopup {
                                            status: SettingsPopupStatus::Idle,
                                            cursor: 0,
                                        });
                                    }
                                    _ => {}
                                }
                            }
                            _ => {}
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
                        info!(target: "main", "[LEDRON-CLI] Already at main settings level");
                    }
                }
            }
            _ => {}
        }
    }

    fn is_in_tab3_command_popup(&self) -> bool {
        if let ViewedTab3::SettingsPage(page) = &self.cache.tab3_data.viewing {
            match page {
                Tab3SettingsPage::DownloadPage(download_page) => {
                    !matches!(download_page.command_popup, DownloadSettingsCommandsPopup::None)
                }
                Tab3SettingsPage::LoggingPage(logging_page) => {
                    !matches!(logging_page.command_popup, LoggingSettingsCommandsPopup::None)
                }
                Tab3SettingsPage::NetworkPage(network_page) => {
                    !matches!(network_page.command_popup, NetworkSettingsCommandsPopup::None)
                }
                Tab3SettingsPage::AppPage(app_page) => {
                    !matches!(app_page.command_popup, AppSettingsCommandsPopup::None)
                }
                Tab3SettingsPage::About() => {
                    false
                }
            }
        } else {
            false
        }
    }

    // Handle command popup events
    pub(crate) fn handle_tab3_command_popup(&mut self, key: KeyCode) {
        match &mut self.cache.tab3_data.viewing {
            ViewedTab3::SettingsPage(Tab3SettingsPage::DownloadPage(download_page)) => {
                match &mut download_page.command_popup {
                    // No command popups for download settings currently
                    _ => {}
                }
            }
            ViewedTab3::SettingsPage(Tab3SettingsPage::LoggingPage(logging_page)) => {
                match &mut logging_page.command_popup {
                    LoggingSettingsCommandsPopup::SetLogTargets(cmd) => {
                        match key {
                            KeyCode::Esc => {
                                logging_page.command_popup = LoggingSettingsCommandsPopup::None;
                            }
                            KeyCode::Up => {
                                if logging_page.target_cursor > 0 {
                                    logging_page.target_cursor -= 1;
                                }
                            }
                            KeyCode::Down => {
                                if logging_page.target_cursor < logging_page.available_targets.len() - 1 {
                                    logging_page.target_cursor += 1;
                                }
                            }
                            KeyCode::Enter => {
                                let selected_target = &logging_page.available_targets[logging_page.target_cursor];
                                if selected_target == "All targets" {
                                    self.cache.logging_selected_targets = logging_page.available_targets.iter()
                                        .filter(|&t| t != "All targets")
                                        .cloned()
                                        .collect();
                                } else {
                                    if self.cache.logging_selected_targets.contains(selected_target) {
                                        self.cache.logging_selected_targets.retain(|t| t != selected_target);
                                    } else {
                                        self.cache.logging_selected_targets.push(selected_target.clone());
                                    }
                                }
                                // Update page state to match cache
                                logging_page.selected_targets = self.cache.logging_selected_targets.clone();
                                
                                // Update main log state
                                let mut new_state = TuiWidgetState::default()
                                    .set_default_display_level(LevelFilter::Off)
                                    .set_level_for_target("main", LevelFilter::max());
                                for target in &self.cache.logging_selected_targets {
                                    new_state = new_state.set_level_for_target(target.as_str(), LevelFilter::max());
                                }
                                self.cache.main_log_state.1 = new_state;
                                cmd.status = SettingsPopupStatus::CorrectlySent;
                            }
                            _ => {}
                        }
                    }
                    LoggingSettingsCommandsPopup::SetLogLevels(cmd) => {
                        match key {
                            KeyCode::Esc => {
                                logging_page.command_popup = LoggingSettingsCommandsPopup::None;
                            }
                            KeyCode::Up => {
                                if logging_page.log_level_cursor > 0 {
                                    logging_page.log_level_cursor -= 1;
                                }
                            }
                            KeyCode::Down => {
                                if logging_page.log_level_cursor < LogLevel::all_levels().len() - 1 {
                                    logging_page.log_level_cursor += 1;
                                }
                            }
                            KeyCode::Enter => {
                                let selected_level = &LogLevel::all_levels()[logging_page.log_level_cursor];
                                if self.cache.logging_selected_levels.contains(selected_level) {
                                    self.cache.logging_selected_levels.retain(|l| l != selected_level);
                                } else {
                                    self.cache.logging_selected_levels.push(selected_level.clone());
                                }
                                
                                // Update page state to match cache
                                logging_page.selected_log_levels = self.cache.logging_selected_levels.clone();
                                
                                // Update main log state with selected levels
                                let mut new_state = TuiWidgetState::default()
                                    .set_default_display_level(LevelFilter::Off);
                                
                                // Set the minimum level to the highest selected level
                                if let Some(highest_level) = self.cache.logging_selected_levels.iter().max_by_key(|l| {
                                    match l {
                                        LogLevel::Trace => 0,
                                        LogLevel::Debug => 1,
                                        LogLevel::Info => 2,
                                        LogLevel::Warn => 3,
                                        LogLevel::Error => 4,
                                    }
                                }) {
                                    let level_filter = match highest_level {
                                        LogLevel::Trace => LevelFilter::Trace,
                                        LogLevel::Debug => LevelFilter::Debug,
                                        LogLevel::Info => LevelFilter::Info,
                                        LogLevel::Warn => LevelFilter::Warn,
                                        LogLevel::Error => LevelFilter::Error,
                                    };
                                    
                                    // Apply to all targets
                                    for target in &self.cache.logging_selected_targets {
                                        new_state = new_state.set_level_for_target(target.as_str(), level_filter);
                                    }
                                    // Also apply to main target
                                    new_state = new_state.set_level_for_target("main", level_filter);
                                }
                                
                                self.cache.main_log_state.1 = new_state;
                                cmd.status = SettingsPopupStatus::CorrectlySent;
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            ViewedTab3::SettingsPage(Tab3SettingsPage::NetworkPage(network_page)) => {
                match &mut network_page.command_popup {
                    NetworkSettingsCommandsPopup::SetAllPDR(cmd) => {
                        match key {
                            KeyCode::Esc => {
                                network_page.command_popup = NetworkSettingsCommandsPopup::None;
                            }
                            KeyCode::Up => {
                                if cmd.cursor > 0 {
                                    cmd.cursor -= 1;
                                }
                            }
                            KeyCode::Down => {
                                if cmd.cursor < 1 {
                                    cmd.cursor += 1;
                                }
                            }
                            KeyCode::Enter => {
                                match cmd.cursor {
                                    0 => {
                                        // Enter input mode for PDR value
                                        self.app_mode = AppMode::Input(InputMode::One);
                                    }
                                    1 => {
                                        // Parse and apply the PDR value
                                        match self.cache.text_buffer.first_field_parse_f32(true) {
                                            Ok(pdr) => {
                                                if pdr >= 0.0 && pdr <= 1.0 {
                                                    let mut zero_errors:bool = true;
                                                    let mut last_error:String = "".to_string();

                                                    // Set PDR for all drones
                                                    for (node,_,_) in &self.cache.tab1_data.net_node_list.items {
                                                        match node{
                                                            Node::Drone(dr) => {
                                                                match self.simulation_controller_data.set_pdr(dr.id,pdr){
                                                                    Ok(_) => (),
                                                                    Err(err) => {
                                                                        zero_errors = false;
                                                                        last_error = err.to_string();
                                                                        error!(target:"main", "{:?}", err);
                                                                    }
                                                                }
                                                            }
                                                            _ => (),
                                                        }

                                                    }
                                                    if zero_errors {
                                                        cmd.status = SettingsPopupStatus::CorrectlySent;
                                                    }else{
                                                        cmd.status = SettingsPopupStatus::Error(last_error);
                                                    }

                                                    info!(target:"main", "[LEDRON-CLI] {:?}", format!("PDR set to {} for all drones", pdr));
                                                } else {
                                                    cmd.status = SettingsPopupStatus::Error("PDR must be between 0.0 and 1.0".to_string());
                                                }
                                            }
                                            Err(err) => {
                                                cmd.status = SettingsPopupStatus::Error(err);
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            ViewedTab3::SettingsPage(Tab3SettingsPage::AppPage(app_page)) => {
                match &mut app_page.command_popup {
                    AppSettingsCommandsPopup::SetRefreshTime(cmd) => {
                        match key {
                            KeyCode::Esc => {
                                app_page.command_popup = AppSettingsCommandsPopup::None;
                            }
                            KeyCode::Up => {
                                if cmd.cursor > 0 {
                                    cmd.cursor -= 1;
                                }
                            }
                            KeyCode::Down => {
                                if cmd.cursor < 2 {
                                    cmd.cursor += 1;
                                }
                            }
                            KeyCode::Enter => {
                                let refresh_levels = [
                                    RefreshTimeLevel::Medium,
                                    RefreshTimeLevel::Fast,
                                    RefreshTimeLevel::ReallyFast,
                                ];
                                let selected = &refresh_levels[cmd.cursor as usize];
                                self.settings.refresh_polling = selected.to_duration();
                                cmd.status = SettingsPopupStatus::CorrectlySent;
                                info!(target: "main", "[LEDRON-CLI] Refresh time set to {}ms", selected.to_duration().as_millis());
                            }
                            _ => {}
                        }
                    }
                    AppSettingsCommandsPopup::ToggleTopologyAnimation(cmd) => {
                        match key {
                            KeyCode::Esc => {
                                app_page.command_popup = AppSettingsCommandsPopup::None;
                            }
                            KeyCode::Enter => {
                                // Toggle animation (example logic)
                                /*self.settings.topology_animation = !self.settings.topology_animation;
                                cmd.status = SettingsPopupStatus::CorrectlySent;
                                self.simulation_controller_data.logs.push_sc_log(
                                    crate::backend::sc_models::LogType::INFO,
                                    format!("Topology animation toggled: {}", self.settings.topology_animation)
                                );*/
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    // Handle file explorer result for download folder
    pub(crate) fn handle_download_folder_selected(&mut self, path: PathBuf) {
        // Update the download folder in simulation controller
        self.simulation_controller_data.download_folder = path.clone();
        
        // Send command to renderer to change download folder
        if let Some(sender) = &self.simulation_controller_data.tx_to_renderer {
            if let Err(e) = sender.send(RendererCommand::ChangeDownloadFolder(path.clone())) {
                error!(target: "main", "[LEDRON-CLI] Failed to send download folder change to renderer: {}", e);
            } else {
                info!(target: "main", "[LEDRON-CLI] Download folder changed to: {}", path.display());
            }
        }
        
        // Exit file exploring mode
        self.cache.tab3_data.browsing_for_download_folder = false;
        self.app_mode = AppMode::Normal;
    }
}