use std::default::Default;
use ratatui::layout::{Alignment, Rect};
use ratatui::buffer::Buffer;
use crate::frontend::app::App;
use crate::frontend::app::models::tab1_types::{NodePopup, PopupSelection, ViewedTab1};
use crate::frontend::app::ui::tabs::{tab1_client_popup, tab1_drone_popup, tab1_main_page, tab1_server_popup, BG_COLOR, CANVAS_CLIENT_COLOR, CANVAS_DRONE_COLOR, CANVAS_SERVER_COLOR, MAIN_BLOCK_BORDERS, POPUP_BG_COLOR};
// popup code

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Widget, Wrap};
use ratatui::widgets::canvas::{Canvas, Circle, Context, Line as CanvasLine, };
use crate::backend::sc_models::{NetNode, NodeType};
use crate::frontend::app::models::tab1_types::SettingsPopupStatus::{CorrectlySent, Idle};
use crate::frontend::app::topology_algorithm::alg_models::TopologyAlgorithm;
use textwrap;
use tui_logger::TuiLoggerWidget;
use wg_2024::network::NodeId;

pub(crate) fn centered_popup_area(area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(4),
            Constraint::Percentage(92),
            Constraint::Percentage(4),
        ])
        .split(area);

    let vertical_center = popup_layout[1];

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(5),
            Constraint::Percentage(90),
            Constraint::Percentage(5),
        ])
        .split(vertical_center);

    horizontal[1]
}


/// Renders Settings tab-page
pub(in crate::frontend::app::ui::tabs) fn render_tab1(app:&mut App, area: Rect, buf : &mut Buffer)->Option<Rect>{
    match &app.cache.tab1_data.viewing{
        ViewedTab1::Main => {
            // Viewing Topology and shi
            tab1_main_page::render_main_page(app, area, buf);
        }
        ViewedTab1::Popup((id, ndpop)) => {
            tab1_main_page::render_main_page(app, area, buf);
            let popup_area = centered_popup_area(area);
            widgets::Clear.render(popup_area, buf);
            let colored_bg_block = Block::default().style(Style::default().bg(POPUP_BG_COLOR));
            let colored_popup_area = colored_bg_block
                .inner(popup_area);
            colored_bg_block.render(popup_area, buf);

            //let popup = Popup::new(Block::default());
            //popup.render(popup_area, buf);
            match ndpop{
                NodePopup::Client(cl) => {
                    return tab1_client_popup::render_client_popup(app, colored_popup_area, buf, cl, *id);
                }
                NodePopup::Server(sr) => {
                    tab1_server_popup::render_server_popup(app, colored_popup_area, buf, sr, *id);
                }
                NodePopup::Drone(dr) => {
                    tab1_drone_popup::render_drone_popup(app, colored_popup_area, buf, dr, *id);
                }
            }
        }
    }
    None
}

// Utils needed by the single tabs

