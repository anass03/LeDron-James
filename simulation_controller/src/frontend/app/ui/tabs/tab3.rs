use ratatui::style::Styled;
use std::default::Default;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Padding, Paragraph, Widget, Wrap, HighlightSpacing, Clear, ListState};
use crate::frontend::app::models::tab3_types::{Tab3MenuSelection, ViewedTab3, Tab3SettingsPage, DownloadSettingsItems, LoggingSettingsItems, NetworkSettingsItems, AppSettingsItems, LogLevel, DownloadSettingsPage, LoggingSettingsPage, NetworkSettingsPage, AppSettingsPage, DownloadSettingsCommandsPopup, LoggingSettingsCommandsPopup, NetworkSettingsCommandsPopup, AppSettingsCommandsPopup, CommandPopup, SettingsPopupStatus, RefreshTimeLevel};
use crate::frontend::app::App;
use crate::frontend::app::ui::tabs::{MAIN_BLOCK_BORDERS, POPUP_BG_COLOR};
use log::LevelFilter;

// ============= TAB3.RS RENDER FUNCTION CHANGES =============

pub(super) fn render_tab3(app: &App, area: Rect, buf: &mut Buffer) {
    match &app.cache.tab3_data.viewing {
        ViewedTab3::Main => {
            render_main_settings_page(app, area, buf);
        }
        ViewedTab3::SettingsPage(settings_page) => {
            render_settings_page(app, area, buf, settings_page);
        }
    }
    
    // Show file explorer overlay if browsing for download folder
    if app.cache.tab3_data.browsing_for_download_folder {
        // Create a centered popup area for the file browser
        let popup_area = centered_popup_area(area);
        Clear.render(popup_area, buf);

        let colored_bg_block = Block::default()
            .style(Style::default().bg(POPUP_BG_COLOR))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Choose Download Folder - Press Enter on a directory, Esc to cancel ");
        let colored_popup_area = colored_bg_block.inner(popup_area);
        colored_bg_block.render(popup_area, buf);

        app.cache.file_explorer.widget().render(colored_popup_area, buf);
    }
}

fn render_main_settings_page(app: &App, area: Rect, buf: &mut Buffer) {
    let [menu_area, opt_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // Menu area (left)
            Constraint::Percentage(70), // Options area (right)
        ])
        .areas(area);

    // Menu Area (Left)
    render_menu_area(app, menu_area, buf);
    
    // Options Area (Right)
    render_main_options_area(app, opt_area, buf);
}

fn render_settings_page(app: &App, area: Rect, buf: &mut Buffer, settings_page: &Tab3SettingsPage) {
    let [menu_area, opt_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // Menu area (left)
            Constraint::Percentage(70), // Options area (right)
        ])
        .areas(area);

    // Menu Area (Left)
    render_menu_area(app, menu_area, buf);
    
    // Options Area (Right) - based on settings page type
    match settings_page {
        Tab3SettingsPage::DownloadPage(page) => {
            render_download_options_area(app, opt_area, buf, page);
        }
        Tab3SettingsPage::LoggingPage(page) => {
            render_logging_options_area(app, opt_area, buf, page);
        }
        Tab3SettingsPage::NetworkPage(page) => {
            render_network_options_area(app, opt_area, buf, page);
        }
        Tab3SettingsPage::AppPage(page) => {
            render_app_options_area(app, opt_area, buf, page);
        }
        Tab3SettingsPage::About() => {
            render_about_options_area(app, opt_area, buf);
        }
    }
}

fn render_menu_area(app: &App, area: Rect, buf: &mut Buffer) {
    let menu_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            if app.cache.tab3_data.menu_selection == Tab3MenuSelection::MenuArea {
                app.animation_color()
            } else {
                MAIN_BLOCK_BORDERS
            }
        ))
        .title(" Settings Menu ")
        .title_alignment(Alignment::Center);

    let menu_items = vec![
        "Download Settings",
        "Logging Settings", 
        "Network Settings",
        "App Settings",
    ];

    let menu_list_items: Vec<ListItem> = menu_items
        .iter()
        .enumerate()
        .map(|(i, &item)| {
            let style = if i == app.cache.tab3_data.menu_cursor {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::from(Line::from(item)).style(style)
        })
        .collect();

    let menu_list = List::new(menu_list_items)
        .block(menu_block)
        .highlight_symbol("> ")
        .highlight_spacing(HighlightSpacing::Always);

    menu_list.render(area, buf);
}

