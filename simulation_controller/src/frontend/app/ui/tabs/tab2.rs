use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Padding, Paragraph, Tabs, Widget, Wrap, HighlightSpacing};
use crate::frontend::app::App;
use crate::frontend::app::models::general_types::TOMLmessages;
use crate::frontend::app::models::main_types::AppMode;
use crate::frontend::app::ui::tabs::{MAIN_BLOCK_BORDERS, ALTERNATE_LIST_COLOR, BG_COLOR, SELECTED_LIST_COLOR};


const SELECTED_STYLE: Style = Style::new().bg(Color::Blue).add_modifier(Modifier::BOLD);

/// Renders TOML Editor tab-page
pub(super) fn render_tab2(app:&App, area: Rect, buf : &mut Buffer){
    let inner_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70),
            Constraint::Percentage(30),
        ]).split(area);


    let toml_view_area = inner_columns[0];
    let menu_view_area = inner_columns[1];


    let mut toml_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            if app.cache.cursor == 0{
                app.animation_color()
            }else{
                MAIN_BLOCK_BORDERS
            }
        ))
        .title(" TOML FILE VIEW ")
        .title_alignment(Alignment::Center);

    let inner_toml_block = toml_block.inner(toml_view_area);

    let inner_toml_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(inner_toml_block);

    let compiler_area = inner_toml_layout[0];

    // Area which will be used for text editor
    let editor_area = inner_toml_layout[1];

    let compiler_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(MAIN_BLOCK_BORDERS))
        .border_type(BorderType::Double);
    let inner_compiler_block = compiler_block.inner(compiler_area);
    let mut compiler_status = Paragraph::default();
    match &app.cache.tab2_data.compiler_buf{
        TOMLmessages::Correct => {
            compiler_status = Paragraph::new("Topology validation passed!")
                .style(Style::default().fg(Color::Rgb(149, 207, 146))
                    .add_modifier(Modifier::BOLD|Modifier::ITALIC))
                .centered();
        }
        TOMLmessages::Error(str) => {
            compiler_status = Paragraph::new(format!("Error compiling: {}",str))
                .style(Style::default().fg(Color::Red)
                    .add_modifier(Modifier::BOLD|Modifier::ITALIC))
                .centered();
        }
        TOMLmessages::FirstInizialized => {
            compiler_status = Paragraph::new("Compile manually to check TOML validity")
                .style(Style::default().fg(Color::Gray)
                    .add_modifier(Modifier::BOLD| Modifier::ITALIC))
                .centered();
        }
    }

    let mut menu_view_area_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            if app.cache.cursor == 1{
                app.animation_color()
            }else{
                MAIN_BLOCK_BORDERS
            }
        ))
        .title(" Choose Topology Template ")
        .title_alignment(Alignment::Center);
    let inner_menu_view_area = menu_view_area_block.inner(menu_view_area);

    // Create layout for topology list and the centered animation-text-animation block
    let menu_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),     // Fixed space for topology list (6 items + some padding)
            Constraint::Min(0),        // Remaining space for the centered block
        ])
        .split(inner_menu_view_area);

    let topology_list_area = menu_layout[0];
    let centered_block_area = menu_layout[1];

    // Calculate the total height needed for animation-text-animation (6 + 3 + 6 = 15)
    let content_height = 15;
    let available_height = centered_block_area.height;

    // Reduce vertical padding to move content higher
    let vertical_padding = if available_height > content_height {
        (available_height - content_height) / 4  // Reduced from /2 to /4
    } else {
        0
    };

    // Create the centered layout for animation-text-animation
    let centered_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(vertical_padding),     // Smaller top padding
            Constraint::Length(6),                    // Reduced top animation height
            Constraint::Length(3),                    // Instruction text
            Constraint::Length(6),                    // Reduced bottom animation height
            Constraint::Min(0),                       // Remaining space (bottom padding)
        ])
        .split(centered_block_area);

    let top_galaxy_area = centered_layout[1];
    let instruction_area = centered_layout[2];
    let bottom_galaxy_area = centered_layout[3];

    // Render top galaxy animation
    render_galaxy_animation(app, top_galaxy_area, buf);

    // Render the centered instruction text
    let instruction_text = Paragraph::new("You can choose your network here\nmodify it if preferred\nand restart the app to use it!")
        .style(Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::ITALIC))
        .centered()
        .wrap(Wrap { trim: true });

    instruction_text.render(instruction_area, buf);

    // Render bottom galaxy animation
    render_galaxy_animation(app, bottom_galaxy_area, buf);

    // Define 6 topology templates
    let topology_names = vec![
        "Personalized Network",
        "Star Network",
        "Sub-Stars Network",
        "Double Chain Network",
        "Tree Network ",
        "Butterfly Network",
    ];

    // Create list items with alternating colors for better readability
    let topology_items: Vec<ListItem> = topology_names
        .iter()
        .enumerate()
        .map(|(idx, &topology)| {
            let base_style = if idx % 2 == 0 {
                // even rows – use background color
                Style::default().bg(BG_COLOR)
            } else {
                // odd rows – use alternate color for contrast
                Style::default().bg(ALTERNATE_LIST_COLOR)
            };

            // Add icon based on topology type for visual appeal
            let icon = match idx {
                0 => "✏️ - ", // Personalized
                1 => "⭐️ - ", // Star
                2 => "✨ - ", // Sub-Star
                3 => "⛓️ - ", // Double Chain
                4 => "🌳 - ", // Tree
                5 => "🦋 - ", // Butterfly
                _ => "❌ -  "
            };

            ListItem::from(Line::from(format!("{}{}", icon, topology)))
                .style(base_style)
        })
        .collect();

    let topology_list = List::new(topology_items)
        .highlight_style(
            Style::default()
                .bg(SELECTED_LIST_COLOR)
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        )
        .highlight_symbol("> ")
        .highlight_spacing(HighlightSpacing::Always);

    // Render all components
    // Note: You'll need to add a state for topology selection in your app cache
    // For now, this assumes you have app.cache.tab2_data.topology_list_state or similar
    ratatui::widgets::StatefulWidget::render(
        topology_list,
        topology_list_area,
        buf,
        &mut app.cache.tab2_data.topology_list_state.clone(),
    );

    // Render the menu block last so it appears as the border
    menu_view_area_block.render(menu_view_area, buf);

    app.cache.text_buffer.toml_text_area.render(editor_area, buf);
    compiler_status.render(inner_compiler_block, buf);
    compiler_block.render(compiler_area,buf);
    toml_block.render(toml_view_area, buf);

}