pub(super) fn draw_map(topology_algorithm_data:&TopologyAlgorithm, area: Rect, buf: &mut Buffer, canvas_block:Block){
    let canvas = Canvas::default()
        .block(canvas_block)
        .background_color(BG_COLOR)
        .x_bounds([0.0, 100.0])
        .y_bounds([0.0, 80.0])
        .paint(|ctx| {
            // Draw topology links
            for &(a, b) in &topology_algorithm_data.map_links {
                if let (Some(na), Some(nb)) = (
                    topology_algorithm_data.map.get(&a),
                    topology_algorithm_data.map.get(&b),
                ) {
                    let line = CanvasLine {
                        x1: na.position.x as f64,
                        y1: na.position.y as f64,
                        x2: nb.position.x as f64,
                        y2: nb.position.y as f64,
                        color: Color::White,
                        ..Default::default()
                    };
                    ctx.draw(&line);
                }
            }
            for (id, pos) in &topology_algorithm_data.map {
                let cx = pos.position.x as f64;
                let cy = pos.position.y as f64;
                let size = 3.0;
                // Draw shape based on node type
                match pos.node{
                    NetNode::Drone(_) => {
                        // Drone: circle
                        let circle = Circle {
                            x: cx,
                            y: cy,
                            radius: size,
                            color: CANVAS_DRONE_COLOR,
                            ..Default::default()
                        };
                        ctx.draw(&circle);
                    }
                    NetNode::Client(_) => {
                        // Client: square
                        let top = CanvasLine { x1: cx - size, y1: cy - size, x2: cx + size, y2: cy - size, color: CANVAS_CLIENT_COLOR, ..Default::default() };
                        let bottom = CanvasLine { x1: cx - size, y1: cy + size, x2: cx + size, y2: cy + size, color: CANVAS_CLIENT_COLOR, ..Default::default() };
                        let left = CanvasLine { x1: cx - size, y1: cy - size, x2: cx - size, y2: cy + size, color: CANVAS_CLIENT_COLOR, ..Default::default() };
                        let right = CanvasLine { x1: cx + size, y1: cy - size, x2: cx + size, y2: cy + size, color: CANVAS_CLIENT_COLOR, ..Default::default() };
                        ctx.draw(&top);
                        ctx.draw(&bottom);
                        ctx.draw(&left);
                        ctx.draw(&right);
                    }
                    NetNode::Server(_) => {
                        // Server: diamond
                        let d = size;
                        let d1 = CanvasLine { x1: cx, y1: cy - d, x2: cx + d, y2: cy, color: CANVAS_SERVER_COLOR, ..Default::default() };
                        let d2 = CanvasLine { x1: cx + d, y1: cy, x2: cx, y2: cy + d, color: CANVAS_SERVER_COLOR, ..Default::default() };
                        let d3 = CanvasLine { x1: cx, y1: cy + d, x2: cx - d, y2: cy, color: CANVAS_SERVER_COLOR, ..Default::default() };
                        let d4 = CanvasLine { x1: cx - d, y1: cy, x2: cx, y2: cy - d, color: CANVAS_SERVER_COLOR, ..Default::default() };
                        ctx.draw(&d1);
                        ctx.draw(&d2);
                        ctx.draw(&d3);
                        ctx.draw(&d4);
                    }
                }
                // Draw node ID at center
                ctx.print(cx, cy, format!("{}", id));
            }
        });
    // Render the canvas into the allocated area
    canvas.render(area, buf);
}

pub(super) fn get_settings_canvas<'a>(app:&App, loc_node:NetNode) -> Canvas<'a, fn(&mut Context)>{
    // Draw settings icon
    Canvas::default()
        .background_color(POPUP_BG_COLOR)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(
                match loc_node{
                    NetNode::Drone(_) | NetNode::Server(_) => {
                        if app.cache.tab1_data.popup_cursor == 1 && app.cache.tab1_data.popup_selection == PopupSelection::MenuArea {
                            Style::default().fg(app.animation_color())
                        }else{
                            Style::default()
                        }
                    }
                    NetNode::Client(_) => {
                        if app.cache.tab1_data.popup_cursor == 3 && app.cache.tab1_data.popup_selection == PopupSelection::MenuArea {
                            Style::default().fg(app.animation_color())
                        }else{
                            Style::default()
                        }
                    }
                }
                )
            .title("Settings"))
        .x_bounds([0.0, 20.0])
        .y_bounds([0.0, 20.0])
        .paint(|ctx| {
            let cx = 10.0;
            let cy = 10.0;
            let r = 6.0;
            // gear outer circle
            ctx.draw(&Circle { x: cx, y: cy, radius: r, color: Color::White, ..Default::default() });
            // gear spokes
            let spoke = 4.0;
            for &(dx, dy) in &[(0.0, spoke), (spoke, 0.0), (0.0, -spoke), (-spoke, 0.0)] {
                ctx.draw(&CanvasLine {
                    x1: cx, y1: cy,
                    x2: cx + dx, y2: cy + dy,
                    color: Color::White,
                    ..Default::default()
                });
            }
        })
}
pub(super) fn get_data_canvas<'a>(app:&App, loc_node:NetNode) -> Canvas<'a, fn(&mut Context)>{
    Canvas::default()
        .background_color(POPUP_BG_COLOR)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(
                match loc_node{
                NetNode::Drone(_) | NetNode::Server(_) => {
                    if app.cache.tab1_data.popup_cursor == 0 && app.cache.tab1_data.popup_selection == PopupSelection::MenuArea {
                        Style::default().fg(app.animation_color())
                    }else{
                        Style::default()
                    }
                }
                NetNode::Client(_) => {
                    if app.cache.tab1_data.popup_cursor == 2 && app.cache.tab1_data.popup_selection == PopupSelection::MenuArea {
                        Style::default().fg(app.animation_color())
                    }else{
                        Style::default()
                    }
                }
            })
            .title("Data"))
        .x_bounds([0.0, 20.0])
        .y_bounds([0.0, 20.0])
        .paint(|ctx| {
            let steps = 40;
            for i in 0..steps {
                let x1 = i         as f64 * 20.0 / (steps - 1) as f64;
                let y1 = 10.0 + 8.0 * ((std::f64::consts::PI * i as f64 / (steps - 1) as f64)).sin();
                let x2 = (i + 1)   as f64 * 20.0 / (steps - 1) as f64;
                let y2 = 10.0 + 8.0 * ((std::f64::consts::PI * (i + 1) as f64 / (steps - 1) as f64)).sin();
                ctx.draw(&CanvasLine { x1, y1, x2, y2, color: Color::White, ..Default::default() });
            }
        })
}