fn render_main_options_area(app: &App, area: Rect, buf: &mut Buffer) {
    let opt_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(MAIN_BLOCK_BORDERS))
        .title(" Settings Overview ")
        .title_alignment(Alignment::Center);

    let inner_area = opt_block.inner(area);
    opt_block.render(area, buf);

    let welcome_text = vec![
        "Welcome to Settings",
        "",
        "Select a category from the menu to configure:",
        "",
        "• Download Settings - Configure download folder and limits",
        "• Logging Settings - Set log levels and targets", 
        "• Network Settings - Configure network parameters",
        "• App Settings - Customize application behavior",
        "",
        "Use ↑/↓ to navigate, Enter to select, Esc to go back"
    ];

    let welcome_paragraph = Paragraph::new(welcome_text.join("\n"))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left);

    welcome_paragraph.render(inner_area, buf);
}

fn render_download_options_area(app: &App, area: Rect, buf: &mut Buffer, page: &DownloadSettingsPage) {
    let opt_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            if app.cache.tab3_data.menu_selection == Tab3MenuSelection::OptArea {
                app.animation_color()
            } else {
                MAIN_BLOCK_BORDERS
            }
        ))
        .title(" Download Settings ")
        .title_alignment(Alignment::Center);

    let inner_area = opt_block.inner(area);
    opt_block.render(area, buf);

    // Get the actual download folder path
    let download_folder_path = app.simulation_controller_data.download_folder.to_string_lossy().to_string();
    
    let options = vec![
        format!("Download Folder: {}", download_folder_path),
    ];

    let options_text: Vec<String> = options
        .iter()
        .enumerate()
        .map(|(i, option)| {
            if i == page.opt_cursor {
                format!("> {}", option)
            } else {
                format!("  {}", option)
            }
        })
        .collect();

    let options_paragraph = Paragraph::new(options_text.join("\n"))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left);

    options_paragraph.render(inner_area, buf);

    // Render command popup if active
    render_download_command_popup(app, area, buf, page);
}

fn render_logging_options_area(app: &App, area: Rect, buf: &mut Buffer, page: &LoggingSettingsPage) {
    let opt_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            if app.cache.tab3_data.menu_selection == Tab3MenuSelection::OptArea {
                app.animation_color()
            } else {
                MAIN_BLOCK_BORDERS
            }
        ))
        .title(" Logging Settings ")
        .title_alignment(Alignment::Center);

    let inner_area = opt_block.inner(area);
    opt_block.render(area, buf);

    // Show actual selected targets and levels from cache
    let targets_text = if app.cache.logging_selected_targets.is_empty() {
        "None".to_string()
    } else {
        app.cache.logging_selected_targets.join(", ")
    };

    let levels_text = if app.cache.logging_selected_levels.is_empty() {
        "None".to_string()
    } else {
        app.cache.logging_selected_levels.iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };

    // Get current log level from selected levels in cache
    let current_level_info = if app.cache.logging_selected_levels.is_empty() {
        "OFF".to_string()
    } else {
        // Find the highest selected level
        let highest_level = app.cache.logging_selected_levels.iter().max_by_key(|l| {
            match l {
                LogLevel::Trace => 0,
                LogLevel::Debug => 1,
                LogLevel::Info => 2,
                LogLevel::Warn => 3,
                LogLevel::Error => 4,
            }
        }).unwrap_or(&LogLevel::Info);
        highest_level.to_string().to_string()
    };

    let options = vec![
        format!("Log Targets: {}", targets_text),
        format!("Log Levels: {}", levels_text),
        format!("Current Filter: {}", current_level_info),
        "Available Targets:".to_string(),
    ];

    let options_text: Vec<String> = options
        .iter()
        .enumerate()
        .map(|(i, option)| {
            if i == page.opt_cursor {
                format!("> {}", option)
            } else {
                format!("  {}", option)
            }
        })
        .collect();

    let options_area = inner_area;

    let options_paragraph = Paragraph::new(options_text.join("\n"))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left);

    options_paragraph.render(options_area, buf);



    // Render command popup if active
    render_logging_command_popup(app, area, buf, page);
}

