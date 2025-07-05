use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::prelude::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Widget, Wrap};
use wg_2024::network::NodeId;
use crate::frontend::app::models::tab1_types::{DronePopup, DroneSettingsPopup, PopupSelection};
use crate::frontend::app::ui::tabs::{tab1, MAIN_BLOCK_BORDERS};
use std::cmp::PartialEq;

use ratatui::style::{Styled, Stylize};
use ratatui::text::Line;
use crate::backend::sc_models::DroneType;
use crate::backend::sc_models::NetNode;
use crate::frontend::app::App;
use crate::frontend::app::models::general_types::Node;
use crate::frontend::app::ui::tabs::tab1::{get_data_canvas, get_settings_canvas, render_add_sender_popup, render_remove_sender_popup};
use crate::frontend::app::ui::tabs::tab1::data_rendering::render_cl_sr_data_options;
use crate::frontend::app::ui::tabs::tab1::draw_map;

use ratatui::widgets::{List, ListItem};
use ratatui::widgets;

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
            render_settings_page(app, area, buf, *selected_node, setpage)
        }
    }
}
fn render_settings_page(app:&App, area:Rect, buf: &mut Buffer, id:NodeId, set_page:&DroneSettingsPopup) {
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

    // Command selection list (styled like other lists)
    let commands = [
        "Set PDR",
        "Add Sender",
        "Remove Sender",
        "Crash",
        "Recompute Neighbours Map",
    ];
    let items: Vec<ListItem> = commands.iter().enumerate().map(|(idx, cmd)| {
        let base_style = if idx % 2 == 0 {
            Style::default().bg(crate::frontend::app::ui::tabs::BG_COLOR)
        } else {
            Style::default().bg(crate::frontend::app::ui::tabs::ALTERNATE_LIST_COLOR)
        };
        ListItem::new(cmd.to_string()).style(base_style)
    }).collect();
    let list = List::new(items)
        .block(Block::default().borders(Borders::NONE))
        .highlight_style(
            Style::default()
                .bg(crate::frontend::app::ui::tabs::SELECTED_LIST_COLOR)
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ")
        .highlight_spacing(ratatui::widgets::HighlightSpacing::Always);
    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(set_page.command_offset as usize));
    ratatui::widgets::StatefulWidget::render(
        list,
        inner_commands_block,
        buf,
        &mut state,
    );

    let [text_area, map_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .spacing(1)
        .areas(view_area);

    let map_block = Block::default()
        .title("Neighbours Map")
        .title_alignment(Alignment::Right)
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

    // Animated LeDron James character
    let le_dron_frames = ["< 🤖 >", "<  🤖  >", "<   🤖   >", "<    🤖    >", "<   🤖   >", "<  🤖  >", "< 🤖 >"];
    let le_dron = le_dron_frames[(app.cache.counter as usize) % le_dron_frames.len()];
    let le_dron_colored = Line::from(le_dron).style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD));

    let command_names = [
        "Set PDR",
        "Add Sender",
        "Remove Sender",
        "Crash",
        "Recompute Neighbours Map",
    ];
    let explanations = [
        "Change the Packet Drop Rate for this drone.\nUseful for simulating unreliable links.",
        "Add a new sender node to this drone.\nExpands the network topology.",
        "Remove a sender node from this drone.\nPrune connections or simulate failures.",
        "Simulate a drone crash.\nRemoves the drone from the network.",
        "Recalculate the network map for this drone.\nUseful after topology changes.",
    ];
    let selected = set_page.command_offset as usize;
    let command_line = Line::from(command_names[selected])
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    let desc_line = Line::from(explanations[selected])
        .style(Style::default().fg(Color::Cyan));

    let lines = vec![
        le_dron_colored,
        Line::from("LeDron James says:").style(Style::default().fg(Color::Magenta)),
        Line::from(""),
        command_line,
        desc_line,
    ];
    let para = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    para.render(text_area, buf);

    render_command_popup(app,area,buf,&set_page.command_popup,set_page.drone_pdr);
}

