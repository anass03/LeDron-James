use std::default::Default;
use ratatui::layout::Rect;
use ratatui::buffer::Buffer;
use ratatui::layout::Alignment::Center;
use crate::frontend::app::App;
use crate::frontend::app::models::tab1_types::{NodePopup, PopupSelection, ViewedTab1};
use crate::frontend::app::ui::tabs::{tab1_client_popup, tab1_drone_popup, tab1_main_page, tab1_server_popup, POPUP_BG_COLOR};
// popup code

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style, Stylize};
use ratatui::widgets;
use ratatui::widgets::{Block, BorderType, Borders, Widget};
use ratatui::widgets::canvas::{Canvas, Circle, Context, Line as CanvasLine,};
use tui_popup::Popup;
use crate::backend::simulation_controller::NetNode;

fn centered_popup_area(area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(6),
            Constraint::Percentage(88),
            Constraint::Percentage(6),
        ])
        .split(area);

    let vertical_center = popup_layout[1];

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(6),
            Constraint::Percentage(88),
            Constraint::Percentage(6),
        ])
        .split(vertical_center);

    horizontal[1]
}


/// Renders Settings tab-page
pub(in crate::frontend::app::ui::tabs) fn render_tab1(app:&App, area: Rect, buf : &mut Buffer){
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
                    tab1_client_popup::render_client_popup(app, colored_popup_area, buf, cl, *id)
                }
                NodePopup::Server(sr) => {
                    tab1_server_popup::render_server_popup(app, colored_popup_area, buf, sr, *id)
                }
                NodePopup::Drone(dr) => {
                    tab1_drone_popup::render_drone_popup(app, colored_popup_area, buf, dr, *id)
                }
            }
        }
    }
}

// Utils needed by the single tabs

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

