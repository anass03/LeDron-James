use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::prelude::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Widget};
use wg_2024::network::NodeId;
use crate::frontend::app::models::tab1_types::{ClientPopup, DronePopup, PopupSelection, ServerPopup};
use crate::frontend::app::ui::tabs::{MAIN_BLOCK_BORDERS, POPUP_BG_COLOR};
use std::cmp::PartialEq;

use ratatui::widgets::canvas::{Canvas, Circle, Line as CanvasLine, Map};
use std::ops::Add;
use ratatui::style::{Styled, Stylize};
use ratatui::text::ToSpan;
use crate::backend::sc_models::DroneType;
use crate::backend::simulation_controller::NetNode;
use crate::frontend::app::App;
use crate::frontend::app::models::general_types::Node;
use crate::frontend::app::ui::tabs::tab1::{get_data_canvas, get_settings_canvas};
use crate::frontend::app::ui::tabs::tab1::data_rendering::render_cl_sr_data_options;
use crate::frontend::app::ui::tabs::tab1::draw_map;

pub(in crate::frontend::app::ui::tabs) fn render_drone_popup(app:&App, area: Rect, buf : &mut Buffer, drone_popup: &DronePopup, id:NodeId) {

    let sim_popup_block = Block::default()
        .borders(Borders::ALL);
    let inner_sim_popup_block_area = sim_popup_block.inner(area);
    sim_popup_block.render(area, buf);
    let [title_area, other_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Percentage(99)])
        .areas(inner_sim_popup_block_area);

    // Draw centered title in title_area
    let title_para = Paragraph::new(format!("Drone {} - {}", id, app.simulation_controller_data.drone_types.get(&id).unwrap_or(&DroneType::Unknown)))
        .alignment(Alignment::Center)
        .style(Style::default().add_modifier(Modifier::RAPID_BLINK));
    title_para.render(title_area, buf);

    let [select_area, opt_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(15), Constraint::Percentage(85)])
        .margin(1)
        .areas(other_area);

    let [barchart_symb, settings_symb] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .areas(select_area);

    // Draw function graph icon (sine wave)
    let data_canvas = get_data_canvas(app, NetNode::Drone(0));
    data_canvas.render(barchart_symb, buf);

    // Draw settings icon
    let settings_canvas = get_settings_canvas(app, NetNode::Drone(0));
    settings_canvas.render(settings_symb, buf);

    let mut opt_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    match app.cache.tab1_data.popup_selection{
        PopupSelection::MenuArea => (),
        PopupSelection::OptArea => {
            opt_block = opt_block.border_style(Style::default().fg(app.animation_color()));
        }
    }
    let opt_inner = opt_block.inner(opt_area);
    opt_block.render(opt_area, buf);

    render_drone_options(app,opt_inner,buf,drone_popup,&id)
}

fn render_drone_options(app:&App, area:Rect, buf: &mut Buffer, drone_popup: &DronePopup, selected_node: &NodeId){
    match drone_popup{
        DronePopup::DataPage(datapage) => {
            render_cl_sr_data_options(app, area, buf, *selected_node)
        }
        DronePopup::SettingsPage(setpage) => {
            render_settings_page(app, area, buf, *selected_node)
        }
    }
}
fn render_settings_page(app:&App, area:Rect, buf: &mut Buffer, id:NodeId) {
    let [sett_area, view_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .spacing(1)
        .areas(area);

    let commands_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style( Style::default().fg(
            if app.cache.tab1_data.popup_cursor == 0 {
                app.animation_color()
            }else {
                MAIN_BLOCK_BORDERS
            }
        ))
        .title(" Commands ")
        .title_alignment(Alignment::Left);

    let inner_commands_block = commands_block
        .inner(sett_area);

    commands_block.render(sett_area, buf);


    // I have to render an option list

    let [text_area, map_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .spacing(1)
        .areas(view_area);

    let map_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style( Style::default().fg(
            if app.cache.tab1_data.popup_cursor == 1 {
                app.animation_color()
            }else {
                MAIN_BLOCK_BORDERS
            }
        ));

    for (node,_,top_alg) in &app.cache.tab1_data.netnode_list.items{
        match node{
            Node::Client(id_1) => {
                if id_1.id.eq(&id) {
                    draw_map(top_alg, map_area, buf, map_block.clone());
                    break;
                }
            }
            Node::Server(id_1) => {
                if id_1.id.eq(&id){
                    draw_map(top_alg,map_area,buf,map_block.clone());
                    break;
                }
            }
            Node::Drone(id_1) => {
                if id_1.id.eq(&id){
                    draw_map(top_alg,map_area,buf,map_block.clone());
                    break;
                }
            }
        }
    }


}