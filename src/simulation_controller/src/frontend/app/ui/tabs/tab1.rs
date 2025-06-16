use std::default::Default;
use ratatui::layout::Rect;
use ratatui::buffer::Buffer;
use ratatui::layout::Alignment::Center;
use crate::frontend::app::App;
use crate::frontend::app::models::tab1_types::{NodePopup, ViewedTab1};
use crate::frontend::app::ui::tabs::{tab1_client_popup, tab1_drone_popup, tab1_main_page, tab1_server_popup, POPUP_BG_COLOR};
// popup code

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Style, Stylize};
use ratatui::widgets;
use ratatui::widgets::{Block, Widget};
use tui_popup::Popup;

fn centered_popup_area(area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(12),
            Constraint::Percentage(75),
            Constraint::Percentage(13),
        ])
        .split(area);

    let vertical_center = popup_layout[1];

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(12),
            Constraint::Percentage(75),
            Constraint::Percentage(13),
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