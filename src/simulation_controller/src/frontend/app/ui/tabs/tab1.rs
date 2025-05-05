use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Padding, Paragraph, Tabs, Widget, Wrap};
use crate::frontend::app::App;

/// Renders Topology tab-page
pub(super) fn render_tab1(app:&App, area: Rect, buf : &mut Buffer){
    // TAB 1 : Start a simulation to view the topology and modify it.
    // Starting phase with a gauge. Network Initializer sends updates to update
    // the value on the load bar.
    // TODO: Demo version with all the base functions needed to test the network


    // DEMO LAYOUT
    // LAYOUT ORIZONTALLY DIVIDED IN 2
    // BIG LOG PAGE IN THE LEFT, CONNECTION NAMES WRITTEN IN A SMALL BOX ON THE RIGHT WITH
    // SOME FUNCTIONS LIKE SEND PACKET TO, GET IMAGE, ETC. ON THE RIGHT


}