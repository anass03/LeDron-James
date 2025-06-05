use petgraph::visit::Walker;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, ToLine};
use ratatui::widgets::{
    Block, BorderType, Borders, HighlightSpacing, List, ListItem, ListState, Padding, Paragraph,
    Tabs, Widget, Wrap,
};
use ratatui_image::picker::Picker;

use super::super::super::model::{Node, Tab1};
use wg_2024::config::Client as ClientConfig;
use wg_2024::config::Drone as DroneConfig;
use wg_2024::config::Server as ServerConfig;

use crate::frontend::app::App;

/// Renders Topology tab-page
pub(super) fn demo_render_tab1(app: &App, area: Rect, buf: &mut Buffer) {
    // TAB 1 : Start a simulation to view the topology and modify it.
    // Starting phase with a gauge. Network Initializer sends updates to update
    // the value on the load bar.
    // TODO: Demo version with all the base functions needed to test the network

    let [left_column, right_column] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
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
    let mut block_list = Block::default()
        .title(Span::from("Available Nodes"))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);

    if app.cache.cursor == 0 {
        block_list = block_list.border_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    }

    // Render List on the block list
    let list_items = app
        .cache
        .tab1_data
        .nodelist
        .items
        .iter()
        .map(|(n, b)| ListItem::from(n)) // Into<ListItem> because you impl From<&Node>
        .collect::<Vec<ListItem>>();

    let list = List::new(list_items)
        .block(block_list)
        .highlight_style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_spacing(HighlightSpacing::Always)
        .highlight_symbol(">");

    ratatui::widgets::StatefulWidget::render(
        list,
        left_column,
        buf,
        &mut app.cache.tab1_data.nodelist.state.clone(),
    );

    render_selected_fns(app, available_fns, buf);

    // Log area will be rendered by the same function on Overview Page ( Tab0 )
    let log_paragraph = super::tab0::process_log_paragraph(app);
    log_paragraph.render(log_area, buf);

    render_selected_out(app, result_fns, buf);

    // Retrieve
}

