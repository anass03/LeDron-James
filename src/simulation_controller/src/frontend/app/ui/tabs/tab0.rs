use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Widget};
use tui_logger::{TuiLoggerSmartWidget, TuiLoggerWidget};
use crate::frontend::app::App;
use crate::frontend::app::ui::tabs::MAIN_BLOCK_BORDERS;

/// Renders Overview tab-page
pub(super) fn render_tab0(app:&App, area: Rect, buf : &mut Buffer){
    let [graph_area,log_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(60), // CLI / Log messages
        ])
        .areas(area);

/*    let log_paragraph = tabs::process_log_paragraph(app);
    log_paragraph.render(inner_columns[1], buf);
*/

    use crate::frontend::app::ui::tabs; // Statistic Block
    use super::super::utils::chart_rendering;

    let mut chart_block = Block::default()
        .borders(Borders::ALL);

    if app.cache.cursor == 0 {
        chart_block = chart_block.border_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    }
    // Shitty performance, gotta improve it with a Tick Rate
    match app.cache.tab0_data.selected_chart{
        0 => { chart_rendering::render_sparkline_1(&app.cache.tab0_data.charts_data,chart_block,graph_area,buf) },
        1 => {  },
        2 => {  },
        _ => panic!("OVERFLOW CHART SELECTION!")
    }

    let log_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(
            if app.cache.cursor == 1 {
                app.animation_color()
            }else{
                MAIN_BLOCK_BORDERS
            }
        ))
        .border_type(BorderType::Rounded);
    let inner_log_block = log_block.inner(log_area);
    log_block.render(log_area, buf);

    TuiLoggerWidget::default()
        .style_error(Style::default().fg(Color::Red))
        .style_warn(Style::default().fg(Color::Yellow))
        .style_info(Style::default().fg(Color::White))
        .style_debug(Style::default().fg(Color::Green))
        .style_trace(Style::default().fg(Color::Magenta))
        .output_separator(' ')
        .output_timestamp(Some("%H:%M:%S%.3f".to_string()))
        .output_target(false)
        .output_file(false)
        .output_line(false)
        .state(&app.cache.main_log_state)
        .render(inner_log_block, buf);

}