pub(super) mod data_rendering{
    use ratatui::buffer::Buffer;
    use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
    use ratatui::prelude::Color;
    use ratatui::style::{Modifier, Style, Stylize};
    use ratatui::symbols;
    use ratatui::symbols::Marker;
    use ratatui::widgets::{Block, BorderType, Borders, Widget, Paragraph, Bar, BarChart, BarGroup, Axis, Dataset, Chart, GraphType};
    use ratatui::text::{Line, Span};
    use tui_logger::TuiLoggerWidget;
    use wg_2024::network::NodeId;
    use crate::backend::sc_models::NodeStatistics;
    use crate::frontend::app::App;
    use crate::frontend::app::ui::tabs::{MAIN_BLOCK_BORDERS, POPUP_BG_COLOR};
    use crate::frontend::app::ui::tabs::tab1::render_data_logging_section;

    // Function to render PDR bar chart
    fn render_pdr_bar_chart(app: &App, stats: &NodeStatistics, area: Rect, buf: &mut Buffer) {
        if stats.pdr_expected.is_empty() {
            let no_data_text = Paragraph::new("No PDR data available")
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);
            no_data_text.render(area, buf);
            return;
        }

        // Convert and sort PDR data
        let mut pdr_data: Vec<(NodeId, f64)> = stats.pdr_expected.iter()
            .map(|(node_id, pdr)| (*node_id, *pdr))
            .collect();
        pdr_data.sort_by(|a, b| a.0.cmp(&b.0));

        // Calculate number of bars and dynamic sizing
        let bars_count = pdr_data.len() as u16;
        let max_gap = 1u16;
        let total_gaps = if bars_count > 1 {
            max_gap * (bars_count - 1)
        } else { 0 };
        // Determine bar width to fill available area (subtracting padding)
        let content_width = area.width.saturating_sub(8) as u16;
        let mut bar_width = if bars_count > 0 {
            content_width.saturating_sub(total_gaps).checked_div(bars_count).unwrap_or(1)
        } else { 1 };
        if bar_width == 0 {
            bar_width = 1;
        }
        let bar_gap = max_gap;

        // Handle rotation if too many bars
        let max_visible = ((content_width + bar_gap) as usize) / ((bar_width + bar_gap) as usize);
        let (display_data, start_index, pdr_data_len) = if pdr_data.len() <= max_visible {
            (pdr_data.clone(), 0, pdr_data.len())
        } else {
            let start_index = (app.cache.counter as usize / 50) % pdr_data.len();
            let end_index = (start_index + max_visible).min(pdr_data.len());
            (pdr_data[start_index..end_index].to_vec(), start_index, pdr_data.len())
        };

        // Title
        let title = if pdr_data_len > max_visible {
            format!("PDR ({}/{})", display_data.len(), pdr_data_len)
        } else {
            "PDR by Node".to_string()
        };
        let rotation_needed = pdr_data_len > max_visible;