fn render_network_options_area(app: &App, area: Rect, buf: &mut Buffer, page: &NetworkSettingsPage) {
    let opt_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            if app.cache.tab3_data.menu_selection == Tab3MenuSelection::OptArea {
                app.animation_color()
            } else {
                MAIN_BLOCK_BORDERS
            }
        ))
        .title(" Network Settings ")
        .title_alignment(Alignment::Center);

    let inner_area = opt_block.inner(area);
    opt_block.render(area, buf);

    let options = vec![
        "Set All PDR".to_string(),
    ];

    let options_text: Vec<String> = options
        .iter()
        .enumerate()
        .map(|(i, option)| {
            if i == page.opt_cursor {
                format!("> {}", option)
            } else {
                format!("  {}", option)
            }
        })
        .collect();

    let options_paragraph = Paragraph::new(options_text.join("\n"))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left);

    options_paragraph.render(inner_area, buf);

    // Render command popup if active
    render_network_command_popup(app, area, buf, page);
}

fn render_app_options_area(app: &App, area: Rect, buf: &mut Buffer, page: &AppSettingsPage) {
    let opt_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            if app.cache.tab3_data.menu_selection == Tab3MenuSelection::OptArea {
                app.animation_color()
            } else {
                MAIN_BLOCK_BORDERS
            }
        ))
        .title(" App Settings ")
        .title_alignment(Alignment::Center);

    let inner_area = opt_block.inner(area);
    opt_block.render(area, buf);

    // Get current refresh time level
    let current_refresh = match app.settings.refresh_polling.as_millis() {
        50 => "MEDIUM",
        40 => "FAST", 
        25 => "REALLY-FAST",
        _ => "CUSTOM",
    };

    let options = vec![
        format!("Refresh Time: {}", current_refresh),
        "Topology Animation: Disabled".to_string(),
    ];

    let options_text: Vec<String> = options
        .iter()
        .enumerate()
        .map(|(i, option)| {
            if i == page.opt_cursor {
                format!("> {}", option)
            } else {
                format!("  {}", option)
            }
        })
        .collect();

    let options_paragraph = Paragraph::new(options_text.join("\n"))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left);

    options_paragraph.render(inner_area, buf);

    // Render command popup if active
    render_app_command_popup(app, area, buf, page);
}

// Command popup rendering functions
fn render_download_command_popup(app: &App, area: Rect, buf: &mut Buffer, page: &DownloadSettingsPage) {
    // No command popups for download settings currently
}

fn render_logging_command_popup(app: &App, area: Rect, buf: &mut Buffer, page: &LoggingSettingsPage) {
    match &page.command_popup {
        LoggingSettingsCommandsPopup::SetLogTargets(cmd) => {
            render_set_log_targets_popup(app, area, buf, page, cmd);
        }
        LoggingSettingsCommandsPopup::SetLogLevels(cmd) => {
            render_set_log_levels_popup(app, area, buf, page, cmd);
        }
        _ => {}
    }
}

fn render_network_command_popup(app: &App, area: Rect, buf: &mut Buffer, page: &NetworkSettingsPage) {
    match &page.command_popup {
        NetworkSettingsCommandsPopup::SetAllPDR(cmd) => {
            render_set_all_pdr_popup(app, area, buf, cmd);
        }
        _ => {}
    }
}

fn render_app_command_popup(app: &App, area: Rect, buf: &mut Buffer, page: &AppSettingsPage) {
    match &page.command_popup {
        AppSettingsCommandsPopup::SetRefreshTime(cmd) => {
            render_set_refresh_time_popup(app, area, buf, cmd);
        }
        AppSettingsCommandsPopup::ToggleTopologyAnimation(cmd) => {
            render_toggle_topology_animation_popup(app, area, buf, cmd);
        }
        _ => {}
    }
}

