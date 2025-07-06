use ratatui::style::Styled;
use std::default::Default;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Padding, Paragraph, Widget, Wrap, HighlightSpacing};
use crate::frontend::app::models::tab3_types::{NavigationLevel};
use crate::frontend::app::App;
use crate::frontend::app::ui::tabs::MAIN_BLOCK_BORDERS;

pub(super) fn render_tab3(app: &App, area: Rect, buf: &mut Buffer) {
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Settings content (new sections)
            Constraint::Percentage(60), // Settings categories
        ])
        .split(area);

    let content_area = main_layout[0];
    let categories_area = main_layout[1];

    // Settings Content Area (Now on the left)
    let mut content_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            match app.cache.tab3_data.navigation_level {
                NavigationLevel::OuterBoxes | NavigationLevel::MiddleBoxes => {
                    app.animation_color()
                }
                _ => MAIN_BLOCK_BORDERS
            }
        ))
        .title(" Settings Options ")
        .title_alignment(Alignment::Center);

    let inner_content_area = content_block.inner(content_area);
    content_block.render(content_area, buf);

    // Content based on selected category (for now showing general settings)
    render_settings_content(app, inner_content_area, buf);

    // Settings Categories List (Now on the right)
    let mut categories_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            match app.cache.tab3_data.navigation_level {
                NavigationLevel::Categories => {
                    app.animation_color()
                }
                _ => MAIN_BLOCK_BORDERS
            }
        ))
        .title(" Settings Categories ")
        .title_alignment(Alignment::Center);

    let settings_categories = vec![
        "Network Configuration",
        "Display Settings",
        "Logging Options",
        "Performance Tuning",
        "File Paths",
        "Keyboard Shortcuts",
        "About",
    ];

    let category_items: Vec<ListItem> = settings_categories
        .iter()
        .enumerate()
        .map(|(i, &category)| {
            let style = if i % 2 == 0 {
                Style::default()
            } else {
                Style::default().fg(Color::Gray)
            };
            ListItem::from(Line::from(category)).style(style)
        })
        .collect();

    let categories_list = List::new(category_items)
        .block(categories_block)
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        )
        .highlight_symbol("> ")
        .highlight_spacing(HighlightSpacing::Always);

    categories_list.render(categories_area, buf);

    // Show file explorer overlay if browsing for download folder
    if app.cache.tab3_data.browsing_for_download_folder {
        use ratatui::widgets::Clear;
        use crate::frontend::app::ui::tabs::{POPUP_BG_COLOR};

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

fn render_settings_content(app: &App, area: Rect, buf: &mut Buffer) {
    let content_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Main content
            Constraint::Length(3),  // Footer info
        ])
        .split(area);

    let header_area = content_layout[0];
    let main_area = content_layout[1];
    let footer_area = content_layout[2];

    // Header
    let header = Paragraph::new("Application Settings")
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    header.render(header_area, buf);

    // Main content - NEW LAYOUT with 3 sections as requested
    let settings_content = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),  // Choose Download Folder
            Constraint::Min(8),     // Logging Preferences (outer box)
            Constraint::Min(2),     // Remaining space
        ])
        .spacing(1)
        .split(main_area);

    // 1. Choose Download Folder Section
    render_download_folder_section(app, settings_content[0], buf);

    // 2. Logging Preferences Section (contains two sub-sections)
    render_logging_preferences_section(app, settings_content[1], buf);

    // Footer
    let footer = Paragraph::new("Use ↑/↓ to navigate options, Enter to select/edit")
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);
    footer.render(footer_area, buf);
}

fn render_download_folder_section(app: &App, area: Rect, buf: &mut Buffer) {
    let download_folder_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            if app.cache.tab3_data.navigation_level == NavigationLevel::OuterBoxes
                && app.cache.tab3_data.selected_outer_box == 0 {
                app.animation_color()
            } else {
                MAIN_BLOCK_BORDERS
            }
        ))
        .title(" Choose Download Folder ");

    let inner_area = download_folder_block.inner(area);
    download_folder_block.render(area, buf);

    // Split inner area for label and field
    let folder_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Label
            Constraint::Length(1),  // Current folder display
            Constraint::Length(1),  // Browse button/instruction
        ])
        .split(inner_area);

    // Current download folder display
    let current_folder = &app.simulation_controller_data.download_folder;
    let folder_display = Paragraph::new(format!("Current: {}", current_folder))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    folder_display.render(folder_layout[1], buf);

    // Browse instruction
    let browse_instruction = Paragraph::new("Press 'B' to browse for folder")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::ITALIC))
        .alignment(Alignment::Center);
    browse_instruction.render(folder_layout[2], buf);
}

