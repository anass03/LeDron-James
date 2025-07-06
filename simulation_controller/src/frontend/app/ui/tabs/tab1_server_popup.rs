use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::prelude::{Color, Line, Modifier, Style, Widget};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph};
use wg_2024::network::NodeId;
use crate::backend::sc_models::{DroneType, NetNode};
use crate::frontend::app::App;
use crate::frontend::app::models::general_types::Node;
use crate::frontend::app::models::tab1_types::{ClientPopup, DronePopup, DroneSettingsPopup, PopupSelection, ServerPopup, ServerSettingsCommandsPopup, ServerSettingsPopup};
use crate::frontend::app::ui::tabs::MAIN_BLOCK_BORDERS;
use crate::frontend::app::ui::tabs::tab1::{draw_map, get_data_canvas, get_settings_canvas, render_add_sender_popup, render_remove_sender_popup};
use crate::frontend::app::ui::tabs::tab1::data_rendering::render_cl_sr_data_options;

pub(in crate::frontend::app::ui::tabs) fn render_server_popup(app:&App, area: Rect, buf : &mut Buffer, server_popup: &ServerPopup, id:NodeId) {

    let sim_popup_block = Block::default()
        .borders(Borders::ALL);
    let inner_sim_popup_block_area = sim_popup_block.inner(area);
    sim_popup_block.render(area, buf);
    let [title_area, other_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Percentage(99)])
        .areas(inner_sim_popup_block_area);

    // Draw centered title in title_area
    let title_para = Paragraph::new(format!("Server {}", id))
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

    render_server_options(app, opt_inner, buf, server_popup, &id)
}

fn render_server_options(app:&App, area:Rect, buf: &mut Buffer, server_popup: &ServerPopup, selected_node: &NodeId){
    match server_popup{
        ServerPopup::DataPage(datapage) => {
            render_cl_sr_data_options(app, area, buf, *selected_node,0)
        }
        ServerPopup::SettingsPage(setpage) => {
            render_settings_page(app, area, buf, *selected_node, setpage)
        }
    }
}
fn render_settings_page(app:&App, area:Rect, buf: &mut Buffer, id:NodeId, set_page:&ServerSettingsPopup) {
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
        "Add Sender",
        "Remove Sender",
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
        "Add Sender",
        "Remove Sender",
        "Recompute Neighbours Map",
    ];
    let explanations = [
        "Add a new sender node to this drone.\nExpands the network topology.",
        "Remove a sender node from this drone.\nPrune connections or simulate failures.",
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

    render_command_popup(app, area, buf, &set_page.command_popup);
}
fn render_command_popup(app: &App, area: Rect, buf: &mut Buffer, popup: &ServerSettingsCommandsPopup) {
    match popup {
        ServerSettingsCommandsPopup::AddSender(cmd) => render_add_sender_popup(app, area, buf, cmd),
        ServerSettingsCommandsPopup::RemoveSender(cmd) => render_remove_sender_popup(app, area, buf, cmd),
        ServerSettingsCommandsPopup::None => {},
    }
}