// Main dispatcher for command popups
fn render_command_popup(app: &App, area: Rect, buf: &mut Buffer, popup: &crate::frontend::app::models::tab1_types::DroneSettingsCommandsPopup, drone_pdr: f32) {
    match popup {
        crate::frontend::app::models::tab1_types::DroneSettingsCommandsPopup::SetPDR(cmd) => render_set_pdr_popup(app, area, buf, cmd, drone_pdr),
        crate::frontend::app::models::tab1_types::DroneSettingsCommandsPopup::AddSender(cmd) => render_add_sender_popup(app, area, buf, cmd),
        crate::frontend::app::models::tab1_types::DroneSettingsCommandsPopup::RemoveSender(cmd) => render_remove_sender_popup(app, area, buf, cmd),
        crate::frontend::app::models::tab1_types::DroneSettingsCommandsPopup::Crash(cmd) => render_crash_popup(app, area, buf, cmd),
        crate::frontend::app::models::tab1_types::DroneSettingsCommandsPopup::None => {},
    }
}

fn render_set_pdr_popup(app: &App, area: Rect, buf: &mut Buffer, cmd: &crate::frontend::app::models::tab1_types::CommandPopup, old_pdr: f32) {
    let popup_area = tab1::small_centered_popup_area(area);
    widgets::Clear.render(popup_area, buf);
    let block = Block::default()
        .title("Set PDR")
        .title_alignment(Alignment::Center)
        .title_bottom(Line::from(format!("Old PDR: {}",old_pdr)).alignment(Alignment::Center))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(tab1::popup_border_color(&cmd.status)));
    let inner = block.inner(popup_area);
    block.render(popup_area, buf);
    let [fields_area, button_area, status_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .areas(inner);

    // New PDR input
    let mut new_pdr_block = Block::default()
        .title("New PDR (f32)")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(
            if cmd.cursor == 0{
                app.animation_color()
            }else{
                MAIN_BLOCK_BORDERS
            }
        ));
    let inner_new_pdr = new_pdr_block.inner(fields_area);
    new_pdr_block.render(fields_area, buf);

    app.cache.text_buffer.inputfirst_text_area.render(inner_new_pdr, buf);
    // Send button
    let send_button = Paragraph::new("[ Send ]")
        .alignment(Alignment::Center)
        .style(Style::default().fg(
            if cmd.cursor == 1{
                app.animation_color()
            }else{
                MAIN_BLOCK_BORDERS
            }
        ).add_modifier(Modifier::BOLD));
    send_button.render(button_area, buf);
    // Status
    match &cmd.status {
        crate::frontend::app::models::tab1_types::SettingsPopupStatus::CorrectlySent => {
            let msg = Paragraph::new("Correctly sent!")
                .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center);
            msg.render(status_area, buf);
        },
        crate::frontend::app::models::tab1_types::SettingsPopupStatus::Error(e) => {
            // Wrap long error messages to fit in multiple rows
            let error_text = format!("Error: {}", e);
            let wrapped_lines = textwrap::wrap(&error_text, status_area.width as usize);
            let lines: Vec<Line> = wrapped_lines.iter()
                .map(|line| Line::from(line.to_string())
                    .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)))
                .collect();
            let msg = Paragraph::new(lines)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            msg.render(status_area, buf);
        },
        _ => {},
    }
}