        // Layout: title, chart, optional rotation
        let [title_area, chart_area, rotation_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(if rotation_needed { 1 } else { 0 }),
            ])
            .areas(area);

        // Render title
        Paragraph::new(title)
            .style(Style::default().fg(MAIN_BLOCK_BORDERS).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .render(title_area, buf);

        // Center the chart horizontally
        let total_bars_width = bars_count * bar_width + total_gaps;
        let horizontal_padding = chart_area
            .width
            .saturating_sub(total_bars_width)
            / 2;
        let padded = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(horizontal_padding),
                Constraint::Length(total_bars_width),
                Constraint::Min(0),
            ])
            .split(chart_area);

        // Prepare bars
        let bars: Vec<Bar> = display_data.iter().map(|(node_id, pdr)| {
            let style = pdr_color_style(*pdr);
            Bar::default()
                .value((*pdr * 100.0) as u64)
                .label(Line::from(format!("N{}", node_id)))
                .text_value(format!("{:.0}%", *pdr * 100.0))
                .style(style)
                .value_style(style.reversed())
        }).collect();

        // Render bar chart
        BarChart::default()
            .data(BarGroup::default().bars(&bars))
            .bar_width(bar_width)
            .bar_gap(bar_gap)
            .bar_set(ratatui::symbols::bar::Set::default())
            .render(padded[1], buf);

        // Rotation indicator
        if rotation_needed {
            let progress = ((start_index * 100) / pdr_data_len).min(100);
            Paragraph::new(format!("Rotating... ({}%)", progress))
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center)
                .render(rotation_area, buf);
        }
    }

    // Function to render download speed chart
    pub fn render_download_speed_chart(app: &App, stats: &Vec<f32>, area: Rect, buf: &mut Buffer) {
        // Fit to available width
        let max_points = area.width as usize;
        let total = stats.len();
        let start = total.saturating_sub(max_points);
        let display: Vec<f32> = stats[start..].iter().cloned().collect();

        // Convert to (x, y) pairs
        let data: Vec<(f64, f64)> = display
            .iter()
            .enumerate()
            .map(|(i, &v)| (i as f64, v as f64))
            .collect();

        // Compute Y-axis bounds
        let y_min = display.iter().cloned().fold(f32::INFINITY, f32::min) as f64;
        let y_max = display.iter().cloned().fold(f32::NEG_INFINITY, f32::max) as f64;

        // Build the line dataset (continuous line in white)
        let line_dataset = Dataset::default()
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::White))
            .data(&data);

        // Build the point dataset (red dots)
        let point_dataset = Dataset::default()
            .graph_type(GraphType::Scatter)
            .marker(Marker::Dot)
            .style(Style::default().fg(Color::Red))
            .data(&data);

        // Collect both datasets
        let datasets = vec![line_dataset, point_dataset];

        // Create the chart, applying background fill of the popup color
        let chart = Chart::new(datasets)
            .style(Style::default().bg(POPUP_BG_COLOR))
            .x_axis(
                Axis::default()
                    .bounds([0.0, (data.len().max(1)) as f64])
                    .labels::<Vec<Line>>(vec![]),
            )
            .y_axis(
                Axis::default()
                    .title("MB/s") // unit label under the highest tick
                    .bounds([y_min, y_max])
                    .labels(vec![
                        Line::from(format!("{:.1}", y_min)),
                        Line::from(format!("{:.1}", y_max)),
                    ]),
            );

        chart.render(area, buf);
    }

    // Helper function to create temperature-like color style based on PDR value
    fn pdr_color_style(pdr: f64) -> Style {
        let color = if pdr >= 0.8 {
            Color::Rgb(0, 255, 0) // Green for high PDR
        } else if pdr >= 0.6 {
            Color::Rgb(255, 255, 0) // Yellow for medium-high PDR
        } else if pdr >= 0.4 {
            Color::Rgb(255, 165, 0) // Orange for medium PDR
        } else if pdr >= 0.2 {
            Color::Rgb(255, 69, 0) // Red-orange for low-medium PDR
        } else {
            Color::Rgb(255, 0, 0) // Red for low PDR
        };
        Style::new().fg(color)
    }

    // Helper function to create color style based on download speed value
    fn download_speed_color_style(speed: f64) -> Style {
        let color = if speed >= 80.0 {
            Color::Rgb(0, 255, 0) // Green for high speed
        } else if speed >= 60.0 {
            Color::Rgb(255, 255, 0) // Yellow for medium-high speed
        } else if speed >= 40.0 {
            Color::Rgb(255, 165, 0) // Orange for medium speed
        } else if speed >= 20.0 {
            Color::Rgb(255, 69, 0) // Red-orange for low-medium speed
        } else {
            Color::Rgb(255, 0, 0) // Red for low speed
        };
        Style::new().fg(color)
    }

    // For Client & Server
    // If mode is 0 is Client, if is 1 is Server
    pub(crate) fn render_cl_sr_data_options(app:&App, area:Rect, buf: &mut Buffer, id:NodeId, mode:u8){
        let [stats_area, filtered_logging_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .areas(area);
        let [data_1, data_2, data_3] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(30), Constraint::Percentage(30)])
            .areas(stats_area);

        // Expected PDR
        let data_1_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default() .fg(
                if app.cache.tab1_data.popup_cursor == 0 {
                    app.animation_color()
                }else{
                    MAIN_BLOCK_BORDERS
                }
            ));
        let inner_data_1_block = data_1_block.inner(data_1);
        data_1_block.render(data_1, buf);

        // DOWNLOAD AT SEC IF CURSOR
        let data_2_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(
                if app.cache.tab1_data.popup_cursor == 1 {
                    app.animation_color()
                }else{
                    MAIN_BLOCK_BORDERS
                }
            ));
        let inner_data_2_block = data_2_block.inner(data_2);
        data_2_block.render(data_2, buf);

        // ROUTE
        let data_3_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(
                if app.cache.tab1_data.popup_cursor == 2 {
                    app.animation_color()
                }else{
                    MAIN_BLOCK_BORDERS
                }
            ));

        let inner_data_3_block = data_3_block.inner(data_3);
        data_3_block.render(data_3, buf);


        match app.simulation_controller_data.stats.node_statistic.get(&id){
            None => {
                // Render inside the graph blocks the text a canvas drawing something that says NOT VALID .
                let no_data_text = Paragraph::new("No data available for this node")
                    .style(Style::default().fg(Color::Gray))
                    .alignment(ratatui::layout::Alignment::Center);
                no_data_text.render(inner_data_1_block, buf);
                
                // Also render no data for data_2_block
                let no_data_text_2 = Paragraph::new("No download data available")
                    .style(Style::default().fg(Color::Gray))
                    .alignment(ratatui::layout::Alignment::Center);
                no_data_text_2.render(inner_data_2_block, buf);
            }
            Some(stats) => {
                // Render PDR bar chart in data_1_block
                render_pdr_bar_chart(app, stats, inner_data_1_block, buf);
                // Render download speed chart in data_2_block
                render_download_speed_chart(app, &stats.download_at_sec, inner_data_2_block, buf);
            }
        }




        let logging_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(
                if app.cache.tab1_data.popup_cursor == 3 {
                    app.animation_color()
                }else{
                    MAIN_BLOCK_BORDERS
                }
            ));

        let inner_log_block = logging_block.inner(filtered_logging_area);
        logging_block.render(filtered_logging_area, buf);

        render_data_logging_section(app, id, inner_log_block, buf);
    }
    pub(crate) fn render_drone_data_options(app:&App, area:Rect, buf: &mut Buffer, id:NodeId){
        let [stats_area, filtered_logging_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .spacing(1)
            .areas(area);
        let [data_1, data_2, data_3] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(33), Constraint::Percentage(34), Constraint::Percentage(33)])
            .spacing(1)
            .areas(stats_area);

        let data_1_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default() .fg(
                if app.cache.tab1_data.popup_cursor == 0 {
                    app.animation_color()
                }else{
                    MAIN_BLOCK_BORDERS
                }
            ));
        let inner_data_1_block = data_1_block.inner(data_1);
        data_1_block.render(data_1, buf);
        // Download Speed

        let data_2_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(
                if app.cache.tab1_data.popup_cursor == 1 {
                    app.animation_color()
                }else{
                    MAIN_BLOCK_BORDERS
                }
            ));
        let inner_data_2_block = data_2_block.inner(data_2);
        data_2_block.render(data_2, buf);
        // CPU Utilize

        let data_3_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(
                if app.cache.tab1_data.popup_cursor == 2 {
                    app.animation_color()
                }else{
                    MAIN_BLOCK_BORDERS
                }
            ));

        let inner_data_3_block = data_3_block.inner(data_3);
        data_3_block.render(data_3, buf);
        // Bar Chart, most received packets type

        // Handle drone data display
        match app.simulation_controller_data.stats.node_statistic.get(&id){
            None => {
                // Render inside the graph blocks the text a canvas drawing something that says NOT VALID .
                let no_data_text = Paragraph::new("No data available for this drone")
                    .style(Style::default().fg(Color::Gray))
                    .alignment(ratatui::layout::Alignment::Center);
                no_data_text.render(inner_data_1_block, buf);
                
                // Also render no data for data_2_block
                let no_data_text_2 = Paragraph::new("No download data available")
                    .style(Style::default().fg(Color::Gray))
                    .alignment(ratatui::layout::Alignment::Center);
                no_data_text_2.render(inner_data_2_block, buf);
            }
            Some(stats) => {
                // Render PDR bar chart in data_1_block for drones too
                render_pdr_bar_chart(app, stats, inner_data_1_block, buf);
                
                // Render download speed chart in data_2_block
                //render_download_speed_chart(app, stats, inner_data_2_block, buf);
            }
        }

        let logging_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(
                if app.cache.tab1_data.popup_cursor == 3 {
                    app.animation_color()
                }else{
                    MAIN_BLOCK_BORDERS
                }
            ));

        let inner_log_block = logging_block.inner(filtered_logging_area);
        logging_block.render(filtered_logging_area, buf);

        render_data_logging_section(app, id, inner_log_block, buf);
    }

}