/// Renders an animated galaxy ASCII art that fills the width of the area
fn render_galaxy_animation(app: &App, area: Rect, buf: &mut Buffer) {
    if area.height < 3 || area.width < 10 {
        return; // Not enough space for animation
    }

    // Get animation frame based on app counter
    let frame = (app.cache.counter / 3) % 12; // Slower animation, 12 frames

    let galaxy_lines = get_galaxy_frame_wide(frame as usize, area.width as usize, area.height as usize);

    // Create animated colors that cycle through galaxy colors
    let color_cycle = (app.cache.counter / 2) % 6;
    let galaxy_color = match color_cycle {
        0 => Color::Rgb(138, 43, 226),  // Blue Violet
        1 => Color::Rgb(75, 0, 130),   // Indigo
        2 => Color::Rgb(255, 20, 147), // Deep Pink
        3 => Color::Rgb(0, 191, 255),  // Deep Sky Blue
        4 => Color::Rgb(148, 0, 211),  // Dark Violet
        _ => Color::Rgb(186, 85, 211), // Medium Orchid
    };

    // Create the galaxy paragraph with animated styling
    let galaxy_text = Paragraph::new(galaxy_lines)
        .style(Style::default()
            .fg(galaxy_color)
            .add_modifier(Modifier::BOLD))
        .centered();

    galaxy_text.render(area, buf);
}

/// Returns galaxy animation frames that adapt to the given width and height
fn get_galaxy_frame_wide(frame: usize, width: usize, height: usize) -> String {
    let base_patterns = [
        vec!["✦", "✧", "⋆", "◦", "✦", "⋆", "✧", "◦"],
        vec!["⋆", "✦", "◦", "✧", "⋆", "✦", "◦", "✧"],
        vec!["✧", "◦", "✦", "⋆", "✧", "◦", "✦", "⋆"],
        vec!["◦", "⋆", "✧", "✦", "◦", "⋆", "✧", "✦"],
        vec!["✦", "⋆", "◦", "✧", "✦", "⋆", "◦", "✧"],
        vec!["⋆", "✧", "✦", "◦", "⋆", "✧", "✦", "◦"],
        vec!["✧", "✦", "⋆", "◦", "✧", "✦", "⋆", "◦"],
        vec!["◦", "✧", "⋆", "✦", "◦", "✧", "⋆", "✦"],
    ];

    let pattern_offset = frame % base_patterns.len();
    let mut lines = Vec::new();

    // Generate lines based on available height
    for line_idx in 0..height {
        let mut line = String::new();
        let pattern = &base_patterns[(pattern_offset + line_idx) % base_patterns.len()];

        // Fill the line with pattern, adjusting for the current frame
        let mut pos = 0;
        while pos < width {
            // Add some randomness based on frame and position
            if (frame + line_idx + pos / 3) % 4 == 0 {
                let pattern_idx = (frame + pos / 2 + line_idx) % pattern.len();
                line.push_str(pattern[pattern_idx]);
                pos += 1;
            } else if (frame + line_idx + pos / 2) % 6 == 0 {
                let pattern_idx = (frame + pos + line_idx * 2) % pattern.len();
                line.push_str(pattern[pattern_idx]);
                pos += 1;
            } else {
                line.push(' ');
                pos += 1;
            }
        }
        lines.push(line);
    }

    lines.join("\n")
}

