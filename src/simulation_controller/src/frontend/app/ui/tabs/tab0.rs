use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Widget};
use crate::frontend::app::App;
/// Renders Overview tab-page
pub(super) fn render_tab0(app:&App, area: Rect, buf : &mut Buffer){
    let inner_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(60), // CLI / Log messages
        ])
        .split(area);

    //let log_paragraph = tabs::process_log_paragraph(app);
    //log_paragraph.render(inner_columns[1], buf);


    use crate::frontend::app::ui::tabs; // Statistic Block
    use super::super::utils::chart_rendering;

    let mut chart_block = Block::default()
        .borders(Borders::ALL);

    if app.cache.cursor == 0 {
        chart_block = chart_block.border_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    }

    // Log Block


    // Shitty performance, gotta improve it with a Tick Rate
    match app.cache.tab0_data.selected_chart{
        0 => { chart_rendering::render_sparkline_1(&app.cache.tab0_data.charts_data,chart_block,inner_columns[0],buf) },
        1 => {  },
        2 => {  },
        _ => panic!("OVERFLOW CHART SELECTION!")
    }
}