/*fn render_add_sender_popup(app: &App, area: Rect, buf: &mut Buffer, cmd: &crate::frontend::app::models::tab1_types::CommandPopup) {
    let popup_area = small_centered_popup_area(area);
    widgets::Clear.render(popup_area, buf);
    let block = Block::default()
        .title("Add Sender")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(popup_border_color(&cmd.status)));
    let inner = block.inner(popup_area);
    block.render(popup_area, buf);
    let [input_area, button_area, status_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .areas(inner);
    let mut nodeid_block = Block::default()
        .title("Node ID (u8)")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(
            if cmd.cursor == 0{
            app.animation_color()
        }else{
            MAIN_BLOCK_BORDERS
        }));
    let inner_nodeid = nodeid_block.inner(input_area);
    nodeid_block.render(input_area, buf);
    app.cache.text_buffer.inputfirst_text_area.render(inner_nodeid, buf);
    let send_button = Paragraph::new("[ Send ]")
        .alignment(Alignment::Center)
        .style(Style::default().fg(
            if cmd.cursor == 1{
                app.animation_color()
            }else{
                MAIN_BLOCK_BORDERS
            }
        ).add_modifier(Modifier::BOLD));
    send_button.render(button_area, buf);
    match &cmd.status {
        crate::frontend::app::models::tab1_types::SettingsPopupStatus::CorrectlySent => {
            let msg = Paragraph::new("Correctly sent!")
                .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center);
            msg.render(status_area, buf);
        },
        crate::frontend::app::models::tab1_types::SettingsPopupStatus::Error(e) => {
            let msg = Paragraph::new(format!("Error: {}", e))
                .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center);
            msg.render(status_area, buf);
        },
        _ => {},
    }
}

fn render_remove_sender_popup(app: &App, area: Rect, buf: &mut Buffer, cmd: &crate::frontend::app::models::tab1_types::CommandPopup) {
    let popup_area = small_centered_popup_area(area);
    widgets::Clear.render(popup_area, buf);
    let block = Block::default()
        .title("Remove Sender")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(popup_border_color(&cmd.status)));
    let inner = block.inner(popup_area);
    block.render(popup_area, buf);
    let [input_area, button_area, status_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .areas(inner);
    let mut nodeid_block = Block::default()
        .title("Node ID (u8)")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(
            if cmd.cursor == 0{
                app.animation_color()
            }else{
                MAIN_BLOCK_BORDERS
            }
        ));
    let inner_nodeid = nodeid_block.inner(input_area);
    nodeid_block.render(input_area, buf);
    app.cache.text_buffer.inputfirst_text_area.render(inner_nodeid, buf);
    let send_button = Paragraph::new("[ Send ]")
        .alignment(Alignment::Center)
        .style(Style::default().fg(
            if cmd.cursor == 1{
                app.animation_color()
            }else{
                MAIN_BLOCK_BORDERS
            }
        ).add_modifier(Modifier::BOLD));
    send_button.render(button_area, buf);
    match &cmd.status {
        crate::frontend::app::models::tab1_types::SettingsPopupStatus::CorrectlySent => {
            let msg = Paragraph::new("Correctly sent!")
                .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center);
            msg.render(status_area, buf);
        },
        crate::frontend::app::models::tab1_types::SettingsPopupStatus::Error(e) => {
            let msg = Paragraph::new(format!("Error: {}", e))
                .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center);
            msg.render(status_area, buf);
        },
        _ => {},
    }
}
*/
fn render_crash_popup(app: &App, area: Rect, buf: &mut Buffer, cmd: &crate::frontend::app::models::tab1_types::CommandPopup) {
    let popup_area = tab1::small_centered_popup_area(area);
    widgets::Clear.render(popup_area, buf);
    let block = Block::default()
        .title("Crash Drone")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(tab1::popup_border_color(&cmd.status)));
    let inner = block.inner(popup_area);
    block.render(popup_area, buf);
    let [button_area, status_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .areas(inner);
    let send_button = Paragraph::new("[ Crash Drone ]")
        .alignment(Alignment::Center)
        .style(Style::default().fg(
            if cmd.cursor == 0{
                app.animation_color()
            }else{
                MAIN_BLOCK_BORDERS
            }
        ).add_modifier(Modifier::BOLD));
    send_button.render(button_area, buf);
    match &cmd.status {
        crate::frontend::app::models::tab1_types::SettingsPopupStatus::CorrectlySent => {
            let msg = Paragraph::new("Correctly sent!")
                .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center);
            msg.render(status_area, buf);
        },
        crate::frontend::app::models::tab1_types::SettingsPopupStatus::Error(e) => {
            // Wrap long error messages to fit in multiple rows
            let error_text = format!("Error: {}", e);
            let wrapped_lines = textwrap::wrap(&error_text, status_area.width as usize);
            let lines: Vec<Line> = wrapped_lines.iter()
                .map(|line| Line::from(line.to_string())
                    .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)))
                .collect();
            let msg = Paragraph::new(lines)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            msg.render(status_area, buf);
        },
        _ => {},
    }
}