/// Returns different galaxy animation frames
fn get_galaxy_frame(frame: usize) -> &'static str {
    match frame {
        0 => r#"    ✦       ✧    ⋆
  ⋆    ✦✧✦    ✧  ⋆
✧   ⋆✦✧ ◦ ✧✦⋆   ✧
  ⋆  ✧✦⋆ ◦ ⋆✦✧  ⋆
    ✧   ⋆✦✧⋆   ✦
      ⋆    ✧    ✦    "#,

        1 => r#"  ⋆  ✦       ✧
✧     ✦✧✦    ✧    ⋆
  ⋆ ✦✧ ◦ ✧✦ ✧   ✦
    ✧✦⋆ ◦ ⋆✦✧ ⋆
✦     ⋆✦✧⋆   ✧  ⋆
    ✧   ⋆    ✦   ✧ "#,

        2 => r#"✧     ✦     ⋆  ✧
    ⋆   ✦✧✦      ✦
  ✦✧ ◦ ✧✦ ⋆ ✧
⋆   ✧✦⋆ ◦ ⋆✦✧   ✦
  ✧   ⋆✦✧⋆     ⋆
✦   ⋆    ✧    ✦  ✧"#,

        3 => r#"  ✦   ⋆     ✧    ⋆
✧     ✦✧✦        ✦
    ✦✧ ◦ ✧✦ ✧ ⋆
  ⋆ ✧✦⋆ ◦ ⋆✦✧
✦     ⋆✦✧⋆   ✧
    ✧   ⋆  ✦   ✧ ⋆"#,

        4 => r#"    ⋆ ✦       ✧
  ✧   ✦✧✦    ⋆    ✦
✦   ✧ ◦ ✧✦   ✧ ⋆
    ⋆✦⋆ ◦ ⋆✦✧   ✦
  ✧   ✦✧⋆   ⋆   ✧
⋆     ⋆    ✧    ✦  "#,

        5 => r#"✧       ✦   ⋆    ✧
    ✦   ✧✦    ✧  ⋆
  ⋆ ✧ ◦ ✦✧ ✦   ✧
✦   ✦⋆ ◦ ⋆✧✦ ⋆
    ⋆ ✦✧⋆   ✧   ✦
  ✧     ⋆  ✦   ✧ ⋆"#,

        6 => r#"  ✦ ⋆     ✧      ✦
✧     ✦✧    ⋆    ✧
    ✧ ◦ ✦✧   ✦ ⋆
  ⋆ ✦⋆ ◦ ⋆✧✦   ✧
✦   ⋆ ✧⋆   ✧     ⋆
    ✧   ⋆✦    ✧   ✦"#,

        7 => r#"    ✧ ✦   ⋆      ✧
  ✦   ✧    ✧  ⋆   ✦
⋆   ✧ ◦ ✦   ✧   ⋆
    ✦⋆ ◦ ⋆✧  ✦ ✧
  ✧ ⋆ ✧⋆   ⋆   ✦
✦     ✧ ⋆✦    ✧   "#,

        8 => r#"⋆     ✦ ✧     ⋆   ✦
    ✧     ✧    ⋆ ✧
  ✦ ✧ ◦ ✦   ⋆   ✧
⋆   ⋆ ◦ ⋆✧  ✧   ✦
  ✧   ✧⋆ ⋆   ✦    ⋆
    ✦   ✧ ⋆✦      ✧"#,

        9 => r#"  ✧     ✦ ⋆   ✧
✦   ⋆     ✧  ✦   ⋆
    ✧ ◦ ✦ ⋆   ✧
  ⋆   ◦ ⋆✧ ✦   ✧ ⋆
✦   ✧ ⋆ ⋆ ✦
⋆     ✦   ✧ ⋆  ✧  "#,

        10 => r#"    ✦   ✧ ⋆     ⋆
  ⋆       ✧ ✦   ✧ ✦
✧   ✧ ◦ ✦ ⋆ ✧
    ⋆ ◦ ⋆✧   ✦ ⋆ ✧
  ✦ ✧   ⋆ ✦   ✧
⋆   ✧ ✦     ⋆  ✧  "#,

        _ => r#"✦       ✧ ⋆   ✧
    ⋆ ✦   ✧   ✦ ⋆ ✧
  ✧   ◦ ✦ ⋆ ✧   ⋆
⋆     ◦ ⋆✧ ✦ ⋆ ✧
  ✦ ✧     ✦ ✧
    ✧ ✦ ⋆     ✧ ⋆ "#,
    }
}