// Individual popup rendering functions
fn render_set_log_targets_popup(app: &App, area: Rect, buf: &mut Buffer, page: &LoggingSettingsPage, cmd: &CommandPopup) {
    let popup_area = small_centered_popup_area(area);
    Clear.render(popup_area, buf);
    
    let block = Block::default()
        .title("Select Log Targets")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(popup_border_color(&cmd.status)));
    
    let inner = block.inner(popup_area);
    block.render(popup_area, buf);
    
    let [list_area, status_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),
            Constraint::Min(2),
        ])
        .areas(inner);
    
    // Create list items with selection indicators and current state info
    let list_items: Vec<ListItem> = page.available_targets.iter().enumerate().map(|(i, target)| {
        let is_selected = app.cache.logging_selected_targets.contains(target);
        let is_cursor = i == page.target_cursor;
        
        // Get current level for this target based on selected levels in cache
        let current_level = if app.cache.logging_selected_levels.is_empty() {
            "OFF".to_string()
        } else {
            // Find the highest selected level
            let highest_level = app.cache.logging_selected_levels.iter().max_by_key(|l| {
                match l {
                    LogLevel::Trace => 0,
                    LogLevel::Debug => 1,
                    LogLevel::Info => 2,
                    LogLevel::Warn => 3,
                    LogLevel::Error => 4,
                }
            }).unwrap_or(&LogLevel::Info);
            highest_level.to_string().to_string()
        };
        
        let prefix = if is_selected { "[✓]" } else { "[ ]" };
        let cursor_prefix = if is_cursor { "> " } else { "  " };
        
        let style = if is_cursor {
            Style::default().bg(Color::Blue).fg(Color::White)
        } else {
            Style::default()
        };
        
        let display_text = if target == "All targets" {
            format!("{}{} {} (select all)", cursor_prefix, prefix, target)
        } else {
            format!("{}{} {} ({})", cursor_prefix, prefix, target, current_level)
        };
        
        ListItem::from(display_text).style(style)
    }).collect();
    
    // Create a scrollable list with proper state management
    let mut list_state = ListState::default();
    list_state.select(Some(page.target_cursor));
    
    let list = List::new(list_items)
        .block(Block::default().title("Available Targets").borders(Borders::ALL))
        .highlight_symbol("> ")
        .highlight_spacing(HighlightSpacing::Always);
    
    ratatui::widgets::StatefulWidget::render(
        list,
        list_area,
        buf,
        &mut list_state
    );
    
    render_popup_status(status_area, buf, &cmd.status);
}

fn render_set_log_levels_popup(app: &App, area: Rect, buf: &mut Buffer, page: &LoggingSettingsPage, cmd: &CommandPopup) {
    let popup_area = small_centered_popup_area(area);
    Clear.render(popup_area, buf);
    
    let block = Block::default()
        .title("Select Log Levels")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(popup_border_color(&cmd.status)));
    
    let inner = block.inner(popup_area);
    block.render(popup_area, buf);
    
    let [list_area, status_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),
            Constraint::Min(2),
        ])
        .areas(inner);
    
    // Create list items with selection indicators
    let list_items: Vec<ListItem> = LogLevel::all_levels().iter().enumerate().map(|(i, level)| {
        let is_selected = app.cache.logging_selected_levels.contains(level);
        let is_cursor = i == page.log_level_cursor;
        
        let prefix = if is_selected { "[✓]" } else { "[ ]" };
        let cursor_prefix = if is_cursor { "> " } else { "  " };
        
        let style = if is_cursor {
            Style::default().bg(Color::Blue).fg(Color::White)
        } else {
            Style::default().fg(level.color())
        };
        
        ListItem::from(format!("{}{} {}", cursor_prefix, prefix, level.to_string())).style(style)
    }).collect();
    
    let list = List::new(list_items)
        .block(Block::default().title("Available Levels").borders(Borders::ALL));
    
    list.render(list_area, buf);
    
    render_popup_status(status_area, buf, &cmd.status);
}

fn render_set_all_pdr_popup(app: &App, area: Rect, buf: &mut Buffer, cmd: &CommandPopup) {
    let popup_area = small_centered_popup_area(area);
    Clear.render(popup_area, buf);
    
    let block = Block::default()
        .title("Set All PDR")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(popup_border_color(&cmd.status)));
    
    let inner = block.inner(popup_area);
    block.render(popup_area, buf);
    
    let [fields_area, button_area, status_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .areas(inner);

    // PDR input
    let mut pdr_block = Block::default()
        .title("PDR Value (0.0 - 1.0)")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(
            if cmd.cursor == 0 {
                app.animation_color()
            } else {
                MAIN_BLOCK_BORDERS
            }
        ));
    let inner_pdr = pdr_block.inner(fields_area);
    pdr_block.render(fields_area, buf);

    app.cache.text_buffer.inputfirst_text_area.render(inner_pdr, buf);
    
    // Send button
    let send_button = Paragraph::new("[ Set PDR for All Drones ]")
        .alignment(Alignment::Center)
        .style(Style::default().fg(
            if cmd.cursor == 1 {
                app.animation_color()
            } else {
                MAIN_BLOCK_BORDERS
            }
        ).add_modifier(Modifier::BOLD));
    send_button.render(button_area, buf);
    
    render_popup_status(status_area, buf, &cmd.status);
}