// Popup border color based on status
pub fn popup_border_color(status: &crate::frontend::app::models::tab1_types::SettingsPopupStatus) -> Color {
    use crate::frontend::app::models::tab1_types::SettingsPopupStatus::*;
    match status {
        CorrectlySent => Color::Green,
        Error(_) => Color::Red,
        Idle => MAIN_BLOCK_BORDERS,
    }
}

// Returns a smaller centered popup area
pub fn small_centered_popup_area(area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(area);
    let vertical_center = popup_layout[1];
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .split(vertical_center);
    horizontal[1]
}

pub(super) fn render_add_sender_popup(app: &App, area: Rect, buf: &mut Buffer, cmd: &crate::frontend::app::models::tab1_types::CommandPopup) {
    let popup_area = small_centered_popup_area(area);
    widgets::Clear.render(popup_area, buf);
    let block = Block::default()
        .title("Add Sender")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(popup_border_color(&cmd.status)));
    let inner = block.inner(popup_area);
    block.render(popup_area, buf);
    let [input_area, button_area, status_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .areas(inner);
    let mut nodeid_block = Block::default()
        .title("Node ID (u8)")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(
            if cmd.cursor == 0{
                app.animation_color()
            }else{
                MAIN_BLOCK_BORDERS
            }));
    let inner_nodeid = nodeid_block.inner(input_area);
    nodeid_block.render(input_area, buf);
    app.cache.text_buffer.inputfirst_text_area.render(inner_nodeid, buf);
    let send_button = Paragraph::new("[ Send ]")
        .alignment(Alignment::Center)
        .style(Style::default().fg(
            if cmd.cursor == 1{
                app.animation_color()
            }else{
                MAIN_BLOCK_BORDERS
            }
        ).add_modifier(Modifier::BOLD));
    send_button.render(button_area, buf);
    match &cmd.status {
        crate::frontend::app::models::tab1_types::SettingsPopupStatus::CorrectlySent => {
            let msg = Paragraph::new("Correctly sent!")
                .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center);
            msg.render(status_area, buf);
        },
        crate::frontend::app::models::tab1_types::SettingsPopupStatus::Error(e) => {
            // Wrap long error messages to fit in multiple rows
            let error_text = format!("Error: {}", e);
            let wrapped_lines = textwrap::wrap(&error_text, status_area.width as usize);
            let lines: Vec<Line> = wrapped_lines.iter()
                .map(|line| Line::from(line.to_string())
                    .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)))
                .collect();
            let msg = Paragraph::new(lines)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            msg.render(status_area, buf);
        },
        _ => {},
    }
}

