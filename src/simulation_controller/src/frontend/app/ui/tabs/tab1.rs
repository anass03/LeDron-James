use std::default::Default;
use ratatui::layout::Rect;
use ratatui::buffer::Buffer;
use ratatui::layout::Alignment::Center;
use crate::frontend::app::App;
use crate::frontend::app::models::tab1_types::{NodePopup, PopupSelection, ViewedTab1};
use crate::frontend::app::ui::tabs::{tab1_client_popup, tab1_drone_popup, tab1_main_page, tab1_server_popup, BG_COLOR, CANVAS_CLIENT_COLOR, CANVAS_DRONE_COLOR, CANVAS_SERVER_COLOR, POPUP_BG_COLOR};
// popup code

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style, Stylize};
use ratatui::widgets;
use ratatui::widgets::{Block, BorderType, Borders, Widget};
use ratatui::widgets::canvas::{Canvas, Circle, Context, Line as CanvasLine,};
use crate::backend::simulation_controller::NetNode;
use crate::frontend::app::topology_algorithm::alg_models::TopologyAlgorithm;

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
    use ratatui::layout::{Direction, Layout, Rect, Constraint};
    use ratatui::prelude::Color;
    use ratatui::style::{Style, Stylize};
    use ratatui::widgets::{Block, BorderType, Borders, Widget};
    use tui_logger::TuiLoggerWidget;
    use crate::frontend::app::App;
    use crate::frontend::app::ui::tabs::{process_log_paragraph, MAIN_BLOCK_BORDERS};

    // For Client & Server
    pub(crate) fn render_cl_sr_data_options(app:&App, area:Rect, buf: &mut Buffer){
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
    // Download / upload
    // Cpu utilize [?]
    // PDR Expected -> BarCHART
    //
}

