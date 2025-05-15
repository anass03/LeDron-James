use petgraph::visit::Walker;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, HighlightSpacing, List, ListItem, ListState, Padding, Paragraph, Tabs, Widget, Wrap};

use wg_2024::config::Drone as DroneConfig;
use wg_2024::config::Client as ClientConfig;
use wg_2024::config::Server as ServerConfig;
use super::super::super::model::{Tab1,Node};

use crate::frontend::app::App;

/// Renders Topology tab-page
pub(super) fn demo_render_tab1(app:&App, area: Rect, buf : &mut Buffer){
    // TAB 1 : Start a simulation to view the topology and modify it.
    // Starting phase with a gauge. Network Initializer sends updates to update
    // the value on the load bar.
    // TODO: Demo version with all the base functions needed to test the network

    let [left_column, right_column] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(60)])
        .areas(area);

    let [fns_area, log_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .areas(right_column);


    let [available_fns, result_fns] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .areas(fns_area);

    // Render Block list on Left Column
    let block_list = Block::default()
        .title(Span::from("Available Nodes"))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);

    // Render List on the block list
    let list_items = app
        .cache
        .tab1_data
        .nodelist
        .items
        .iter()
        .map(ListItem::from)        // Into<ListItem> because you impl From<&Node>
        .collect::<Vec<ListItem>>();

    let list = List::new(list_items)
        .block(block_list)
        .highlight_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .highlight_spacing(HighlightSpacing::Always)
        .highlight_symbol(">");

    list.render(left_column,buf)
    // Log area will be rendered by the same function on Overview Page ( Tab0 )

    // Retrieve

}
impl From<&Node> for ListItem<'_>{
    fn from(n: &Node) -> Self {
        let line = match n{
            Node::Client(c) => {
                Line::styled(format!("Client ID: {}", c.id), Style::default().fg(Color::LightCyan))
            },
            Node::Drone(d) => {
                Line::styled(format!("Drone ID: {}", d.id), Style::default().fg(Color::LightBlue))
            }
            Node::Server(s) => {
                Line::styled(format!("Server ID: {}", s.id), Style::default().fg(Color::LightRed))
            }
        };
        ListItem::new(line)
    }
}
// Render functions depending on the item currently selected
fn render_selected_fns(){
}

// Render output depending on the item currently selected
// In case of a client showing: Chat
fn render_selected_out(){

}