pub(super) fn render_remove_sender_popup(app: &App, area: Rect, buf: &mut Buffer, cmd: &crate::frontend::app::models::tab1_types::CommandPopup) {
    let popup_area = small_centered_popup_area(area);
    widgets::Clear.render(popup_area, buf);
    let block = Block::default()
        .title("Remove Sender")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(popup_border_color(&cmd.status)));
    let inner = block.inner(popup_area);
    block.render(popup_area, buf);
    let [input_area, button_area, status_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .areas(inner);
    let mut nodeid_block = Block::default()
        .title("Node ID (u8)")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(
            if cmd.cursor == 0{
                app.animation_color()
            }else{
                MAIN_BLOCK_BORDERS
            }
        ));
    let inner_nodeid = nodeid_block.inner(input_area);
    nodeid_block.render(input_area, buf);
    app.cache.text_buffer.inputfirst_text_area.render(inner_nodeid, buf);
    let send_button = Paragraph::new("[ Send ]")
        .alignment(Alignment::Center)
        .style(Style::default().fg(
            if cmd.cursor == 1{
                app.animation_color()
            }else{
                MAIN_BLOCK_BORDERS
            }
        ).add_modifier(Modifier::BOLD));
    send_button.render(button_area, buf);
    match &cmd.status {
        crate::frontend::app::models::tab1_types::SettingsPopupStatus::CorrectlySent => {
            let msg = Paragraph::new("Correctly sent!")
                .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center);
            msg.render(status_area, buf);
        },
        crate::frontend::app::models::tab1_types::SettingsPopupStatus::Error(e) => {
            // Wrap long error messages to fit in multiple rows
            let error_text = format!("Error: {}", e);
            let wrapped_lines = textwrap::wrap(&error_text, status_area.width as usize);
            let lines: Vec<Line> = wrapped_lines.iter()
                .map(|line| Line::from(line.to_string())
                    .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)))
                .collect();
            let msg = Paragraph::new(lines)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            msg.render(status_area, buf);
        },
        _ => {},
    }
}