impl From<&Node> for ListItem<'_> {
    fn from(n: &Node) -> Self {
        let line = match n {
            Node::Client(c) => Line::styled(
                format!("Client ID: {}", c.id),
                Style::default().fg(Color::LightCyan),
            ),
            Node::Drone(d) => Line::styled(
                format!("Drone ID: {}", d.id),
                Style::default().fg(Color::LightBlue),
            ),
            Node::Server(s) => Line::styled(
                format!("Server ID: {}", s.id),
                Style::default().fg(Color::LightRed),
            ),
        };
        ListItem::new(line)
    }
}
// Render functions depending on the item currently selected
fn render_selected_fns(app: &App, area: Rect, buf: &mut Buffer) {
    use super::super::super::super::app::model;
    let selected_n = app.cache.tab1_data.nodelist.selected;
    match app.cache.tab1_data.nodelist.items.get(selected_n as usize) {
        Some((n, b)) => {
            // Function Process
            let mut vec_and_block = match n {
                Node::Client(_) => {
                    // Render Paragraph Client
                    render_client_opt(b)
                }
                Node::Server(_) => {
                    // Render Paragraph Server
                    render_server_opt(b)
                }
                Node::Drone(_) => {
                    // Render Drone
                    render_drone_opt(b)
                }
            };
            if app.cache.cursor==1{
                vec_and_block.1 = vec_and_block.1.border_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
            }
            let list = List::new(vec_and_block.0)
                .block(vec_and_block.1)
                .highlight_style(
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_spacing(HighlightSpacing::Always)
                .highlight_symbol(">");
            ratatui::widgets::StatefulWidget::render(
                list,
                area,
                buf,
                &mut app.cache.tab1_data.selected_fn.state.clone(),
            );
        }
        None => {
            // Error LOG Block processing
            let paragraph = Paragraph::new("finirà mai sto progetto di merda?")
                .block(
                    Block::default()
                        .title(Span::from(" NO ITEM AVAILABLE / SELECTED "))
                        .borders(Borders::ALL),
                )
                .style(Style::default().fg(Color::Red))
                .centered();
            paragraph.render(area, buf);
        }
    }
}
fn render_client_opt<'a>(bl: &bool) -> (Vec<Line<'a>>, Block<'a>) {
    let mut block_list = Block::default()
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);
    let mut list_items: Vec<&str> = vec![];
    if *bl {
        block_list = block_list.title(Span::from("Choose your Client option"));
        list_items = vec![
            "[1] Server Type",
            "[2] FilesList 1 input",
            "[3] File 2 input",
            "[4] MediaList 1 input",
            "[5] Media 2 input",
            "[6] Registration to chat 1 input",
            "[7] ClientList 1 input",
            "[8] Message For 3 input",
        ];
    } else {
        block_list = block_list.title(Span::from("Client deactivated"));
    }
    (
        list_items.iter().map(|&a| Line::from(a)).collect(),
        block_list,
    )
}
fn render_server_opt<'a>(bl: &bool) -> (Vec<Line<'a>>, Block<'a>) {
    let mut block_list = Block::default()
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);
    let mut list_items: Vec<&str> = vec![];
    if *bl {
        block_list = block_list.title(Span::from("Choose your Server option"));
        list_items = vec!["[1] Send Nudes to Patrignani"];
    } else {
        block_list = block_list.title(Span::from("Server deactivated"));
    }
    (
        list_items.iter().map(|&a| Line::from(a)).collect(),
        block_list,
    )
}
fn render_drone_opt<'a>(bl: &bool) -> (Vec<Line<'a>>, Block<'a>) {
    let mut block_list = Block::default()
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);
    let mut list_items: Vec<&str> = vec![];
    if *bl {
        block_list = block_list.title(Span::from("Choose your Drone option"));
        list_items = vec!["[1] Remove Sender", "[2] Add Sender", "[3] Set PDR", "[4] Crash"];
    } else {
        block_list = block_list.title(Span::from("Drone deactivated"));
    }
    (
        list_items.iter().map(|&a| Line::from(a)).collect(),
        block_list,
    )
}
// Render output depending on the item currently selected
// In case of a client showing: Chat
fn render_selected_out(app:&App, area:Rect, buf: &mut Buffer) {
    let selected_n = app.cache.tab1_data.nodelist.selected;
    match app.cache.tab1_data.nodelist.items.get(selected_n as usize) {
        None => {
            let paragraph = Paragraph::new("finirà mai sto progetto di merda?")
                .block(
                    Block::default()
                        .title(Span::from(" NO ITEM AVAILABLE / SELECTED "))
                        .borders(Borders::ALL),
                )
                .style(Style::default().fg(Color::Red))
                .centered();
            paragraph.render(area, buf);
        }
        Some((n, b)) => {
            match n {
                Node::Client(_) => {
                    match app.cache.tab1_data.selected_fn.selected {
           /*             "[1] Server Type",
                        "[2] FilesList 1 input",
                        "[3] File 2 input",
                        "[4] MediaList 1 input",
                        "[5] Media 2 input",
                        "[6] Registration to chat 1 input",
                        "[7] ClientList 1 input",
                        "[8] Message For 3 input",*/
                        0 => {
                            render_no_input_fn(app, area, buf);
                        }
                        1 | 3 | 5 | 6 => {
                            render_one_input_fn(app, area, buf);
                        }
                        2 | 4 => {
                            render_two_input_fn(app, area, buf);
                        }
                        7 => {
                            render_three_input_fn(app, area, buf);
                        }
                        _ => (),
                    }
                }
                Node::Server(_) => {
               /*     let block_image = Block::default()
                        .title(" PAT PAT PAT ")
                        .title_alignment(Alignment::Center)
                        .borders(Borders::ALL);
                    let inner_block = block_image.inner(area);

                    let dyn_img = image::ImageReader::open("./assets/IMG_0400.jpg")
                        .unwrap()
                        .decode()
                        .unwrap();*/
                    /*a
                    // dInitialize a Picker for Crossterm graphics
                    let font_size = (8, 16); // adjust based on your terminal cell size if needed
                    let mut picker = Picker::new(font_size, BackendType::Crossterm, None).unwrap();

                    // Create a fixed backend that fits the image into the log area
                    let fb = picker
                        .new_static_fit(dyn_img, inner_block, Resize::Fit)
                        .unwrap();

                    // Wrap it in the FixedImage widget and render
                    let image_widget = FixedImage::new(fb).block(block_image);
                    image_widget.render(area, buf);

                     */
                }
                Node::Drone(_) => {
                    // Drone commands
                    /*"[1] Remove Sender 1 INPUT",
                     "[2] Add Sender 1 input",
                     "[3] Set PDR 1 input",
                     "[4] Crash 0 input"*/
                    match app.cache.tab1_data.selected_fn.selected {
                        3 => {
                            render_no_input_fn(app, area, buf);
                        }
                        0 | 1 | 2 => {
                            render_one_input_fn(app, area, buf);
                        }
                        _ => (),
                    }

                }
            }
        }
    }
}
fn render_no_input_fn(app:&App, area:Rect, buf:&mut Buffer){
    let [top_area, _] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
        .areas(area);
    let mut enter_block = Block::new()
        .title(Span::from(" PRESS ENTER TO CONFIRM "))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);
    if app.cache.tab1_data.selected_button == 0 && app.cache.cursor == 2 {
        enter_block = enter_block.border_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    }
    enter_block.render(top_area, buf);
}
fn render_one_input_fn(app:&App, area:Rect, buf:&mut Buffer){
    let [top_area, input_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
        .areas(area);
    let [button_area, _] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .areas(top_area);

    let mut enter_block = Block::new()
        .title(Span::from(" PRESS ENTER TO CONFIRM "))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);
    if app.cache.tab1_data.selected_button == 0 && app.cache.cursor == 2  {
        enter_block = enter_block.border_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    }
    enter_block.render(button_area, buf);


    let mut first_block = Block::new()
        .title(Span::from(" insert the ID ( u8 ) "))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);
    if app.cache.tab1_data.selected_button == 1 {
        first_block = first_block.border_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    }
    let inner_first_block = first_block.inner(input_area);

    first_block.render(input_area, buf);
    app.cache.text_buffer.inputfirst_text_area.render(inner_first_block, buf);

}
fn render_two_input_fn(app:&App, area:Rect, buf:&mut Buffer){
    let [top_area, input_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
        .areas(area);

    let [first_input_area, second_input_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .areas(input_area);

    let [button_area, _] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .areas(top_area);

    let mut enter_block = Block::new()
        .title(Span::from(" PRESS ENTER TO CONFIRM "))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);
    if app.cache.tab1_data.selected_button == 0 && app.cache.cursor == 2 {
        enter_block = enter_block.border_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    }
    enter_block.render(button_area, buf);

    // I'll render 2 TUI_TEXT_AREA on the two areas, with all the 2 different buffers
    // 1 - Add 2 buffers on tab1_data
    // 2 - Add a fn to flush each data each tab change,
    // that parse each buffer!

    let mut first_block = Block::new()
        .title(Span::from(" insert the ID ( u8 ) "))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);
    if app.cache.tab1_data.selected_button == 1 {
        first_block = first_block.border_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    }
    let inner_first_block = first_block.inner(first_input_area);
    first_block.render(first_input_area, buf);
    app.cache.text_buffer.inputfirst_text_area.render(inner_first_block, buf);

    let mut second_block = Block::new()
        .title(Span::from(" insert the server ID ( u8 ) "))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);
    if app.cache.tab1_data.selected_button == 2 {
        second_block = second_block.border_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    }
    let inner_second_block = second_block.inner(second_input_area);
    second_block.render(second_input_area, buf);
    app.cache.text_buffer.inputsecond_text_area.render(inner_second_block, buf);

}

