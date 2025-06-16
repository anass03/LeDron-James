use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use wg_2024::network::NodeId;
use crate::frontend::app::App;
use crate::frontend::app::models::tab1_types::{DronePopup, ServerPopup};

pub(in crate::frontend::app::ui::tabs) fn render_drone_popup(app:&App, area: Rect, buf : &mut Buffer, drone_popup: &DronePopup, id:NodeId) {

}