pub(crate) fn render_data_logging_section(app: &App, id: NodeId, inner_log_block: Rect, buf: &mut Buffer) {
    // Determina se è un drone, client o server controllando i vettori
    let node_type = if app.simulation_controller_data.drones.iter().any(|d| d.id == id) {
        "drone"
    } else if app.simulation_controller_data.clients.iter().any(|c| c.id == id) {
        "client"
    } else if app.simulation_controller_data.servers.iter().any(|s| s.id == id) {
        "server"
    } else {
        "unknown"
    };

    match node_type {
        "drone" => {
            // Per i DRONI: mostra i log generali + log specifici del drone
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
                .state(&app.cache.main_log_state.1)
                .render(inner_log_block, buf);
        }
        _ => {
            // Per CLIENT e SERVER: usa il host_log_state specifico se esiste, altrimenti main_log_state
            match app.simulation_controller_data.host_log_state.get(&id) {
                Some((_, log_filter)) => {
                    TuiLoggerWidget::default()
                        .style_error(Style::default().fg(Color::Red))
                        .style_warn(Style::default().fg(Color::Yellow))
                        .style_info(Style::default().fg(Color::White))
                        .style_debug(Style::default().fg(Color::Green))
                        .style_trace(Style::default().fg(Color::Magenta))
                        .output_separator(' ')
                        .output_timestamp(Some("%H:%M:%S%.3f".to_string()))
                        .output_target(true)  // Mostra target per debug
                        .output_file(false)
                        .output_line(false)
                        .state(log_filter)
                        .render(inner_log_block, buf);
                }
                None => {
                    // Fallback al main_log_state se non c'è host_log_state
                    TuiLoggerWidget::default()
                        .style_error(Style::default().fg(Color::Red))
                        .style_warn(Style::default().fg(Color::Yellow))
                        .style_info(Style::default().fg(Color::White))
                        .style_debug(Style::default().fg(Color::Green))
                        .style_trace(Style::default().fg(Color::Magenta))
                        .output_separator(' ')
                        .output_timestamp(Some("%H:%M:%S%.3f".to_string()))
                        .output_target(true)
                        .output_file(false)
                        .output_line(false)
                        .state(&app.cache.main_log_state.1)
                        .render(inner_log_block, buf);
                }
            }
        }
        /*_ => {
            // Nodo sconosciuto - mostra messaggio di errore
            let error_text = Paragraph::new("Unknown node type - no logs available")
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center);
            error_text.render(inner_log_block, buf);
        }*/
    }
}
