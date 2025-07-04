use ratatui::widgets::{Block, BorderType, Borders, HighlightSpacing, List, ListItem};
use ratatui::prelude::{Color, Line, Modifier, Span, StatefulWidget, Style};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::buffer::Buffer;
use crate::frontend::app::App;
use crate::frontend::app::models::general_types::Node;
use crate::frontend::app::topology_algorithm::alg_models::TopologyStatus;
use crate::frontend::app::ui::tabs::{ALTERNATE_LIST_COLOR, BG_COLOR, CANVAS_CLIENT_COLOR, CANVAS_DRONE_COLOR, CANVAS_SERVER_COLOR, MAIN_BLOCK_BORDERS, SELECTED_LIST_COLOR};
use crate::frontend::app::ui::tabs::tab1::draw_map;

pub(in crate::frontend::app::ui::tabs) fn render_main_page(app: &App, area: Rect, buf: &mut Buffer){
    let [top_area,log_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
        .areas(area);
    let [list_area, map_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
        .areas(top_area);

    // Selection Menu
    // Render Nodes list on left column – styled after the ratatui list example
    let mut nodes_block = Block::default()
        .title(Span::styled(" Available Nodes ", Style::default().add_modifier(Modifier::BOLD)))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(MAIN_BLOCK_BORDERS));

    if app.cache.cursor == 0 {
        nodes_block = nodes_block.border_style(Style::default().fg(app.animation_color()));
    }

    // Collect items to display – alternate row colors for zebra‑striping
    let items: Vec<ListItem> = app
        .cache
        .tab1_data
        .netnode_list
        .items
        .iter()
        .enumerate()
        .map(|(idx, (n, _, _))| {
            let base_style = if idx % 2 == 0 {
                // even rows – brighter foreground
                Style::default().bg(BG_COLOR)
            } else {
                // odd rows – dimmer foreground for contrast
                Style::default().bg(ALTERNATE_LIST_COLOR)
            };
            ListItem::from(n).style(base_style)
        })
        .collect();

    // Build the list widget
    let nodes_list = List::new(items)
        .block(nodes_block)
        // Keep a cohesive palette: selected rows use the global BG_COLOR with a bright accent
        .highlight_style(
            Style::default()
                .bg(SELECTED_LIST_COLOR)
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">")
        .highlight_spacing(HighlightSpacing::Always);

    // Render the list
    ratatui::widgets::StatefulWidget::render(
        nodes_list,
        list_area,
        buf,
        &mut app.cache.tab1_data.netnode_list.state.clone(),
    );


    // Drawing canvas section
    let mut canvas_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(MAIN_BLOCK_BORDERS))
        .border_type(BorderType::Rounded)
        .title_alignment(Alignment::Center);

    if app.cache.cursor == 1 {
        canvas_block = canvas_block.border_style(Style::default().fg(app.animation_color()));
    }


    match app.simulation_controller_data.topology_algorithm_data.status{
        TopologyStatus::Stopped => {
            canvas_block = canvas_block
                .title(vec![Span::styled("MAP: ", Style::default().add_modifier(Modifier::BOLD|Modifier::ITALIC)),
                                   Span::styled(" Simulation is stopped, re-run to view the topology ", Style::default().add_modifier(Modifier::BOLD|Modifier::ITALIC).fg(Color::Rgb(222, 50, 76)))]);

        }
        TopologyStatus::ToInitialize => {
            canvas_block = canvas_block
                .title(vec![Span::styled("MAP: ", Style::default().add_modifier(Modifier::BOLD|Modifier::ITALIC)),
                                   Span::styled(" Initialize the topology [ Press P ] ", Style::default().add_modifier(Modifier::BOLD|Modifier::ITALIC).fg(Color::Rgb(244, 137, 95)))]);
        }
        TopologyStatus::Computing => {
/*            canvas_block = canvas_block
                .title(Span::styled(" MAP: Computing best graph representation  ",
                                           Style::default().add_modifier(Modifier::BOLD|Modifier::ITALIC)))*/
            canvas_block = canvas_block
                .title(vec![Span::styled("MAP: ", Style::default().add_modifier(Modifier::BOLD|Modifier::ITALIC)),
                                   Span::styled(" Computing best graph representation ", Style::default().add_modifier(Modifier::BOLD|Modifier::ITALIC).fg(Color::Rgb(248, 225, 111)))]);
        }
        TopologyStatus::Converged => {
/*            canvas_block = canvas_block
                .title(Span::styled(" MAP: Converged ",
                                           Style::default().add_modifier(Modifier::BOLD|Modifier::ITALIC)))*/
            canvas_block = canvas_block
                .title(vec![Span::styled("MAP: ", Style::default().add_modifier(Modifier::BOLD|Modifier::ITALIC)),
                                   Span::styled(" Converged ", Style::default().add_modifier(Modifier::BOLD|Modifier::ITALIC).fg(Color::Rgb(149, 207, 146)))]);

        }
    }
    draw_map(&app.simulation_controller_data.topology_algorithm_data, map_area, buf, canvas_block);
/*    let canvas = Canvas::default()
        .block(canvas_block)
        .background_color(BG_COLOR)
        .x_bounds([0.0, 100.0])
        .y_bounds([0.0, 80.0])
        .paint(|ctx| {
            // Draw topology links
            for &(a, b) in &app.simulation_controller_data.topology_algorithm_data.map_links {
                if let (Some(na), Some(nb)) = (
                    app.simulation_controller_data.topology_algorithm_data.map.get(&a),
                    app.simulation_controller_data.topology_algorithm_data.map.get(&b),
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
            for (id, pos) in &app.simulation_controller_data.topology_algorithm_data.map {
                let cx = pos.position.x as f64;
                let cy = pos.position.y as f64;
                let size = 3.0;
                // Draw shape based on node type
                if app.simulation_controller_data.running_network.contains_key(&NetNode::Drone(*id)) {
                    // Drone: circle
                    let circle = Circle {
                        x: cx,
                        y: cy,
                        radius: size,
                        color: CANVAS_DRONE_COLOR,
                        ..Default::default()
                    };
                    ctx.draw(&circle);
                } else if app.simulation_controller_data.running_network.contains_key(&NetNode::Client(*id)) {
                    // Client: square
                    let top = CanvasLine { x1: cx - size, y1: cy - size, x2: cx + size, y2: cy - size, color: CANVAS_CLIENT_COLOR, ..Default::default() };
                    let bottom = CanvasLine { x1: cx - size, y1: cy + size, x2: cx + size, y2: cy + size, color: CANVAS_CLIENT_COLOR, ..Default::default() };
                    let left = CanvasLine { x1: cx - size, y1: cy - size, x2: cx - size, y2: cy + size, color: CANVAS_CLIENT_COLOR, ..Default::default() };
                    let right = CanvasLine { x1: cx + size, y1: cy - size, x2: cx + size, y2: cy + size, color: CANVAS_CLIENT_COLOR, ..Default::default() };
                    ctx.draw(&top);
                    ctx.draw(&bottom);
                    ctx.draw(&left);
                    ctx.draw(&right);
                } else {
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
                // Draw node ID at center
                ctx.print(cx, cy, format!("{}", id));
            }
        });
    // Render the canvas into the allocated area
    canvas.render(map_area, buf);*/
}

impl From<&Node> for ListItem<'_> {
    fn from(n: &Node) -> Self {
        let line = match n {
            Node::Client(c) => Line::styled(
                format!("Client ID {}", c.id),
                Style::default().fg(CANVAS_CLIENT_COLOR),
            ).centered(),
            Node::Drone(d) => Line::styled(
                format!("Drone ID {}", d.id),
                Style::default().fg(CANVAS_DRONE_COLOR),
            ).centered(),
            Node::Server(s) => Line::styled(
                format!("Server ID {}", s.id),
                Style::default().fg(CANVAS_SERVER_COLOR),
            ).centered(),
        };
        ListItem::new(line)
    }
}