fn render_logging_preferences_section(app: &App, area: Rect, buf: &mut Buffer) {
    // Outer Logging Preferences box
    let logging_preferences_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            if app.cache.tab3_data.navigation_level == NavigationLevel::OuterBoxes
                && app.cache.tab3_data.selected_outer_box == 1 {
                app.animation_color()
            } else {
                MAIN_BLOCK_BORDERS
            }
        ))
        .title(" Logging Preferences ")
        .title_alignment(Alignment::Center);

    let inner_area = logging_preferences_block.inner(area);
    logging_preferences_block.render(area, buf);

    // Split into two sub-sections
    let logging_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),  // Choose Target
            Constraint::Length(6),  // Choose Log Type
        ])
        .spacing(1)
        .split(inner_area);

    // 1. Choose Target sub-section
    render_choose_target_section(app, logging_layout[0], buf);

    // 2. Choose Log Type sub-section
    render_choose_log_type_section(app, logging_layout[1], buf);
}

fn render_choose_target_section(app: &App, area: Rect, buf: &mut Buffer) {
    let target_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            if app.cache.tab3_data.navigation_level == NavigationLevel::MiddleBoxes
                && app.cache.tab3_data.selected_middle_box == 0 {
                app.animation_color()
            } else {
                MAIN_BLOCK_BORDERS
            }
        ))
        .title(" Choose Target ");

    let inner_area = target_block.inner(area);
    target_block.render(area, buf);

    let target_content = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Current selection
            Constraint::Length(1),  // Available label
            Constraint::Length(1),  // Available options
        ])
        .split(inner_area);

    // Current target display
    let current_target = Paragraph::new(format!("Current: {}", &app.cache.tab3_data.current_log_target))
        .style(Style::default().fg(Color::Green))
        .alignment(Alignment::Center);
    current_target.render(target_content[0], buf);

    // Available label
    let available_label = Paragraph::new("Available:")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    available_label.render(target_content[1], buf);

    // Available targets
    let targets_text = app.cache.tab3_data.available_targets.join(" | ");
    let targets_para = Paragraph::new(targets_text)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    targets_para.render(target_content[2], buf);
}

fn render_choose_log_type_section(app: &App, area: Rect, buf: &mut Buffer) {
    let log_type_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            if app.cache.tab3_data.navigation_level == NavigationLevel::MiddleBoxes
                && app.cache.tab3_data.selected_middle_box == 1 {
                app.animation_color()
            } else {
                MAIN_BLOCK_BORDERS
            }
        ))
        .title(" Choose Log Type ");

    let inner_area = log_type_block.inner(area);
    log_type_block.render(area, buf);

    // Available log types
    let log_types = vec![
        ("TRACE", Color::Magenta),
        ("DEBUG", Color::Green),
        ("INFO", Color::White),
        ("WARN", Color::Yellow),
        ("ERROR", Color::Red),
        ("FATAL", Color::LightRed),
    ];

    let log_type_content = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Current selection
            Constraint::Length(1),  // Available label
            Constraint::Length(1),  // Available options
        ])
        .split(inner_area);

    // Current log level display
    let current_level = Paragraph::new(format!("Current: {}", app.cache.tab3_data.current_log_level.to_string()))
        .style(Style::default().fg(Color::Green))
        .alignment(Alignment::Center);
    current_level.render(log_type_content[0], buf);

    // Available label
    let available_label = Paragraph::new("Available:")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    available_label.render(log_type_content[1], buf);

    // Available log types with colors - all in one line
    let mut spans = Vec::new();

    for (i, (log_type, color)) in log_types.iter().enumerate() {
        spans.push(Span::styled(*log_type, Style::default().fg(*color).add_modifier(Modifier::BOLD)));
        if i < log_types.len() - 1 {
            spans.push(Span::raw(" | "));
        }
    }

    let log_types_para = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    log_types_para.render(log_type_content[2], buf);
}