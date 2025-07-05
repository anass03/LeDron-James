use std::env::SplitPaths;
use log::Level;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Widget};
use tui_logger::{ExtLogRecord, LogFormatter, TuiLoggerSmartWidget, TuiLoggerWidget};
use wg_2024::network::NodeId;
use crate::frontend::app::App;
use crate::frontend::app::ui::tabs::MAIN_BLOCK_BORDERS;

/// Renders Overview tab-page
pub(super) fn render_tab0(app:&App, area: Rect, buf : &mut Buffer){
    let [left_area,right_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(70), // CLI / Log messages
        ])
        .areas(area);
    let [help_area,log_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .areas(right_area);
    let [graph1, graph2, graph3] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(33), Constraint::Percentage(34), Constraint::Percentage(33)])
        .areas(left_area);

/*    let log_paragraph = tabs::process_log_paragraph(app);
    log_paragraph.render(inner_columns[1], buf);
*/

    use crate::frontend::app::ui::tabs; // Statistic Block
    use super::super::utils::chart_rendering;

    let mut chart1_block = Block::default()
        .title(" Dropped Messages ")
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            if app.cache.cursor == 0 {
                app.animation_color()
            }else{
                MAIN_BLOCK_BORDERS
            }
        ));
    chart_rendering::render_sparkline_1(&app.cache.tab0_data.charts_data,chart1_block,graph1,buf);

    let mut chart2_block = Block::default()
        .title(" Sent Messages ")
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            if app.cache.cursor == 1 {
                app.animation_color()
            }else{
                MAIN_BLOCK_BORDERS
            }
        ));
    chart2_block.render(graph2,buf);

    let mut chart3_block = Block::default()
        .title(" Most active nodes ")
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            if app.cache.cursor == 2 {
                app.animation_color()
            }else{
                MAIN_BLOCK_BORDERS
            }
        ));
    chart3_block.render(graph3,buf);



    // Shitty performance, gotta improve it with a Tick Rate/*
    /*match app.cache.tab0_data.selected_chart{
        0 => { chart_rendering::render_sparkline_1(&app.cache.tab0_data.charts_data,chart_block,graph_area,buf) },
        1 => {  },
        2 => {  },
        _ => panic!("OVERFLOW CHART SELECTION!")
    }*/

    let log_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(
            if app.cache.cursor == 3 {
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
        .opt_formatter(Some(Box::new(MyLogFormatter{})))
        .output_separator(' ')
        .output_timestamp(Some("%H:%M:%S".to_string()))
        .output_target(false)
        .output_file(false)
        .output_line(false)
        .state(&app.cache.main_log_state.1)
        .render(inner_log_block, buf);
}

struct MyLogFormatter {}
impl LogFormatter for MyLogFormatter {
    fn min_width(&self) -> u16 {
        4
    }
    fn format(&self, _width: usize, evt: &ExtLogRecord) -> Vec<Line> {
        let mut lines = vec![];
        let identifier_str = match parse_node_string(evt.target()){
            None => {
                format!("Unknown Target")
            }
            Some((id,node)) => {
                format!("{:?} ID: {:?}", node,id)
            }
        };

        match evt.level {
            log::Level::Error => {
                lines.push(
                    Line::from(vec![Span::styled(format!("{} | [{:?}] ERR | {}",evt.timestamp.format("%H:%M:%S"), evt.level, evt.msg()),
                                                 Style::default().fg(Color::Rgb(255, 168, 168))
                    )])
                );
            }
            log::Level::Debug => {
                lines.push(
                    Line::from(vec![Span::styled(format!("{} | [{:?}] DBG | {}",evt.timestamp.format("%H:%M:%S"), evt.level, evt.msg()),
                                            Style::default().fg(Color::Rgb(201, 255, 246))

                    )])
                );
            }
            Level::Warn => {
                lines.push(
                    Line::from(vec![Span::styled(format!("{} | [{:?}] WARN | {}",evt.timestamp.format("%H:%M:%S"), evt.level, evt.msg()),
                                                 Style::default().fg(Color::Rgb(253, 215, 170))
                    )])
                );
            }
            Level::Info => {
                lines.push(
                    Line::from(vec![Span::styled(format!("{} | [{:?}] INFO | {}",evt.timestamp.format("%H:%M:%S"), evt.level, evt.msg()),
                                                 Style::default().fg(Color::Rgb(182, 255, 206))
                    )])                    );
            }
            Level::Trace => {
                lines.push(
                    Line::from(vec![Span::styled(format!("{} | [{:?}] GEN | {}",evt.timestamp.format("%H:%M:%S"), evt.level, evt.msg()),
                                                 Style::default().fg(Color::Rgb(237, 254, 173))
                    )])
                );
            }
        };
        lines
    }
}
fn parse_node_string(input: &str) -> Option<(NodeId, &str)> {
    let parts: Vec<&str> = input.split('_').collect();

    if parts.len() != 2 {
        return None;
    }

    let node_id = parts[0].parse::<u8>().ok()?;
    let role = parts[1];

    match role {
        "Server" | "Client" => Some((node_id, role)),
        _ => None,
    }
}


