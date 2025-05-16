use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Padding, Paragraph, Tabs, Widget, Wrap};
use crate::frontend::app::App;
use crate::frontend::app::model::LogType;

/// Renders Overview tab-page
pub(super) fn render_tab0(app:&App, area: Rect, buf : &mut Buffer){
    let inner_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(60), // CLI / Log messages
        ])
        .split(area);

    let log_paragraph = process_log_paragraph(app);
    log_paragraph.render(inner_columns[1], buf);

    // Statistic Block
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
pub(super) fn process_log_paragraph<'a>(app:&'a App<'a>)->Paragraph<'a>{
    let lines: Vec<Line> = app
        .cache
        .tab0_data.logs_memory
        .iter()
        .map(|(log,msg)| {
            // Single color matching not working
            // Ratatui BUG ? When changing tabs some colored chars stay visible
            // in the frame even in different tabs.
            /*
            match log{
                LogType::WARN => {
                    Line::from(Span::raw(format!("[WARN]\t{}", msg)).style(Style::default().fg(Color::LightYellow)))
                }
                LogType::ERROR => {
                    Line::from(Span::raw(format!("[ERROR]\t{}", msg)).style(Style::default().fg(Color::LightRed)))
                }
                LogType::INFO => {
                    Line::from(Span::raw(format!("[INFO]\t{}", msg)).style(Style::default().fg(Color::White)))
                }
                LogType::DEBUG => {
                    Line::from(Span::raw(format!("[DEBUG]\t{}", msg)).style(Style::default().fg(Color::LightBlue)))
                }
                LogType::FATAL => {
                    Line::from(Span::raw(format!("[ERROR]\t{}", msg)).style(Style::default().fg(Color::Red)))
                }
            }
             */
            Line::from(Span::raw(msg)).style(Style::default().fg(Color::White))
        }
        )
        .collect();
    let mut log_block = Block::default()
        .title("Logs")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    if app.cache.cursor == 1{
        log_block = log_block.border_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    }

    let a = Paragraph::new(lines)
        .block(
            log_block
        )
        .wrap(Wrap { trim: true })
        .scroll((app.cache.tab0_data.log_scroll_offset as u16, 0));
    return a
}