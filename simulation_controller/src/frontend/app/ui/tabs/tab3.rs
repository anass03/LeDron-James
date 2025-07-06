use ratatui::style::Styled;
use std::default::Default;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Padding, Paragraph, Widget, Wrap, HighlightSpacing, Clear};
use crate::frontend::app::models::tab3_types::{Tab3MenuSelection, ViewedTab3, Tab3SettingsPage, DownloadSettingsItems, LoggingSettingsItems, NetworkSettingsItems, AppSettingsItems, LogLevel, DownloadSettingsPage, LoggingSettingsPage, NetworkSettingsPage, AppSettingsPage};
use crate::frontend::app::App;
use crate::frontend::app::ui::tabs::{MAIN_BLOCK_BORDERS, POPUP_BG_COLOR};

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
        "Auto Download: Enabled".to_string(),
        "Download Limit: 100 MB/s".to_string(),
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

    let options = vec![
        "Log Targets: main, simulation_controller",
        "Log Levels: DEBUG, INFO, WARN",
        "Log File: /var/log/app.log",
        "Log Format: JSON",
    ];

    let options_text: Vec<String> = options
        .iter()
        .enumerate()
        .map(|(i, &option)| {
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
        "Connection Timeout: 30s",
        "Retry Attempts: 3",
        "Max Connections: 10",
        "Network Interface: eth0",
    ];

    let options_text: Vec<String> = options
        .iter()
        .enumerate()
        .map(|(i, &option)| {
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

    let options = vec![
        "Theme: Dark",
        "Language: English",
        "Auto Save: Enabled",
        "Notifications: Enabled",
    ];

    let options_text: Vec<String> = options
        .iter()
        .enumerate()
        .map(|(i, &option)| {
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