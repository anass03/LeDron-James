use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use wg_2024::network::NodeId;
use crate::frontend::app::App;
use crate::frontend::app::models::tab1_types::{ClientPopup, ServerPopup};

pub(in crate::frontend::app::ui::tabs) fn render_server_popup(app:&App, area: Rect, buf : &mut Buffer, server_popup: &ServerPopup, id:NodeId) {

}