fn render_set_refresh_time_popup(app: &App, area: Rect, buf: &mut Buffer, cmd: &CommandPopup) {
    let popup_area = small_centered_popup_area(area);
    Clear.render(popup_area, buf);
    
    let block = Block::default()
        .title("Set Refresh Time")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(popup_border_color(&cmd.status)));
    
    let inner = block.inner(popup_area);
    block.render(popup_area, buf);
    
    let [list_area, status_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),
            Constraint::Min(2),
        ])
        .areas(inner);
    
    let refresh_levels = vec![
        RefreshTimeLevel::Medium,
        RefreshTimeLevel::Fast,
        RefreshTimeLevel::ReallyFast,
    ];
    
    let list_items: Vec<ListItem> = refresh_levels.iter().enumerate().map(|(i, level)| {
        let is_cursor = i == cmd.cursor as usize;
        
        let style = if is_cursor {
            Style::default().bg(Color::Blue).fg(Color::White)
        } else {
            Style::default()
        };
        
        ListItem::from(format!("  {} ({}ms)", level.to_string(), level.to_duration().as_millis())).style(style)
    }).collect();
    
    let list = List::new(list_items)
        .block(Block::default().title("Refresh Time Options").borders(Borders::ALL));
    
    list.render(list_area, buf);
    
    render_popup_status(status_area, buf, &cmd.status);
}

fn render_toggle_topology_animation_popup(app: &App, area: Rect, buf: &mut Buffer, cmd: &CommandPopup) {
    let popup_area = small_centered_popup_area(area);
    Clear.render(popup_area, buf);
    
    let block = Block::default()
        .title("Toggle Topology Animation")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(popup_border_color(&cmd.status)));
    
    let inner = block.inner(popup_area);
    block.render(popup_area, buf);
    
    let [button_area, status_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .areas(inner);
    
    let toggle_button = Paragraph::new("[ Toggle Animation ]")
        .alignment(Alignment::Center)
        .style(Style::default().fg(
            if cmd.cursor == 0 {
                app.animation_color()
            } else {
                MAIN_BLOCK_BORDERS
            }
        ).add_modifier(Modifier::BOLD));
    toggle_button.render(button_area, buf);
    
    render_popup_status(status_area, buf, &cmd.status);
}

fn render_about_options_area(_app: &App, area: Rect, buf: &mut Buffer) {
    let opt_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(MAIN_BLOCK_BORDERS))
        .title(" About ")
        .title_alignment(Alignment::Center);

    let inner_area = opt_block.inner(area);
    opt_block.render(area, buf);

    let about_text = vec![
        "LeDron James - Network Simulation Controller",
        "",
        "Version: 1.0.0",
        "Author: LeDron Team",
        "",
        "A TUI-based network simulation controller",
        "for drone network management and monitoring.",
        "",
        "Features:",
        "• Real-time network monitoring",
        "• Drone command and control",
        "• File and media management",
        "• Topology visualization",
        "• Logging and statistics",
        "",
        "Press Esc to go back"
    ];

    let about_paragraph = Paragraph::new(about_text.join("\n"))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left);

    about_paragraph.render(inner_area, buf);
}

// Helper functions
fn render_popup_status(area: Rect, buf: &mut Buffer, status: &SettingsPopupStatus) {
    match status {
        SettingsPopupStatus::Idle => {
            let status_paragraph = Paragraph::new("")
                .style(Style::default())
                .alignment(Alignment::Center);
            status_paragraph.render(area, buf);
        }
        SettingsPopupStatus::CorrectlySent => {
            let status_paragraph = Paragraph::new("✓ Success")
                .style(Style::default().fg(Color::Green))
                .alignment(Alignment::Center);
            status_paragraph.render(area, buf);
        }
        SettingsPopupStatus::Error(err) => {
            let error_text = format!("✗ Error: {}", err);
            let status_paragraph = Paragraph::new(error_text)
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center);
            status_paragraph.render(area, buf);
        }
    }
}

fn popup_border_color(status: &SettingsPopupStatus) -> Color {
    match status {
        SettingsPopupStatus::Idle => MAIN_BLOCK_BORDERS,
        SettingsPopupStatus::CorrectlySent => Color::Green,
        SettingsPopupStatus::Error(_) => Color::Red,
    }
}

// Helper function to create a centered popup area
fn centered_popup_area(area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(popup_layout[1])[1]
}

// Helper function to create a small centered popup area (for command popups)
fn small_centered_popup_area(area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .split(popup_layout[1])[1]
}