fn render_three_input_fn(app:&App, area:Rect, buf:&mut Buffer){
    let [top_area, input_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
        .areas(area);

    let [first_input_area, second_input_area,third_input_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(33), Constraint::Percentage(34), Constraint::Percentage(33)])
        .areas(input_area);

    let [button_area, _] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .areas(top_area);

    let mut enter_block = Block::new()
        .title(Span::from(" PRESS ENTER TO CONFIRM "))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);
    if app.cache.tab1_data.selected_button == 0 && app.cache.cursor == 2 {
        enter_block = enter_block.border_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    }
    enter_block.render(button_area, buf);

    let mut first_block = Block::new()
        .title(Span::from(" insert the ID ( u8 ) "))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);
    if app.cache.tab1_data.selected_button == 1 {
        first_block = first_block.border_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    }
    let inner_first_block = first_block.inner(first_input_area);
    first_block.render(first_input_area, buf);
    app.cache.text_buffer.inputfirst_text_area.render(inner_first_block, buf);


    let mut second_block = Block::new()
        .title(Span::from(" insert the server ID ( u8 ) "))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);
    if app.cache.tab1_data.selected_button == 2 {
        second_block = second_block.border_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    }
    let inner_second_block = second_block.inner(second_input_area);
    second_block.render(second_input_area, buf);
    app.cache.text_buffer.inputsecond_text_area.render(inner_second_block, buf);


    let mut third_block = Block::new()
        .title(Span::from(" insert the message "))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);
    if app.cache.tab1_data.selected_button == 3 {
        third_block = third_block.border_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    }
    let inner_third_block = third_block.inner(third_input_area);
    third_block.render(third_input_area, buf);
    app.cache.text_buffer.inputthird_text_area.render(inner_third_block, buf);

}
