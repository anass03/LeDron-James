use ratatui::style::Styled;
use std::default::Default;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Padding, Paragraph, Widget, Wrap, HighlightSpacing};
use crate::frontend::app::App;
use crate::frontend::app::ui::tabs::MAIN_BLOCK_BORDERS;

pub(super) fn render_tab3(app: &App, area: Rect, buf: &mut Buffer) {
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // Settings categories
            Constraint::Percentage(70), // Settings content
        ])
        .split(area);

    let categories_area = main_layout[0];
    let content_area = main_layout[1];

    // Settings Categories List
    let mut categories_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            if app.cache.cursor == 0 {
                app.animation_color()
            } else {
                MAIN_BLOCK_BORDERS
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

    // For now, render with a simple state since we don't have the actual state management
    // In a full implementation, you'd want to track which category is selected
    categories_list.render(categories_area, buf);

    // Settings Content Area
    let mut content_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            if app.cache.cursor == 1 {
                app.animation_color()
            } else {
                MAIN_BLOCK_BORDERS
            }
        ))
        .title(" Settings Options ")
        .title_alignment(Alignment::Center);

    let inner_content_area = content_block.inner(content_area);
    content_block.render(content_area, buf);

    // Content based on selected category (for now showing general settings)
    render_settings_content(app, inner_content_area, buf);
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

    // Main content - showing various settings options
    let settings_content = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),  // Network settings
            Constraint::Length(4),  // Display settings
            Constraint::Length(4),  // Logging settings
            Constraint::Min(2),     // Remaining space
        ])
        .split(main_area);

    // Network Settings Section
    let network_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Network Configuration ");
    let network_content = Paragraph::new(vec![
        Line::from("• Default Port: 8080"),
        Line::from("• Connection Timeout: 30s"),
        Line::from("• Max Concurrent Connections: 100"),
    ])
        .block(network_block)
        .wrap(Wrap { trim: true });
    network_content.render(settings_content[0], buf);

    // Display Settings Section
    let display_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Display Settings ");
    let display_content = Paragraph::new(vec![
        Line::from("• Animation Speed: Normal"),
        Line::from("• Color Scheme: Default"),
        Line::from("• Refresh Rate: 60 FPS"),
    ])
        .block(display_block)
        .wrap(Wrap { trim: true });
    display_content.render(settings_content[1], buf);

    // Logging Settings Section
    let logging_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Logging Configuration ");
    let logging_content = Paragraph::new(vec![
        Line::from("• Log Level: Debug"),
        Line::from("• Max Log Files: 10"),
        Line::from(format!("• Current Log Entries: {}", app.simulation_controller_data.logs.logs_memory.len())),
    ])
        .block(logging_block)
        .wrap(Wrap { trim: true });
    logging_content.render(settings_content[2], buf);

    // Additional info area
    let info_content = Paragraph::new("Use ↑/↓ to navigate categories, ←/→ to switch between panels")
        .style(Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    info_content.render(settings_content[3], buf);

    // Footer
    let footer = Paragraph::new("Press 'q' to quit • Press 'a'/'d' to change tabs")
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);
    footer.render(footer_area, buf);
}