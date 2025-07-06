use std::cmp::PartialEq;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect, Size};
use ratatui::buffer::Buffer;
use wg_2024::network::NodeId;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Widget, Wrap};
use ratatui::widgets::canvas::{Canvas, Circle, Context, Line as CanvasLine, Map};
use std::ops::Add;
use ratatui::style::{Color, Modifier, Style, Styled, Stylize};
use ratatui::text::{Line, Text, ToSpan};
use crate::backend::sc_models::NetNode;
use crate::frontend::app::App;
use crate::frontend::app::models::tab1_types::{ClientPopup, PopupSelection};
use crate::frontend::app::ui::tabs::POPUP_BG_COLOR;
use crate::frontend::app::ui::tabs::tab1::{get_data_canvas, get_settings_canvas};
use crate::frontend::app::ui::tabs::tab1_client_popup::chat_rendering::render_chat_options;
use crate::frontend::app::ui::tabs::tab1::data_rendering::render_cl_sr_data_options;
use crate::frontend::app::ui::tabs::tab1_client_popup::net_rendering::render_net_options;
use crate::frontend::app::ui::tabs::tab1_client_popup::settings_rendering::render_settings_page;
use tui_scrollview::{ScrollView, ScrollViewState};

pub(in crate::frontend::app::ui::tabs) fn render_client_popup(app:&App, area: Rect, buf : &mut Buffer, client_popup: &ClientPopup, id:NodeId) ->Option<Rect>{
    /*    let [area] = Layout::default()
            .constraints([Constraint::Fill(1)])
            .margin(3)
            .areas(area);*/
    let sim_popup_block = Block::default()
        .borders(Borders::ALL);
    let inner_sim_popup_block_area = sim_popup_block.inner(area);
    sim_popup_block.render(area, buf);
    let [title_area, other_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Percentage(99)])
        .areas(inner_sim_popup_block_area);

    // Draw centered title in title_area
    let title_para = Paragraph::new(format!("Client {}", id))
        .alignment(Alignment::Center)
        .style(Style::default().add_modifier(Modifier::RAPID_BLINK));
    title_para.render(title_area, buf);

    let [select_area, opt_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(15), Constraint::Percentage(85)])
        .margin(1)
        .areas(other_area);

    /*    let select_block = Block::default()
            .borders(Borders::ALL);
        let inner_select_block_area = select_block.inner(select_area);
        select_block.render(select_area, buf);*/

    let [chat_symb, server_symb, barchart_symb, settings_symb] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(25), Constraint::Percentage(25), Constraint::Percentage(25)])
        .areas(select_area);

    // Draw improved chat bubble icon
    let chat_canvas = Canvas::default()
        .background_color(POPUP_BG_COLOR)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if app.cache.tab1_data.popup_cursor == 0 && app.cache.tab1_data.popup_selection == PopupSelection::MenuArea {
                Style::default().fg(app.animation_color())
            }else{
                Style::default()
            })
            .title("Chat"))
        .x_bounds([0.0, 20.0])
        .y_bounds([0.0, 20.0])
        .paint(|ctx| {
            // Main bubble outline
            ctx.draw(&CanvasLine { x1: 2.0,  y1: 15.0, x2: 18.0, y2: 15.0, color: Color::White, ..Default::default() });
            ctx.draw(&CanvasLine { x1: 2.0,  y1: 5.0,  x2: 18.0, y2: 5.0,  color: Color::White, ..Default::default() });
            ctx.draw(&CanvasLine { x1: 2.0,  y1: 5.0,  x2: 2.0,  y2: 15.0, color: Color::White, ..Default::default() });
            ctx.draw(&CanvasLine { x1: 18.0, y1: 5.0,  x2: 18.0, y2: 15.0, color: Color::White, ..Default::default() });
            // Tail
            ctx.draw(&CanvasLine { x1: 6.0,  y1: 5.0,  x2: 8.0,  y2: 1.0,  color: Color::White, ..Default::default() });
            ctx.draw(&CanvasLine { x1: 8.0,  y1: 1.0,  x2: 10.0, y2: 5.0,  color: Color::White, ..Default::default() });
        });
    chat_canvas.render(chat_symb, buf);

    // Draw world map in Internet canvas
    let internet_canvas = Canvas::default()
        .background_color(POPUP_BG_COLOR)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if app.cache.tab1_data.popup_cursor == 1 && app.cache.tab1_data.popup_selection == PopupSelection::MenuArea {
                Style::default().fg(app.animation_color())
            }else{
                Style::default()
            })
            .title("Internet"))
        .x_bounds([-180.0, 180.0])
        .y_bounds([-90.0, 90.0])
        .paint(|ctx| {
            ctx.draw(
                { &Map::default() })
        });
    internet_canvas.render(server_symb, buf);

    // Draw function graph icon (sine wave)
    let data_canvas = get_data_canvas(app, NetNode::Client(0));
    data_canvas.render(barchart_symb, buf);

    // Draw settings icon
    let settings_canvas = get_settings_canvas(app, NetNode::Client(0));
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

    // Now I render the selected option in opt_area
    render_client_options(app, opt_inner, buf, client_popup,&id)
}

fn render_client_options(app:&App, area:Rect, buf: &mut Buffer, client_popup: &ClientPopup, selected_node: &NodeId)->Option<Rect>{
    // Now here based on the information we have in the struct we draw the menu's
    match client_popup{
        ClientPopup::ChatPage(chat_page_data) => {
            render_chat_options(app,area, buf, chat_page_data, selected_node);
        }
        ClientPopup::InternetPage(net_page_data) => {
            return render_net_options(app, area, buf, net_page_data, selected_node)
        }
        ClientPopup::DataPage(data_page_data) => {
            render_cl_sr_data_options(app, area, buf, *selected_node);
        }
        ClientPopup::SettingsPage(cmd) => {
            render_settings_page(app,area,buf,*selected_node,cmd);
        }
    }
    None
}


// Render Servers / None Available is shared between chat_rendering and net_rendering
fn render_servers_available(app:&App, area:Rect, buf:&mut Buffer, known_servers: &Vec<NodeId>, is_chat:bool){
    let [_,first_second, last] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1),Constraint::Percentage(100), Constraint::Min(1)])
        /*.horizontal_margin(1)*/
        .areas(area);
    let [first,second] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .horizontal_margin(1)
        .areas(first_second);

    // Use the appropriate cursor based on whether we're in MenuArea or OptArea
    let server_cursor = match app.cache.tab1_data.popup_selection {
        PopupSelection::MenuArea => 0,
        PopupSelection::OptArea => app.cache.tab1_data.popup_cursor, // In OptArea, popup_cursor is already the server cursor
    };

    let mut first_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    if server_cursor % 2 == 0{
        first_block = first_block.border_style(Style::default().fg(app.animation_color()));
    }

    let [first_canvas_area, first_paragraph_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        /*.vertical_margin(1)*/
        .areas(first_block.inner(first));
    let first_canvas = draw_server_canvas(app.cache.counter as u64);


    let vec_len = known_servers.len();
    // Now depending on how many server do we have we scroll the tabs.
    if usize::from(server_cursor)>=vec_len{
        // Very buggy.
        // Idea: Add a different cursor for Opt area
        // We're overflowing the Vec
        // Should be unreachable as increasing / decreasing cursor is pretty firm.
    }else{
        let tab_num = server_cursor / 2;
        let total_tabs_num = (known_servers.len()-1)/2;

        let [left_last,right_last] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .areas(last);
        let page = Line::from(format!("{}/{}", tab_num, total_tabs_num))
            .style(Style::default().add_modifier(Modifier::BOLD | Modifier::ITALIC))
            .alignment(Alignment::Right);
        page.render(right_last, buf);

        let info = Line::from(
            if is_chat{
                "Press 'r' to find new chat-servers in the network!"
            }else{
                "Press 'r' to find new content-servers in the network!"
            }
        )
            .style(Style::default().add_modifier(Modifier::BOLD | Modifier::ITALIC))
            .alignment(Alignment::Left);
        info.render(left_last, buf);

        let first_server_info = Paragraph::new(
            if is_chat {
                format!("Chat Server {:?}", known_servers.get(tab_num*2).unwrap_or(&255))
            }else{
                format!("Content Server {:?}", known_servers.get(tab_num*2).unwrap_or(&255))
            }
        )
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        first_block.render(first,buf);
        first_canvas.render(first_canvas_area, buf);
        first_server_info.render(first_paragraph_area, buf);


        // Siamo in una lista di lunghezza dispari!
        if (vec_len % 2 == 0 && server_cursor.add(1) as usize == vec_len) || server_cursor as usize +1<vec_len{
            let mut second_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded);
            if server_cursor % 2 == 1{
                second_block = second_block.border_style(Style::default().fg(app.animation_color()));
            }
            let [second_canvas_area, second_paragraph_area] = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                /*.vertical_margin(1)*/
                .areas(second_block.inner(second));
            let second_canvas = draw_server_canvas(app.cache.counter as u64);


            let second_server_info = Paragraph::new(
                if is_chat {
                    format!("Chat Server {:?}", known_servers.get(tab_num*2 + 1).unwrap_or(&255))
                }else{
                    format!("Content Server {:?}", known_servers.get(tab_num*2 + 1).unwrap_or(&255))
                }
            )
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });

            second_block.render(second,buf);
            second_canvas.render(second_canvas_area, buf);
            second_server_info.render(second_paragraph_area, buf);
        }
    }
}
fn draw_server(ctx: &mut Context) {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Derive a time value from the clock instead of an external `counter`
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let time = millis as f64 * 0.00015;

    let center_x = 20.0;
    let center_y = 20.0;
    let scale = 0.9 + (time * 0.5).sin() * 0.1;
    let rotation = (time * 0.2).sin() * 0.2;

    // Rainbow pulsing box (server body)
    let points = [
        (10.0, 10.0), (30.0, 10.0), (30.0, 30.0), (10.0, 30.0)
    ];
    for i in 0..4 {
        let (x1, y1) = points[i];
        let (x2, y2) = points[(i + 1) % 4];
        let rot_x1 = center_x + (x1 - center_x) * scale * rotation.cos() - (y1 - center_y) * scale * rotation.sin();
        let rot_y1 = center_y + (x1 - center_x) * scale * rotation.sin() + (y1 - center_y) * scale * rotation.cos();
        let rot_x2 = center_x + (x2 - center_x) * scale * rotation.cos() - (y2 - center_y) * scale * rotation.sin();
        let rot_y2 = center_y + (x2 - center_x) * scale * rotation.sin() + (y2 - center_y) * scale * rotation.cos();
        let hue = (time * 40.0 + i as f64 * 90.0) % 360.0;
        let (r, g, b) = hsv_to_rgb(hue, 0.8, 1.0);
        ctx.draw(&CanvasLine {
            x1: rot_x1, y1: rot_y1, x2: rot_x2, y2: rot_y2,
            color: Color::Rgb(r, g, b),
            ..Default::default()
        });
    }
    // Animated server lights (disks)
    for i in 0..3 {
        let y = 15.0 + i as f64 * 6.0 + (time * 1.2 + i as f64).sin() * 0.7;
        let hue = (time * 60.0 + i as f64 * 120.0) % 360.0;
        let (r, g, b) = hsv_to_rgb(hue, 0.7, 1.0);
        ctx.draw(&Circle {
            x: 20.0, y, radius: 1.2 + (time * 0.7 + i as f64).sin() * 0.3,
            color: Color::Rgb(r, g, b),
            ..Default::default()
        });
    }
    // Sparkles
    for i in 0..8 {
        let angle = time + i as f64 * 0.8;
        let radius = 16.0 + (time * 0.5).sin() * 2.0;
        let x = center_x + angle.cos() * radius;
        let y = center_y + angle.sin() * radius;
        let size = 0.3 + (time + i as f64).sin() * 0.2;
        let hue = (time * 80.0 + i as f64 * 45.0) % 360.0;
        let (r, g, b) = hsv_to_rgb(hue, 0.9, 1.0);
        ctx.draw(&Circle {
            x, y, radius: size,
            color: Color::Rgb(r, g, b),
            ..Default::default()
        });
    }
    // Pulsing ring
    let ring_radius = 18.0 + (time * 0.8).sin() * 2.0;
    let (r, g, b) = hsv_to_rgb((time * 30.0) % 360.0, 0.5, 1.0);
    ctx.draw(&Circle {
        x: center_x, y: center_y, radius: ring_radius,
        color: Color::Rgb(r, g, b),
        ..Default::default()
    });

}
fn draw_server_canvas<'a>(counter: u64) -> Canvas<'a, fn(&mut Context)> {
    Canvas::default()
        .background_color(POPUP_BG_COLOR)
        .x_bounds([0.0, 40.0])
        .y_bounds([0.0, 40.0])
        .paint(draw_server)
}

// --- New fancy "no servers found" page ---
fn render_no_servers_available(app: &App, area: Rect, buf: &mut Buffer, is_chat: bool) {
    let [top, center, bottom] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(35),
            Constraint::Percentage(25),
        ])
        .areas(area);

    // Top: Animated server icon
    let icon_canvas = draw_server_canvas(app.cache.counter as u64);
    icon_canvas.render(top, buf);

    // Center: Main message
    let main_text = Text::from(vec![
        Line::from(
            if is_chat {
                "No Chat-Server FOUND"
            } else {
                "No Content-Server FOUND"
            }
        )
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD | Modifier::ITALIC)),
        Line::from("")
    ]);
    Paragraph::new(main_text)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .render(center, buf);

    // Bottom: Tip
    let tip_text = Text::from(vec![
        Line::from(
            if is_chat {
                "💡 Press 'r' to send a network request to find new chat-servers"
            } else {
                "💡 Press 'r' to send a network request to find new content-servers"
            }
        )
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD | Modifier::ITALIC)),
    ]);
    Paragraph::new(tip_text)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .render(bottom, buf);
}

/// All the functions needed to render the chat opt page
pub(super) mod chat_rendering{
    use std::cmp::PartialEq;
    use std::collections::HashMap;
    use wg_2024::network::NodeId;
    use crate::frontend::app::App;
    use crate::frontend::app::models::tab1_types::{ChatThirdPageSelection, ClientChatPopup, ClientNetPopup, NetFirstPage, NetSecondPage, SelectedFileMedia};
    use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
    use ratatui::buffer::{Buffer, Cell};

    use ratatui::prelude::{Color, Line, Modifier, Style, Text, Widget};
    use ratatui::widgets::{Block, BorderType, Borders, Gauge, HighlightSpacing, List, ListItem, Padding, Paragraph, Wrap};
    use toml::value::Time;
    use textwrap::wrap;
    use ratatui::layout::Direction::Vertical;
    use ratatui::widgets::canvas::{Canvas, Circle, Context, Line as CanvasLine};
    use ratatui::{widgets, Frame};
    use ratatui::style::{Styled, Stylize};
    use ratatui::text::{ToSpan, ToText};
    use ratatui_image::protocol::StatefulProtocol;
    use ratatui_image::StatefulImage;
    use server::Chat;
    use server::MessageSC::{Media, Message};
    use server::MediaSC;
    use crate::backend::sc_models::{NetCacheFile, NetCacheMedia};
    use crate::frontend::app::models::main_types::AppMode;
    use crate::frontend::app::models::tab1_types::{ChSecondPage, ChThirdPage};
    use crate::frontend::app::ui::tabs::{ALTERNATE_LIST_COLOR, BG_COLOR, MAIN_BLOCK_BORDERS, POPUP_BG_COLOR, SELECTED_LIST_COLOR};
    use crate::frontend::app::ui::tabs::tab1::centered_popup_area;
    use crate::frontend::app::ui::tabs::tab1_client_popup::{hsv_to_rgb, render_no_servers_available, render_servers_available};

    // Struct to hold precomputed avatar animation parameters
    struct AvatarAnimParams {
        rings: Vec<(f64, (u8, u8, u8))>, // (radius, color)
        avatar_color: (u8, u8, u8),
        avatar_radius: f64,
        sparkles: Vec<(f64, f64, f64, (u8, u8, u8))>, // (x, y, radius, color)
        dot: (f64, f64, f64, (u8, u8, u8)), // (x, y, radius, color)
    }

    fn avatar_painter(ctx: &mut Context) {
        use std::cell::RefCell;
        thread_local! {
            static PARAMS: RefCell<Option<AvatarAnimParams>> = RefCell::new(None);
        }
        PARAMS.with(|cell| {
            if let Some(params) = &*cell.borrow() {
                // Draw rings
                for (radius, (r, g, b)) in &params.rings {
                    ctx.draw(&Circle {
                        x: 10.0, y: 10.0, radius: *radius,
                        color: Color::Rgb(*r, *g, *b),
                        ..Default::default()
                    });
                }
                // Draw avatar
                let (r, g, b) = params.avatar_color;
                ctx.draw(&Circle {
                    x: 10.0, y: 10.0, radius: params.avatar_radius,
                    color: Color::Rgb(r, g, b),
                    ..Default::default()
                });
                // Draw sparkles
                for (x, y, radius, (r, g, b)) in &params.sparkles {
                    ctx.draw(&Circle {
                        x: *x, y: *y, radius: *radius,
                        color: Color::Rgb(*r, *g, *b),
                        ..Default::default()
                    });
                }
                // Draw online dot
                let (x, y, radius, (r, g, b)) = &params.dot;
                ctx.draw(&Circle {
                    x: *x, y: *y, radius: *radius,
                    color: Color::Rgb(*r, *g, *b),
                    ..Default::default()
                });
            }
        });
    }

    fn draw_animated_profile_photo_canvas(params: AvatarAnimParams) -> Canvas<'static, fn(&mut Context)> {
        use std::cell::RefCell;
        thread_local! {
            static PARAMS: RefCell<Option<AvatarAnimParams>> = RefCell::new(None);
        }
        PARAMS.with(|cell| {
            *cell.borrow_mut() = Some(params);
        });
        Canvas::default()
            .x_bounds([0.0, 20.0])
            .y_bounds([0.0, 20.0])
            .paint(avatar_painter)
    }

    pub(super) fn render_chat_options(app:&App, area:Rect, buf: &mut Buffer, client_popup: &ClientChatPopup, selected_node: &NodeId){
        match client_popup {
            // Chat Server view option
            ClientChatPopup::FirstPage(data) => {
                render_chat_first_menu(app,area,buf,*selected_node);
            }
            ClientChatPopup::SecondPage(data) => {
                render_chat_second_menu(app,area,buf,*selected_node,data)
            }
            ClientChatPopup::ThirdPage(data) => {
                render_chat_third_menu(app,area,buf,*selected_node,data);
                if app.app_mode == AppMode::FileExploring{
                    let centered_popup_area = centered_popup_area(area);
                    widgets::Clear.render(centered_popup_area, buf);
                    let colored_bg_block = Block::default().style(Style::default().bg(POPUP_BG_COLOR));
                    let colored_popup_area = colored_bg_block
                        .inner(centered_popup_area);
                    colored_bg_block.render(centered_popup_area, buf);

                    app.cache.file_explorer.widget().render(centered_popup_area, buf);
                }
            }
        }
    }

    // First Menù rendering ( Choose the server )
    fn render_chat_first_menu(app: &App, area: Rect, buf: &mut Buffer, selected_cl:NodeId) {
        match app.simulation_controller_data.chat_cache.known_chat_servers.get(&selected_cl){
            None => {
                render_no_servers_available(app, area, buf, true);
            }
            Some(known_servers) => {
                if known_servers.is_empty() {
                    render_no_servers_available(app, area, buf, true);
                }else{
                    render_servers_available(app, area, buf, known_servers, true);
                }
            }
        }
    }

    // Second Menù rendering ( Choose contact menù )
    fn render_chat_second_menu(app:&App, area: Rect, buf: &mut Buffer, selected_cl:NodeId, sec_page_data: &ChSecondPage) {
        match app.simulation_controller_data.chat_cache.known_contacts.get(&selected_cl) {
            None => {
                render_no_contacts_available(app,area,buf);
            }
            Some(list) => {
                match list.get(&sec_page_data.selected_server) {
                    None => {
                        render_no_contacts_available(app,area,buf);
                    }
                    Some(contacts) => {
                        if contacts.is_empty() {
                            render_no_contacts_available(app,area,buf);
                        }else{
                            render_contacts_available(app,area,buf,sec_page_data,selected_cl, contacts);
                        }
                    }
                }
            }
        }
    }

    fn render_contacts_available(app:&App, area: Rect, buf: &mut Buffer, sec_page_data:&ChSecondPage, selected_cl:NodeId, contacts:&Vec<NodeId>){
        let [others, last] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(100), Constraint::Min(1)])
            .areas(area);

        let info = Line::from(" Press 'r' to find new server chats in the network! ")
            .style(Style::default().add_modifier(Modifier::ITALIC))
            .alignment(Alignment::Left);
        info.render(last, buf);

        let [first,second,third,fourth ] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(others.height / 4); 4
            ])
            .vertical_margin(1)
            .areas(others);

        let selected_index = app.cache.tab1_data.popup_cursor;
        let offset = selected_index.saturating_sub(3);
        let visible_contacts = &contacts.iter().skip(offset).take(4).collect::<Vec<_>>();

        for (i, contact_id) in visible_contacts.iter().enumerate() {
            let is_selected = selected_index == offset + i;
            let block = match i {
                0 => first,
                1 => second,
                2 => third,
                3 => fourth,
                _ => continue,
            };

            // Animated border color for selected
            let border_style = if is_selected {
                let time = app.cache.counter as f64 * 0.18 + i as f64 * 0.7;
                let (r, g, b) = hsv_to_rgb((time * 60.0) % 360.0, 0.8, 1.0);
                Style::default().fg(Color::Rgb(r, g, b))
            } else {
                Style::default()
            };
            let contact_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_style);
            let inner_contact_block = contact_block.inner(block);
            contact_block.render(block, buf);

            let [canvas_area, info_area] = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
                .areas(inner_contact_block);

            let [info_main, info_msg] = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(2)])
                .areas(info_area);

            // Precompute animation parameters for avatar
            let time = app.cache.counter as f64 * 0.18 + i as f64 * 0.7;
            let center_x = 10.0;
            let center_y = 10.0;
            let base_radius = 4.5 + (time * 0.2).sin() * 0.2;
            let mut rings = vec![];
            if is_selected {
                for ring in 0..2 {
                    let ring_radius = base_radius + 1.0 + ring as f64 * 1.2 + (time * 0.7 + ring as f64).sin() * 0.3;
                    let hue = (time * 60.0 + ring as f64 * 60.0) % 360.0;
                    let (r, g, b) = hsv_to_rgb(hue, 0.8, 1.0);
                    rings.push((ring_radius, (r, g, b)));
                }
            }
            let avatar_hue = (time * 40.0) % 360.0;
            let avatar_color = if is_selected {
                hsv_to_rgb(avatar_hue, 0.7, 1.0)
            } else {
                hsv_to_rgb(avatar_hue, 0.3, 0.7)
            };
            let mut sparkles = vec![];
            if is_selected {
                for s in 0..5 {
                    let angle = time + s as f64 * 1.2;
                    let radius = base_radius + 2.0 + (time * 0.5 + s as f64).sin() * 0.5;
                    let x = center_x + angle.cos() * radius;
                    let y = center_y + angle.sin() * radius;
                    let sparkle_hue = (time * 80.0 + s as f64 * 60.0) % 360.0;
                    let (r, g, b) = hsv_to_rgb(sparkle_hue, 0.9, 1.0);
                    sparkles.push((x, y, 0.4 + (time * 0.7 + s as f64).sin() * 0.1, (r, g, b)));
                }
            }
            let dot_y = center_y + base_radius - 1.0;
            let dot_x = center_x + base_radius - 1.0;
            let dot_brightness = 0.7 + 0.3 * (time * 2.0).sin();
            let (r, g, b) = hsv_to_rgb(120.0, 0.7, dot_brightness); // greenish
            let dot = (dot_x, dot_y, 0.7 + 0.2 * (time * 1.5).sin(), (r, g, b));
            let avatar_params = AvatarAnimParams {
                rings,
                avatar_color,
                avatar_radius: base_radius,
                sparkles,
                dot,
            };
            let canvas = draw_animated_profile_photo_canvas(avatar_params);
            canvas.render(canvas_area, buf);

            let id_text = Paragraph::new(format!("Client ID {}", contact_id))
                .alignment(Alignment::Left)
                .style(if is_selected { Style::default().add_modifier(Modifier::BOLD) } else { Style::default() });
            id_text.render(info_main, buf);

            match app.simulation_controller_data.chat_cache.local_chats.get(&sec_page_data.selected_server){
                None => {
                    let last_msg = Paragraph::new(Text::styled("Send the first message!", Style::default().add_modifier(Modifier::ITALIC).fg(Color::Gray)))
                        .alignment(Alignment::Left)
                        .wrap(Wrap { trim: true });
                    last_msg.render(info_msg, buf);
                }
                Some(chat_list) => {
                    let mut found_chat:Option<&Chat> = None;
                    match chat_list.chats_stored.get(&(selected_cl,**contact_id )){
                        None => {
                            match chat_list.chats_stored.get(&(**contact_id, selected_cl)){
                                None => (),
                                Some(ch) => {
                                    found_chat = Some(ch);
                                }
                            }
                        }
                        Some(ch) => {
                            found_chat = Some(ch);
                        }
                    }
                    match found_chat{
                        None => {
                            let last_msg = Paragraph::new(Text::styled("Send the first message!", Style::default().add_modifier(Modifier::ITALIC).fg(Color::Gray)))
                                .alignment(Alignment::Left)
                                .wrap(Wrap { trim: true });
                            last_msg.render(info_msg, buf);
                        }
                        Some(ch) => {
                            match ch.get_last_message(){
                                None => {
                                    let last_msg = Paragraph::new(Text::styled("Send the first message!", Style::default().add_modifier(Modifier::ITALIC).fg(Color::Gray)))
                                        .alignment(Alignment::Left)
                                        .wrap(Wrap { trim: true });
                                    last_msg.render(info_msg, buf);
                                }
                                Some((mess,messdata)) => {
                                    if messdata.id==selected_cl {
                                        match mess{
                                            Message(str) => {
                                                let initial_mess = format!("{:?}:{:?} - YOU: ",messdata.timestamp.time.unwrap_or(Time{ hour: 0, minute: 0, second: 0, nanosecond: 0, }).hour,
                                                                           messdata.timestamp.time.unwrap_or(Time{ hour: 0, minute: 0, second: 0, nanosecond: 0, }).minute);
                                                let last_msg = Paragraph::new(Line::from(vec![initial_mess
                                                                                                  .set_style(Style::default().add_modifier(Modifier::ITALIC).fg(Color::Gray)),
                                                                                              str.to_span()]))
                                                    .alignment(Alignment::Left)
                                                    .wrap(Wrap { trim: true });
                                                last_msg.render(info_msg, buf);
                                            }
                                            Media(med) => {
                                                let initial_mess = format!("{:?}:{:?} - You sent a media 🖇",messdata.timestamp.time.unwrap_or(Time{ hour: 0, minute: 0, second: 0, nanosecond: 0, }).hour,
                                                                           messdata.timestamp.time.unwrap_or(Time{ hour: 0, minute: 0, second: 0, nanosecond: 0, }).minute);
                                                let last_msg = Paragraph::new(Text::styled(initial_mess, Style::default().add_modifier(Modifier::ITALIC).fg(Color::Gray)))
                                                    .alignment(Alignment::Left)
                                                    .wrap(Wrap { trim: true });
                                                last_msg.render(info_msg, buf);
                                            }
                                        }
                                    }else {
                                        match mess{
                                            Message(str) => {
                                                let initial_mess = format!("{:?}:{:?} - ID {:?}: ",messdata.timestamp.time.unwrap_or(Time{ hour: 0, minute: 0, second: 0, nanosecond: 0, }).hour,
                                                                           messdata.timestamp.time.unwrap_or(Time{ hour: 0, minute: 0, second: 0, nanosecond: 0, }).minute,
                                                                           messdata.id);
                                                let last_msg = Paragraph::new(Line::from(vec![initial_mess.set_style(Style::default().add_modifier(Modifier::ITALIC).fg(Color::Gray)),
                                                                                              str.to_span()]))
                                                    .alignment(Alignment::Left)
                                                    .wrap(Wrap { trim: true });
                                                last_msg.render(info_msg, buf);
                                            }
                                            Media(med) => {
                                                let initial_mess = format!("{:?}:{:?} - ID {:?} sent a media 🖇",messdata.timestamp.time.unwrap_or(Time{ hour: 0, minute: 0, second: 0, nanosecond: 0, }).hour,
                                                                           messdata.timestamp.time.unwrap_or(Time{ hour: 0, minute: 0, second: 0, nanosecond: 0, }).minute
                                                                           ,messdata.id);
                                                let last_msg = Paragraph::new(Text::styled(initial_mess, Style::default().add_modifier(Modifier::ITALIC).fg(Color::Gray)))
                                                    .alignment(Alignment::Left)
                                                    .wrap(Wrap { trim: true });
                                                last_msg.render(info_msg, buf);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn render_no_contacts_available(app:&App, area: Rect, buf: &mut Buffer){
        let [top, center, bottom] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Percentage(35),
                Constraint::Percentage(25),
            ])
            .areas(area);

        // Top: Animated empty contacts icon
        let icon_canvas = draw_empty_contacts_canvas(app.cache.counter as u64);
        icon_canvas.render(top, buf);

        // Center: Main message
        let main_text = Text::from(vec![
            Line::from("No Contacts Available")
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD | Modifier::ITALIC)),
            Line::from("")
        ]);
        Paragraph::new(main_text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .render(center, buf);

        // Bottom: Tip
        let tip_text = Text::from(vec![
            Line::from("💡 Press 'r' to send a network request to find new contacts")
                .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD | Modifier::ITALIC)),
        ]);
        Paragraph::new(tip_text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .render(bottom, buf);
    }

    // Draw animated empty contacts icon
    fn draw_empty_contacts(ctx: &mut Context) {
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as f64 * 0.001;
        
        // Draw multiple empty contact placeholders
        for i in 0..3 {
            let x_offset = 5.0 + i as f64 * 5.0;
            let y_offset = 10.0;
            
            // Animated pulse effect
            let pulse = (time * 2.0 + i as f64 * 1.5).sin() * 0.3 + 0.7;
            let radius = 3.0 * pulse;
            
            // Draw empty contact circle with animated border
            let border_color = if i == 0 {
                // First contact gets rainbow animation
                let hue = (time * 60.0) % 360.0;
                let (r, g, b) = hsv_to_rgb(hue, 0.8, 1.0);
                Color::Rgb(r, g, b)
            } else {
                // Others get subtle animation
                let hue = (time * 30.0 + i as f64 * 60.0) % 360.0;
                let (r, g, b) = hsv_to_rgb(hue, 0.4, 0.6);
                Color::Rgb(r, g, b)
            };
            
            // Draw contact circle
            ctx.draw(&Circle {
                x: x_offset,
                y: y_offset,
                radius,
                color: Color::DarkGray,
                ..Default::default()
            });
            
            // Draw animated border
            ctx.draw(&Circle {
                x: x_offset,
                y: y_offset,
                radius: radius + 0.5,
                color: border_color,
                ..Default::default()
            });
            
            // Draw "?" inside the circle
            let question_color = if i == 0 {
                let hue = (time * 80.0) % 360.0;
                let (r, g, b) = hsv_to_rgb(hue, 0.9, 1.0);
                Color::Rgb(r, g, b)
            } else {
                Color::Gray
            };
            
            // Draw question mark using lines
            let q_scale = 0.8;
            let q_x = x_offset;
            let q_y = y_offset;
            
            // Question mark shape
            ctx.draw(&CanvasLine {
                x1: q_x - 0.5 * q_scale,
                y1: q_y - 1.0 * q_scale,
                x2: q_x + 0.5 * q_scale,
                y2: q_y - 1.0 * q_scale,
                color: question_color,
                ..Default::default()
            });
            ctx.draw(&CanvasLine {
                x1: q_x + 0.5 * q_scale,
                y1: q_y - 1.0 * q_scale,
                x2: q_x + 0.5 * q_scale,
                y2: q_y - 0.3 * q_scale,
                color: question_color,
                ..Default::default()
            });
            ctx.draw(&CanvasLine {
                x1: q_x + 0.5 * q_scale,
                y1: q_y - 0.3 * q_scale,
                x2: q_x - 0.2 * q_scale,
                y2: q_y + 0.2 * q_scale,
                color: question_color,
                ..Default::default()
            });
            ctx.draw(&CanvasLine {
                x1: q_x - 0.2 * q_scale,
                y1: q_y + 0.2 * q_scale,
                x2: q_x - 0.2 * q_scale,
                y2: q_y + 0.8 * q_scale,
                color: question_color,
                ..Default::default()
            });
            
            // Draw animated sparkles around the first contact
            if i == 0 {
                for s in 0..6 {
                    let angle = time * 2.0 + s as f64 * 1.0;
                    let sparkle_radius = 6.0 + (time * 0.5 + s as f64).sin() * 0.5;
                    let sparkle_x = x_offset + angle.cos() * sparkle_radius;
                    let sparkle_y = y_offset + angle.sin() * sparkle_radius;
                    let sparkle_hue = (time * 100.0 + s as f64 * 60.0) % 360.0;
                    let (r, g, b) = hsv_to_rgb(sparkle_hue, 0.9, 1.0);
                    
                    ctx.draw(&Circle {
                        x: sparkle_x,
                        y: sparkle_y,
                        radius: 0.3 + (time * 0.7 + s as f64).sin() * 0.1,
                        color: Color::Rgb(r, g, b),
                        ..Default::default()
                    });
                }
            }
        }
        
        // Draw connecting lines between contacts with animation
        for i in 0..2 {
            let x1 = 5.0 + i as f64 * 5.0;
            let x2 = 10.0 + i as f64 * 5.0;
            let y = 10.0;
            
            let line_alpha = (time * 1.5 + i as f64).sin() * 0.5 + 0.5;
            let line_color = if line_alpha > 0.7 {
                let hue = (time * 40.0 + i as f64 * 30.0) % 360.0;
                let (r, g, b) = hsv_to_rgb(hue, 0.6, 0.8);
                Color::Rgb(r, g, b)
            } else {
                Color::DarkGray
            };
            
            ctx.draw(&CanvasLine {
                x1,
                y1: y,
                x2,
                y2: y,
                color: line_color,
                ..Default::default()
            });
        }
        
        // Draw floating "search" particles
        for p in 0..8 {
            let angle = time * 0.5 + p as f64 * 0.8;
            let radius = 8.0 + (time * 0.3 + p as f64).sin() * 2.0;
            let x = 10.0 + angle.cos() * radius;
            let y = 10.0 + angle.sin() * radius;
            
            // Keep particles within bounds
            if x >= 0.0 && x <= 20.0 && y >= 0.0 && y <= 20.0 {
                let particle_hue = (time * 50.0 + p as f64 * 45.0) % 360.0;
                let (r, g, b) = hsv_to_rgb(particle_hue, 0.7, 0.6);
                
                ctx.draw(&Circle {
                    x,
                    y,
                    radius: 0.2 + (time * 0.5 + p as f64).sin() * 0.1,
                    color: Color::Rgb(r, g, b),
                    ..Default::default()
                });
            }
        }
    }

    fn draw_empty_contacts_canvas<'a>(counter: u64) -> Canvas<'a, fn(&mut Context)> {
        Canvas::default()
            .background_color(POPUP_BG_COLOR)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Empty Contacts"))
            .x_bounds([0.0, 20.0])
            .y_bounds([0.0, 20.0])
            .paint(draw_empty_contacts)
    }

    // Third Menù rendering ( Message with who you chose )
    fn render_chat_third_menu(app: &App, area: Rect, buf: &mut Buffer, selected_cl: NodeId, third_page_data: &ChThirdPage) {
        // Suddivido l'area e passo ai renderer di chat solo il necessario

        let chat_opt_block = Block::default()
            .border_type(BorderType::Rounded)
            .borders(Borders::LEFT | Borders::TOP)
            .style(Style::default().fg(Color::Rgb(89, 72, 198)))
            .title_alignment(Alignment::Right)
            .title_style(Style::default().add_modifier(Modifier::ITALIC).fg(Color::Rgb(163, 183, 202)))
            .title(format!("Chatting with Client {:?}", third_page_data.selected_contact));
        let inner_chat_opt_block = chat_opt_block.inner(area);
        chat_opt_block.render(area, buf);

        let [chat_area, input_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(100),Constraint::Min(3)])
            .areas(inner_chat_opt_block);

        let [text_buffer_area,three_lines_select_file_area] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(95),Constraint::Percentage(5)])
            .areas(input_area);
        let [_,select_file_area,_] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50),Constraint::Min(3), Constraint::Percentage(50)])
            .areas(three_lines_select_file_area);

        let text_buffer_block = Block::default()
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL)
            .border_style(
                match third_page_data.selected_item{
                    ChatThirdPageSelection::ScrollingMessages => Style::default().fg(Color::Rgb(89, 72, 198)),
                    ChatThirdPageSelection::BottomTab => {
                        if third_page_data.scroll_offset==0{
                            Style::default().fg(app.animation_color())
                        }else{
                            Style::default().fg(Color::Rgb(89, 72, 198))
                        }
                    }
                }
            );
        let inner_text_buffer_block = text_buffer_block.inner(text_buffer_area);
        text_buffer_block.render(text_buffer_area, buf);
        app.cache.text_buffer.chat_text_area.render(inner_text_buffer_block, buf);

        // Draw attach-file option (paperclip icon + label)
        let attach_para = Paragraph::new("🖇️")
            .block(Block::default().borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(
                    match third_page_data.selected_item{
                        ChatThirdPageSelection::ScrollingMessages => Style::default().fg(Color::Rgb(89, 72, 198)),
                        ChatThirdPageSelection::BottomTab => {
                            if third_page_data.scroll_offset==1{
                                Style::default().fg(app.animation_color())
                            }else{
                                Style::default().fg(Color::Rgb(89, 72, 198))
                            }
                        }
                    }
                ))
            .alignment(Alignment::Center);

        attach_para.render(select_file_area, buf);

        //
        match app.simulation_controller_data.chat_cache.local_chats.get(&third_page_data.selected_server) {
            None => {
                // Server has no chats
                render_empty_chat(app,chat_area,buf);
            }
            Some(chat_list) => {
                match chat_list.chats_stored.get(&(selected_cl,third_page_data.selected_contact )){
                    None => {
                        match chat_list.chats_stored.get(&(third_page_data.selected_contact, selected_cl)){
                            None => {
                                render_empty_chat(app,chat_area,buf);
                            }
                            Some(ch) => {
                                render_chats_2(app,chat_area,buf,selected_cl,third_page_data,ch)
                            }
                        }
                    }
                    Some(ch) => {
                        render_chats_2(app,chat_area,buf,selected_cl,third_page_data,ch)
                    }
                }
            }
        }
    }

    fn render_empty_chat(app:&App, area:Rect, buf:&mut Buffer){

    }

    fn render_chats(app: &App, area: Rect, buf: &mut Buffer, selected_cl: NodeId, third_page_data: &ChThirdPage, ch:&Chat ){

        // Messages list (scrollable) with single unified offset
        let msgs = &ch.chat;
        let total = msgs.len();
        // Single offset: 0 = newest (bottom), increasing walks upward through history
        let offset = app.cache.tab1_data.popup_cursor as usize;
        // Row height approximation (~3 lines per message bubble)
        let view_count = (area.height as usize / 3).max(1);

        // Scroll keeps the selected message (offset) visible.
        // If offset < view_count we don't scroll yet; once it equals view_count‒1
        // the next upward move begins scrolling while the cursor sticks to the top row.
        let scroll = if offset >= view_count { offset - (view_count - 1) } else { 0 };

        // Determine slice [start, end) in global message vector
        let end = total.saturating_sub(scroll);
        let visible_len = view_count.min(end); // could be < view_count near the beginning
        let start = end - visible_len;
        let visible = &msgs[start..end];

        // Relative index of selected message inside visible slice
        // sel_rel = offset - scroll   (0 = bottom row)
        let sel_rel = offset.saturating_sub(scroll);
        let sel_index = visible_len.saturating_sub(1).saturating_sub(sel_rel);

        // Build horizontal layout and capture rectangles
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);
        let msgs_col = cols[0];
        let scrollbar_col = cols[1];

        // Centered top-down layout for visible messages
        // First compute each row's height
        let mut row_heights = Vec::with_capacity(visible.len());
        for (msg, _) in visible.iter() {
            let content = match msg {
                Message(text) => text.clone(),
                Media(_) => "[MEDIA] 🖇️ Select this message to open the file!".to_string(),
            };
            let wrapped = wrap(&content, msgs_col.width.saturating_sub(2) as usize);
            let lines = wrapped.len().max(1) as u16;
            let height = lines + 3; // 2 padding + 1 timestamp
            row_heights.push(height);
        }
        let total_height: u16 = row_heights.iter().sum();
        // Start Y so messages are vertically centered
        let mut current_y = msgs_col.y + (msgs_col.height.saturating_sub(total_height) / 2);

        // Draw each message in order (oldest at top)
        for (i, (msg, meta)) in visible.iter().enumerate() {
            let row_height = row_heights[i];
            // Only render messages that fully fit in the messages pane
            if current_y < msgs_col.y || current_y + row_height > msgs_col.y + msgs_col.height {
                break;
            }
            let row_area = Rect::new(msgs_col.x, current_y, msgs_col.width, row_height);
            current_y += row_height;

            // Determine if sent or received and if selected
            let is_sent = meta.id == selected_cl;
            let is_selected_msg = i == sel_index;

            // Split left/right for bubble
            let cols = if is_sent {
                [Constraint::Percentage(30), Constraint::Percentage(70)]
            } else {
                [Constraint::Percentage(70), Constraint::Percentage(30)]
            };
            let parts = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(cols)
                .split(row_area);
            let bubble_area = if is_sent { parts[1] } else { parts[0] };


            let time_str = meta.timestamp
                .time
                .map(|t| format!("{:02}:{:02}", t.hour, t.minute))
                .unwrap_or_default();

            // Draw bubble block (rounded, bold if selected)
            let bubble_color = if is_sent { Color::Rgb(117, 147, 175) } else { Color::Rgb(163, 183, 202) };
            let border_ty = BorderType::Rounded;
            let mut bubble_style = if is_selected_msg { Style::default().fg(Color::Rgb(147, 133, 227))} else { Style::default().fg(bubble_color) };
            if is_selected_msg {
                bubble_style = bubble_style.add_modifier(Modifier::BOLD);
            }
            let bubble_block = Block::default()
                .borders(Borders::ALL)
                .border_type(border_ty)
                .style(bubble_style)
                .title_bottom(time_str)
                .title_alignment(if is_sent {Alignment::Left} else {Alignment::Right})
                .title_style(Style::default().fg(Color::Gray));

            // Wrap and render message text
            let content = match msg {
                Message(text) => text.clone(),
                Media(_) => "[media]".to_string(),
            };
            let wrapped_text = wrap(&content, bubble_area.width.saturating_sub(2) as usize).join("\n");
            let mut text_style = Style::default().add_modifier(Modifier::ITALIC);
            if is_selected_msg {
                text_style = text_style.add_modifier(Modifier::BOLD);
            }
            Paragraph::new(wrapped_text)
                .block(bubble_block)
                .wrap(Wrap { trim: false })
                .alignment(if is_sent { Alignment::Right } else { Alignment::Left })
                .style(text_style)
                .render(bubble_area, buf);

            /*// Render timestamp at bottom-left
            let time_str = meta.timestamp
                .time
                .map(|t| format!("{:02}:{:02}", t.hour, t.minute))
                .unwrap_or_default();
            let ts_area = Rect::new(
                bubble_area.x + 1,
                bubble_area.y + bubble_area.height.saturating_sub(1),
                bubble_area.width.saturating_sub(2),
                1,
            );
            Paragraph::new(time_str)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Left)
                .render(ts_area, buf);*/
        }

        // Draw scrollbar on the right
        // scrollbar_col already defined above
        for y in 0..scrollbar_col.height {
            buf.get_mut(scrollbar_col.x, scrollbar_col.y + y)
                .set_symbol(" ")
                .set_style(Style::default().bg(Color::DarkGray));
        }
        let total_msgs = total as f64;
        let vis_msgs = visible.len() as f64;
        let area_h = scrollbar_col.height as f64;
        let thumb_h = ((vis_msgs / total_msgs) * area_h).ceil().max(1.0) as u16;
        let max_offset = total.saturating_sub(1) as f64;
        let scroll_f = scroll as f64;
        let avail = area_h - thumb_h as f64;
        let thumb_pos = if max_offset > 0.0 {
            ((max_offset - scroll_f) / max_offset * avail).floor() as u16
        } else {
            0
        };
        for dy in 0..thumb_h.min(scrollbar_col.height) {
            Buffer::cell_mut(buf,(scrollbar_col.x, scrollbar_col.y + thumb_pos + dy)).unwrap_or(&mut Cell::default())
                .set_symbol(" ")
                .set_style(Style::default().bg(Color::White));
            /*        buf.get_mut(scrollbar_col.x, scrollbar_col.y + thumb_pos + dy)
                        .set_symbol(" ")
                        .set_style(Style::default().bg(Color::White));*/
        }

    }

    fn render_chats_2(app: &App, area: Rect, buf: &mut Buffer, selected_cl: NodeId, third_page_data: &ChThirdPage, ch: &Chat) {
        use textwrap::wrap;
        use ratatui::widgets::{Block, Borders, BorderType, Paragraph, Wrap};
        use ratatui::style::{Color, Style, Modifier};
        use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect, Size};
        use tui_scrollview::{ScrollView, ScrollViewState};

        // Get all messages from the chat
        let msgs = &ch.chat;
        let total = msgs.len();
        if total == 0 {
            // No messages to display
            return;
        }

        // Calculate total height needed for all messages
        let mut total_height = 0;
        let mut message_heights = Vec::with_capacity(total);
        
        // Use a more conservative width to prevent horizontal scrolling
        let available_width = area.width.saturating_sub(2); // Leave margin for scrollbar
        
        for (msg, _) in msgs.iter() {
            // Calculate height based on message type
            let height = match msg {
                Message(text) => {
                    let wrap_width = available_width.saturating_sub(8) as usize;
                    let wrapped = wrap(&text, wrap_width);
                    let lines = wrapped.len().max(1) as u16;
                    lines + 3 // 2 padding + 1 timestamp
                }
                Media(media_sc) => {
                    // Media messages have fixed height for the visual representation
                    match media_sc {
                        MediaSC::Image(_, _) => 5, // Image preview
                        MediaSC::File(_, _) => 4,  // File info
                    }
                }
            };
            message_heights.push(height);
            total_height += height;
        }

        // Create scroll view with constrained width to prevent horizontal scrolling
        let mut scroll_view = ScrollView::new(ratatui::layout::Size::new(available_width, total_height));
        let scroll_buf = scroll_view.buf_mut();

        // Fill the entire scroll buffer with the popup background color
        for y in 0..scroll_buf.area.height {
            for x in 0..scroll_buf.area.width {
                scroll_buf.get_mut(x, y).set_style(Style::default().bg(POPUP_BG_COLOR));
            }
        }

        // Calculate which message is selected based on scroll_offset
        // scroll_offset = 0 means the last message (most recent) is selected
        let selected_message_index = if third_page_data.scroll_offset == 0 {
            total.saturating_sub(1) // Last message
        } else {
            total.saturating_sub(third_page_data.scroll_offset as usize + 1)
        };

        // Render each message bubble into the scroll buffer
        let mut current_y = 0;
        for (i, (msg, meta)) in msgs.iter().enumerate() {
            let row_height = message_heights[i];
            let row_area = Rect::new(0, current_y, available_width, row_height);
            current_y += row_height;

            // Determine if sent or received and if this message is selected
            let is_sent = meta.id == selected_cl;
            let is_selected = (i == selected_message_index && third_page_data.selected_item == ChatThirdPageSelection::ScrollingMessages);

            // Split left/right for bubble with more conservative margins
            let cols = if is_sent {
                [Constraint::Percentage(20), Constraint::Percentage(80)]
            } else {
                [Constraint::Percentage(80), Constraint::Percentage(20)]
            };
            let parts = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(cols)
                .split(row_area);
            let bubble_area = if is_sent { parts[1] } else { parts[0] };

            // Format timestamp
            let time_str = meta.timestamp
                .time
                .map(|t| format!("{:02}:{:02}", t.hour, t.minute))
                .unwrap_or_default();

            // Create bubble block with timestamp and selection styling
            let bubble_color = if is_sent { 
                if is_selected {
                    app.animation_color() // Purple for selected sent messages
                } else {
                    Color::Rgb(117, 147, 175) // Original blue for sent messages
                }
            } else { 
                if is_selected {
                    app.animation_color()// Purple for selected received messages
                } else {
                    Color::Rgb(163, 183, 202) // Original gray for received messages
                }
            };

            let mut bubble_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title_bottom(time_str)
                .title_alignment(if is_sent { Alignment::Left } else { Alignment::Right })
                .title_style(Style::default().fg(Color::Gray))
                .border_style(Style::default().fg(bubble_color));

            // Render message content based on type
            match msg {
                Message(text) => {
                    // Regular text message
                    let wrap_width = bubble_area.width.saturating_sub(6) as usize;
                    let wrapped_text = wrap(&text, wrap_width).join("\n");

                    let text_style = if is_selected {
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    Paragraph::new(wrapped_text)
                        .block(bubble_block)
                        .style(text_style)
                        .wrap(Wrap { trim: false })
                        .alignment(if is_sent { Alignment::Right } else { Alignment::Left })
                        .render(bubble_area, scroll_buf);
                }
                Media(media_sc) => {
                    // Media message with visual representation
                    match media_sc {
                        MediaSC::Image(image_id, _) => {
                            // Image message with preview
                            let image_content = format!("🖼️  Image: {}", image_id);
                            let image_style = if is_selected {
                                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::Yellow)
                            };

                            Paragraph::new(image_content)
                                .block(bubble_block)
                                .style(image_style)
                                .wrap(Wrap { trim: false })
                                .alignment(if is_sent { Alignment::Right } else { Alignment::Left })
                                .render(bubble_area, scroll_buf);
                        }
                        MediaSC::File(file_id, _) => {
                            // File message with file info
                            let file_content = format!("📄 File: {}", file_id);
                            let file_style = if is_selected {
                                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::Cyan)
                            };

                            Paragraph::new(file_content)
                                .block(bubble_block)
                                .style(file_style)
                                .wrap(Wrap { trim: false })
                                .alignment(if is_sent { Alignment::Right } else { Alignment::Left })
                                .render(bubble_area, scroll_buf);
                        }
                    }
                }
            }
        }

        // Calculate scroll position to keep selected message visible
        let mut scroll_state = ScrollViewState::default();
        
        // Calculate the position of the selected message
        let mut cumulative_height: usize = 0;
        for i in 0..selected_message_index {
            cumulative_height += message_heights[i] as usize;
        }
        
        // Scroll to position the selected message in the center of the view
        let view_height = area.height as usize;
        let selected_message_height = message_heights[selected_message_index] as usize;
        let mut target_scroll = cumulative_height.saturating_sub(view_height / 2).saturating_add(selected_message_height / 2);
        
        // Ensure first message is fully visible at the top
        if selected_message_index == 0 {
            target_scroll = 0;
        }
        
        // Clamp scroll so the last message is always fully visible
        let max_scroll = total_height.saturating_sub(view_height as u16);
        if target_scroll > max_scroll as usize {
            target_scroll = max_scroll as usize;
        }
        
        // Apply scroll
        for _ in 0..target_scroll {
            scroll_state.scroll_down();
        }
        
        ratatui::widgets::StatefulWidget::render(scroll_view, area, buf, &mut scroll_state);
    }
}

/// All the functions needed to render the internet opt page
pub(super) mod net_rendering{
    use std::cmp::PartialEq;
    use std::collections::HashMap;
    use wg_2024::network::NodeId;
    use crate::frontend::app::App;
    use crate::frontend::app::models::tab1_types::{ChatThirdPageSelection, ClientChatPopup, ClientNetPopup, NetFirstPage, NetSecondPage, SelectedFileMedia};
    use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Position, Rect, Size};
    use ratatui::buffer::{Buffer, Cell};

    use ratatui::prelude::{Color, Line, Modifier, Style, Text, Widget};
    use ratatui::widgets::{Block, BorderType, Borders, Gauge, HighlightSpacing, List, ListItem, Padding, Paragraph, Wrap};
    use toml::value::Time;
    use textwrap::wrap;
    use ratatui::layout::Direction::Vertical;
    use ratatui::widgets::canvas::{Canvas, Circle, Context, Line as CanvasLine};
    use std::ops::Add;
    use std::sync::mpsc::Receiver;
    use client::Client;
    use host::{ClientId, FileId, MediaId, ServerId};
    use log::error;
    use ratatui::{widgets, Frame};
    use ratatui::style::{Styled, Stylize};
    use ratatui::text::{ToSpan, ToText};
    use ratatui_image::protocol::StatefulProtocol;
    use ratatui_image::StatefulImage;
    use crate::backend::sc_models::{FileType, NetCacheFile, NetCacheMedia};
    use crate::frontend::app::models::main_types::AppMode;
    use crate::frontend::app::models::tab1_types::{ChSecondPage, ChThirdPage};
    use crate::frontend::app::ui::tabs::{ALTERNATE_LIST_COLOR, BG_COLOR, MAIN_BLOCK_BORDERS, POPUP_BG_COLOR, SELECTED_LIST_COLOR};
    use crate::frontend::app::ui::tabs::tab1::centered_popup_area;
    use crate::frontend::app::ui::tabs::tab1_client_popup::{hsv_to_rgb, render_no_servers_available, render_servers_available};

    pub(super) fn render_net_options(app:&App, area:Rect, buf: &mut Buffer, client_popup: &ClientNetPopup, selected_node: &NodeId)->Option<Rect>{
        match client_popup {
            ClientNetPopup::FirstPage(st) => {
                // Choose the Content Server
                render_net_first_page(app,area,buf,st,selected_node);
                None
            }
            ClientNetPopup::SecondPage(nd) => {
                let ret = render_net_second_page(app,area,buf,nd,selected_node);
                if app.app_mode == AppMode::FileExploring{
                    let centered_popup_area = centered_popup_area(area);
                    widgets::Clear.render(centered_popup_area, buf);
                    let colored_bg_block = Block::default().style(Style::default().bg(POPUP_BG_COLOR));
                    let colored_popup_area = colored_bg_block
                        .inner(centered_popup_area);
                    colored_bg_block.render(centered_popup_area, buf);
                    app.cache.file_explorer.widget().render(centered_popup_area, buf);
                }
                ret
            }
        }
    }
    fn render_net_first_page(app:&App, area:Rect, buf: &mut Buffer, stpage_data: &NetFirstPage, selected_node: &ClientId){
        match app.simulation_controller_data.net_cache.known_content_servers.get(&selected_node) {
            None => {
                render_no_servers_available(app,area,buf,false)
            }
            Some(known_servers) => {
                if known_servers.is_empty() {
                    render_no_servers_available(app,area,buf,false)
                }else{
                    render_servers_available(app,area,buf,known_servers,false)
                }
            }
        }
    }
    fn render_net_second_page(app:&App, area:Rect, buf: &mut Buffer, ndpage_data: &NetSecondPage, selected_node: &ClientId)->Option<Rect>{
        let [lists_area,file_visualizer_area] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .spacing(1)
            .areas(area);

        let [media_area, file_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .spacing(1)
            .areas(lists_area);

        let [media_list_area, attach_media_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(100), Constraint::Min(3)])
            .areas(media_area);

        let [file_list_area, attach_file_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(100), Constraint::Min(3)])
            .areas(file_area);


        let media_list_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(
                if app.cache.tab1_data.popup_cursor == 0 {
                    app.animation_color()
                }else{
                    MAIN_BLOCK_BORDERS
                })
            )
            .title(" Media List ")
            .title_alignment(Alignment::Center);

        let attach_media_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(
                if app.cache.tab1_data.popup_cursor == 1 {
                    app.animation_color()
                }else{
                    MAIN_BLOCK_BORDERS
                })
            );

        // Render animated attach media button
        let inner_attach_media = attach_media_block.inner(attach_media_area);
        attach_media_block.render(attach_media_area, buf);

        let upload_media_text = Paragraph::new("Upload Media")
            .style(Style::default().fg(MAIN_BLOCK_BORDERS))
            .alignment(Alignment::Center);

        upload_media_text.render(inner_attach_media, buf);


        let file_list_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(
                if app.cache.tab1_data.popup_cursor == 2 {
                    app.animation_color()
                }else{
                    MAIN_BLOCK_BORDERS
                })
            )
            .title(" File List ")
            .title_alignment(Alignment::Center);

        let mut attach_file_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(
                if app.cache.tab1_data.popup_cursor == 3 {
                    app.animation_color()
                }else{
                    MAIN_BLOCK_BORDERS
                })
            );

        // Render animated attach file button
        let inner_attach_file = attach_file_block.inner(attach_file_area);
        attach_file_block.render(attach_file_area, buf);

        let upload_file_text = Paragraph::new("Upload File")
            .style(Style::default().fg(MAIN_BLOCK_BORDERS))
            .alignment(Alignment::Center);

        upload_file_text.render(inner_attach_file, buf);

        let media_items = match app.simulation_controller_data.net_cache.known_media_list.get(&selected_node) {
            None => vec![],
            Some(hash) => {
                match hash.get(&ndpage_data.selected_server){
                    None => vec![],
                    Some(vec_media) => {
                        vec_media
                            .iter()
                            .enumerate()
                            .map(|(idx, id)| {
                                let base_style = if idx % 2 == 0 {
                                    // even rows – brighter foreground
                                    Style::default().bg(BG_COLOR)
                                } else {
                                    // odd rows – dimmer foreground for contrast
                                    Style::default().bg(ALTERNATE_LIST_COLOR)
                                };
                                match app.simulation_controller_data.net_cache.downloaded_medias.get(id.as_str()){
                                    None => {
                                        ListItem::from(format!("{:?}",id)).style(base_style)
                                    }
                                    Some(media_entry) => {
                                        let load_char = match &media_entry.cache{
                                            NetCacheMedia::Downloading(_) | NetCacheMedia::Rendering(_) => {
                                                match app.cache.counter % 4 {
                                                    0 => '|', 1=> '/', 2=> '-', 3 => '\\', _ => ' '
                                                }
                                            }
                                            NetCacheMedia::Downloaded(_) => {
                                                match app.cache.counter % 20 {
                                                    (num) => {
                                                        if num<10{
                                                            '✓'
                                                        }else{
                                                            ' '
                                                        }
                                                    }
                                                }
                                            }
                                            NetCacheMedia::Rendered(_) => '✓',
                                            _ => {
                                                ' '
                                            }
                                        };
                                        ListItem::from(format!("{} - {:?}",load_char,id)).style(base_style)
                                    }
                                }
                            })
                            .collect()
                    }
                }
            }
        };
        let file_items = match app.simulation_controller_data.net_cache.known_file_list.get(&selected_node) {
            None => vec![],
            Some(hash) => {
                match hash.get(&ndpage_data.selected_server){
                    None => vec![],
                    Some(vec_file) => {
                        vec_file
                            .iter()
                            .enumerate()
                            .map(|(idx, id)| {
                                let base_style = if idx % 2 == 0 {
                                    // even rows – brighter foreground
                                    Style::default().bg(BG_COLOR)
                                } else {
                                    // odd rows – dimmer foreground for contrast
                                    Style::default().bg(ALTERNATE_LIST_COLOR)
                                };
                                match app.simulation_controller_data.net_cache.downloaded_files.get(id.as_str()){
                                    None => {
                                        ListItem::from(format!("{:?}",id)).style(base_style)
                                    }
                                    Some(item) => {
                                        let load_char = match item.value(){
                                            NetCacheFile::Downloading(_) | NetCacheFile::Rendering(_) => {
                                                match app.cache.counter % 4 {
                                                    0 => '|', 1=> '/', 2=> '-', 3 => '\\', _ => ' '
                                                }
                                            }
                                            NetCacheFile::Downloaded(_) => {
                                                match app.cache.counter % 20 {
                                                    (num) => {
                                                        if num<10{
                                                            '✓'
                                                        }else{
                                                            ' '
                                                        }
                                                    }
                                                }
                                            }
                                            NetCacheFile::Rendered(_) => '✓',
                                        };
                                        ListItem::from(format!("{} - {:?}",load_char,id)).style(base_style)
                                    }
                                }
                            })
                            .collect()
                    }
                }
            }
        };
        let media_list = List::new(media_items)
            .block(media_list_block)
            // Keep a cohesive palette: selected rows use the global BG_COLOR with a bright accent
            .highlight_style(
                Style::default()
                    .bg(SELECTED_LIST_COLOR)
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);

        let file_list = List::new(file_items)
            .block(file_list_block)
            // Keep a cohesive palette: selected rows use the global BG_COLOR with a bright accent
            .highlight_style(
                Style::default()
                    .bg(SELECTED_LIST_COLOR)
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);

        let mut mediastate = ratatui::widgets::ListState::default();
        mediastate.select(Some(app.cache.tab1_data.media_list_render_utils.selected as usize));
        // Render the list
        ratatui::widgets::StatefulWidget::render(
            media_list,
            media_list_area,
            buf,
            &mut mediastate
        );

        let mut filestate = ratatui::widgets::ListState::default();
        filestate.select(Some(app.cache.tab1_data.file_list_render_utils.selected as usize));
        ratatui::widgets::StatefulWidget::render(
            file_list,
            file_list_area,
            buf,
            &mut filestate
        );


        // Da fare entro stasera:
        /*
            Block of file renderer
            File rendering if it is txt
            Loading Bar if receiving
            File renderer

        */

        let file_visualizer_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(
                if app.cache.tab1_data.popup_cursor == 4 {
                    app.animation_color()
                }else{
                    MAIN_BLOCK_BORDERS
                })
            );

        let inner_file_visualizer_block = file_visualizer_block.inner(file_visualizer_area);

        /*
        Ora qui il rendering dipende da:

        Ho selezionato un file ?
            SI - Processo lo status
                1] Richiedo il file
                2] Downloading(u8)
                3] Downloaded(File)
                    - Loading File(Renderizzando l'immagine / file) <- Bisogna trovare un modo per non renderlo bloccante
                    - Loaded(StatufulProtocol / File)
            NO - Mostro la schermata "Select a file to view"
         */

        match ndpage_data.selected_file.clone() {
            SelectedFileMedia::None => {
                // Render cool None page with animated file icon and helpful text
                let [top_area, center_area, bottom_area] = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(20),
                        Constraint::Percentage(60),
                        Constraint::Percentage(20)
                    ])
                    .areas(inner_file_visualizer_block);

                // Top area: Animated file icon
                let file_icon_canvas = Canvas::default()
                    .background_color(POPUP_BG_COLOR)
                    .x_bounds([0.0, 40.0])
                    .y_bounds([0.0, 40.0])
                    .paint(|ctx| {
                        let time = app.cache.counter as f64 * 0.15;

                        // DNA-like helix effect in background
                        for i in 0..20 {
                            let angle = time * 0.5 + (i as f64 * 0.314); // 18 degrees apart
                            let radius = 12.0 + (time * 0.3).sin() * 2.0;
                            let x = 20.0 + angle.cos() * radius;
                            let y = 20.0 + angle.sin() * radius + (i as f64 * 0.5);
                            let size = 0.2 + (time + i as f64 * 0.2).sin() * 0.1;

                            let helix_color = if i % 2 == 0 {
                                Color::Rgb(100, 200, 255)
                            } else {
                                Color::Rgb(255, 100, 200)
                            };

                            ctx.draw(&Circle {
                                x, y, radius: size,
                                color: helix_color,
                                ..Default::default()
                            });
                        }

                        // Magnetic field lines
                        for line in 0..6 {
                            let line_angle = time * 0.2 + line as f64 * 1.047; // 60 degrees apart
                            let start_x = 20.0 + line_angle.cos() * 5.0;
                            let start_y = 20.0 + line_angle.sin() * 5.0;
                            let end_x = 20.0 + line_angle.cos() * 35.0;
                            let end_y = 20.0 + line_angle.sin() * 35.0;

                            let field_intensity = (100 + (time * 30.0 + line as f64 * 20.0).sin() as u8).min(200);
                            ctx.draw(&CanvasLine {
                                x1: start_x, y1: start_y, x2: end_x, y2: end_y,
                                color: Color::Rgb(field_intensity, field_intensity, 255),
                                ..Default::default()
                            });
                        }

                        // Floating particles background with rainbow effect
                        for i in 0..8 {
                            let angle = time + (i as f64 * 0.785); // 45 degrees apart
                            let radius = 15.0 + (time * 0.5).sin() * 3.0;
                            let x = 20.0 + angle.cos() * radius;
                            let y = 20.0 + angle.sin() * radius;
                            let size = 0.5 + (time + i as f64).sin() * 0.3;

                            // Color cycling through rainbow
                            let hue = (time * 20.0 + i as f64 * 45.0) % 360.0;
                            let (r, g, b) = hsv_to_rgb(hue, 0.8, 1.0);

                            ctx.draw(&Circle {
                                x, y, radius: size,
                                color: Color::Rgb(r, g, b),
                                ..Default::default()
                            });
                        }

                        // Main file icon with rotation and scaling
                        let rotation = time * 0.3;
                        let scale = 0.8 + (time * 0.5).sin() * 0.2;
                        let center_x = 20.0;
                        let center_y = 20.0;

                        // File body with rotation and rainbow gradient
                        let points = [
                            (8.0, 5.0), (32.0, 5.0), (32.0, 35.0), (8.0, 35.0)
                        ];

                        for i in 0..4 {
                            let (x1, y1) = points[i];
                            let (x2, y2) = points[(i + 1) % 4];

                            // Apply rotation and scaling
                            let rot_x1 = center_x + (x1 - center_x) * scale * rotation.cos() - (y1 - center_y) * scale * rotation.sin();
                            let rot_y1 = center_y + (x1 - center_x) * scale * rotation.sin() + (y1 - center_y) * scale * rotation.cos();
                            let rot_x2 = center_x + (x2 - center_x) * scale * rotation.cos() - (y2 - center_y) * scale * rotation.sin();
                            let rot_y2 = center_y + (x2 - center_x) * scale * rotation.sin() + (y2 - center_y) * scale * rotation.cos();

                            // Dynamic rainbow color
                            let hue = (time * 30.0 + i as f64 * 90.0) % 360.0;
                            let (r, g, b) = hsv_to_rgb(hue, 0.7, 1.0);

                            ctx.draw(&CanvasLine {
                                x1: rot_x1, y1: rot_y1, x2: rot_x2, y2: rot_y2,
                                color: Color::Rgb(r, g, b),
                                ..Default::default()
                            });
                        }

                        // Animated file tab with glow effect
                        let tab_points = [(8.0, 5.0), (15.0, 2.0), (22.0, 5.0)];
                        for i in 0..2 {
                            let (x1, y1) = tab_points[i];
                            let (x2, y2) = tab_points[i + 1];

                            let rot_x1 = center_x + (x1 - center_x) * scale * rotation.cos() - (y1 - center_y) * scale * rotation.sin();
                            let rot_y1 = center_y + (x1 - center_x) * scale * rotation.sin() + (y1 - center_y) * scale * rotation.cos();
                            let rot_x2 = center_x + (x2 - center_x) * scale * rotation.cos() - (y2 - center_y) * scale * rotation.sin();
                            let rot_y2 = center_y + (x2 - center_x) * scale * rotation.sin() + (y2 - center_y) * scale * rotation.cos();

                            // Glow effect with multiple lines
                            for glow in 0..3 {
                                let glow_alpha = (255 - glow * 80) as u8;
                                ctx.draw(&CanvasLine {
                                    x1: rot_x1, y1: rot_y1, x2: rot_x2, y2: rot_y2,
                                    color: Color::Rgb(glow_alpha, glow_alpha, 255),
                                    ..Default::default()
                                });
                            }
                        }

                        // Animated text lines with wave effect and rainbow cycling
                        for i in 0..4 {
                            let base_y = 10.0 + (i as f64 * 5.0);
                            let wave_offset = (time + i as f64 * 0.5).sin() * 2.0;
                            let y_pos = base_y + wave_offset;

                            let line_length = 16.0 + (time + i as f64).sin() * 4.0;
                            let start_x = 12.0 - line_length * 0.5;
                            let end_x = 12.0 + line_length * 0.5;

                            // Apply rotation to text lines
                            let rot_start_x = center_x + (start_x - center_x) * scale * rotation.cos() - (y_pos - center_y) * scale * rotation.sin();
                            let rot_start_y = center_y + (start_x - center_x) * scale * rotation.sin() + (y_pos - center_y) * scale * rotation.cos();
                            let rot_end_x = center_x + (end_x - center_x) * scale * rotation.cos() - (y_pos - center_y) * scale * rotation.sin();
                            let rot_end_y = center_y + (end_x - center_x) * scale * rotation.sin() + (y_pos - center_y) * scale * rotation.cos();

                            // Rainbow color cycling for text lines
                            let hue = (time * 40.0 + i as f64 * 60.0) % 360.0;
                            let (r, g, b) = hsv_to_rgb(hue, 0.8, 1.0);

                            ctx.draw(&CanvasLine {
                                x1: rot_start_x, y1: rot_start_y, x2: rot_end_x, y2: rot_end_y,
                                color: Color::Rgb(r, g, b),
                                ..Default::default()
                            });
                        }

                        // Animated cursor with energy field and rainbow trail
                        let cursor_base_x = 12.0;
                        let cursor_x = cursor_base_x + (time * 2.0).sin() * 8.0;
                        let cursor_y = 12.0 + (time * 0.8).sin() * 2.0;

                        // Energy field around cursor
                        let energy_radius = 8.0 + (time * 0.6).sin() * 2.0;
                        ctx.draw(&Circle {
                            x: cursor_x, y: cursor_y, radius: energy_radius,
                            color: Color::Rgb(100, 255, 100),
                            ..Default::default()
                        });

                        // Cursor trail with rainbow effect
                        for trail in 0..5 {
                            let trail_x = cursor_x - trail as f64 * 0.8;
                            let trail_y = cursor_y + (time * 0.8 + trail as f64 * 0.3).sin() * 1.5;
                            let trail_alpha = (255 - trail * 50) as u8;

                            let rot_trail_x = center_x + (trail_x - center_x) * scale * rotation.cos() - (trail_y - center_y) * scale * rotation.sin();
                            let rot_trail_y = center_y + (trail_x - center_x) * scale * rotation.sin() + (trail_y - center_y) * scale * rotation.cos();

                            let trail_hue = (time * 50.0 + trail as f64 * 30.0) % 360.0;
                            let (r, g, b) = hsv_to_rgb(trail_hue, 0.9, trail_alpha as f64 / 255.0);

                            ctx.draw(&CanvasLine {
                                x1: rot_trail_x, y1: rot_trail_y, x2: rot_trail_x, y2: rot_trail_y + 4.0,
                                color: Color::Rgb(r, g, b),
                                ..Default::default()
                            });
                        }

                        // Pulsing selection ring with multiple layers
                        for ring in 0..3 {
                            let ring_radius = 25.0 + (time * 0.8).sin() * 3.0 + ring as f64 * 2.0;
                            let ring_alpha = (100 + (time * 50.0).sin() as u8).min(200) - ring * 50;
                            if ring_alpha > 0 {
                                ctx.draw(&Circle {
                                    x: center_x, y: center_y, radius: ring_radius,
                                    color: Color::Rgb(ring_alpha, ring_alpha, 255),
                                    ..Default::default()
                                });
                            }
                        }

                        // Sparkle effects with rainbow colors
                        for sparkle in 0..8 {
                            let sparkle_angle = time * 0.7 + sparkle as f64 * 0.785; // 45 degrees apart
                            let sparkle_radius = 18.0 + (time * 0.3).sin() * 2.0;
                            let sparkle_x = center_x + sparkle_angle.cos() * sparkle_radius;
                            let sparkle_y = center_y + sparkle_angle.sin() * sparkle_radius;
                            let sparkle_size = 0.3 + (time + sparkle as f64).sin() * 0.3;

                            // Rainbow sparkles
                            let sparkle_hue = (time * 60.0 + sparkle as f64 * 45.0) % 360.0;
                            let (r, g, b) = hsv_to_rgb(sparkle_hue, 1.0, 1.0);

                            ctx.draw(&Circle {
                                x: sparkle_x, y: sparkle_y, radius: sparkle_size,
                                color: Color::Rgb(r, g, b),
                                ..Default::default()
                            });
                        }

                        // Quantum particles (smaller, faster particles)
                        for quantum in 0..12 {
                            let quantum_angle = time * 2.0 + quantum as f64 * 0.523; // 30 degrees apart
                            let quantum_radius = 8.0 + (time * 1.0).sin() * 4.0;
                            let quantum_x = center_x + quantum_angle.cos() * quantum_radius;
                            let quantum_y = center_y + quantum_angle.sin() * quantum_radius;
                            let quantum_size = 0.1 + (time * 2.0 + quantum as f64).sin() * 0.1;

                            ctx.draw(&Circle {
                                x: quantum_x, y: quantum_y, radius: quantum_size,
                                color: Color::White,
                                ..Default::default()
                            });
                        }

                        // Orbital rings
                        for orbit in 0..3 {
                            let orbit_radius = 10.0 + orbit as f64 * 3.0;
                            let orbit_alpha = (50 + (time * 20.0 + orbit as f64 * 30.0).sin() as u8).min(150);
                            ctx.draw(&Circle {
                                x: center_x, y: center_y, radius: orbit_radius,
                                color: Color::Rgb(orbit_alpha, orbit_alpha, orbit_alpha),
                                ..Default::default()
                            });
                        }
                    });

                file_icon_canvas.render(top_area, buf);

                // Center area: Main message
                let main_message = Paragraph::new(vec![
                    Line::from(""),
                    Line::from("📁 No File Selected").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Line::from(""),
                    Line::from("Select a file or media from the lists on the left").style(Style::default().fg(Color::Gray)),
                    Line::from("to preview its contents here.").style(Style::default().fg(Color::Gray)),
                    Line::from(""),
                    Line::from("💡 1st Tip: Press Enter to select the list you want,").style(Style::default().fg(Color::Cyan)),
                    Line::from("   use arrow keys to navigate and again").style(Style::default().fg(Color::Cyan)),
                    Line::from("   Enter to select a file.").style(Style::default().fg(Color::Cyan)),
                    Line::from(""),
                    Line::from("💡 2nd Tip: Press Upload to share your medias / files").style(Style::default().fg(Color::Cyan)),
                    Line::from("   with the network").style(Style::default().fg(Color::Cyan)),
                ])
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::NONE));
                main_message.render(center_area, buf);

                // Bottom area: Animated status indicators
                let [status_left, status_right] = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(bottom_area);

                // Right status: File count
                let file_count_text = format!("📄 {} files available",
                                              match app.simulation_controller_data.net_cache.known_file_list.get(selected_node) {
                                                  Some(server_files) => match server_files.get(&ndpage_data.selected_server) {
                                                      Some(files) => files.len(),
                                                      None => 0
                                                  },
                                                  None => 0
                                              }
                );
                let file_count_para = Paragraph::new(file_count_text)
                    .style(Style::default().fg(Color::Green))
                    .alignment(Alignment::Center);
                file_count_para.render(status_right, buf);

                // Left status: Media count
                let media_count_text = format!("🖼️ {} media files",
                                               match app.simulation_controller_data.net_cache.known_media_list.get(selected_node) {
                                                   Some(server_media) => match server_media.get(&ndpage_data.selected_server) {
                                                       Some(media) => media.len(),
                                                       None => 0
                                                   },
                                                   None => 0
                                               }
                );
                let media_count_para = Paragraph::new(media_count_text)
                    .style(Style::default().fg(Color::Magenta))
                    .alignment(Alignment::Center);
                media_count_para.render(status_left, buf);
            }
            SelectedFileMedia::File(file_id) => {
                render_file_visualizer(app,inner_file_visualizer_block,buf,ndpage_data,selected_node,file_id);
            }
            SelectedFileMedia::Media(media_id) => {
                render_media_visualizer(app,inner_file_visualizer_block, buf, ndpage_data, selected_node, media_id);
                // Rendering it externally
                return Some(inner_file_visualizer_block)
            }
        }
        None
    }
    fn render_loading_gauge(app: &App, area: Rect, buf: &mut Buffer, net_cache_media: Option<&NetCacheMedia>, net_cache_file: Option<&NetCacheFile>, server_id: ServerId) {
        // Split area vertically: [top, gauge, info, bottom]
        let [top_area, gauge_area, info_area, bottom_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Length(3), // Gauge height
                Constraint::Percentage(20),
                Constraint::Percentage(40),
            ])
            .areas(area);

        // Top: Animated spinner or bouncing dots
        let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let spinner = spinner_frames[(app.cache.counter as usize) % spinner_frames.len()];
        let spinner_line = Line::from(format!("{} Downloading from Server {:?}", spinner, server_id))
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        let spinner_para = Paragraph::new(spinner_line)
            .alignment(Alignment::Center);
        spinner_para.render(top_area, buf);

        // Gauge: Centered, smaller
        let (label, ratio) = match net_cache_media{
            None => {
                match net_cache_file{
                    None => ("0%".to_string(), 0.0),
                    Some(net) => {
                        match net {
                            NetCacheFile::Downloading(status) | NetCacheFile::Rendering(status) => {
                                (format!("{:.1}%", status), *status as f64 / 100.0)
                            }
                            _ => ("0%".to_string(), 0.0),
                        }
                    }
                }
            }
            Some(net) => {
                match net {
                    NetCacheMedia::Downloading(status) | NetCacheMedia::Rendering(status) => {
                        (format!("{:.1}%", status), *status as f64 / 100.0)
                    }
                    _ => ("0%".to_string(), 0.0),
                }
            }
        };

        let gauge_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Progress");
        Gauge::default()
            .block(gauge_block)
            .gauge_style(app.animation_color())
            .ratio(ratio)
            .label(label)
            .use_unicode(true)
            .render(gauge_area, buf);

        // Info: Show what is happening
        let info_lines = match net_cache_media {
            Some(media) => {
                match media {
                    NetCacheMedia::Downloading(_) => vec![
                        Line::from("Downloading media from server...").style(Style::default().fg(Color::Gray)),
                        Line::from("This may take a few seconds for large files.").style(Style::default().fg(Color::Gray)),
                    ],
                    NetCacheMedia::Rendering(_) => vec![
                        Line::from("Rendering image, please wait...").style(Style::default().fg(Color::Gray)),
                        Line::from("Rendering time depends on image size.").style(Style::default().fg(Color::Gray)),
                    ],
                    _ => vec![],
                }
            }
            None => {
                // It's a file, not media
                match net_cache_file {
                    Some(file) => {
                        match file {
                            NetCacheFile::Downloading(_) => vec![
                                Line::from("Downloading file from server...").style(Style::default().fg(Color::Gray)),
                                Line::from("This may take a few seconds for large files.").style(Style::default().fg(Color::Gray)),
                            ],
                            NetCacheFile::Rendering(_) => vec![
                                Line::from("Processing file, please wait...").style(Style::default().fg(Color::Gray)),
                                Line::from("Processing time depends on file size.").style(Style::default().fg(Color::Gray)),
                            ],
                            _ => vec![],
                        }
                    }
                    None => vec![],
                }
            }
        };

        // Wrap the info lines to fit the available width
        let mut wrapped_info_lines = Vec::new();
        for line in info_lines {
            let text = line.to_string();
            let wrapped_text = wrap(&text, info_area.width.saturating_sub(2) as usize);
            for wrapped_line in wrapped_text {
                wrapped_info_lines.push(Line::from(wrapped_line.to_string()).style(Style::default().fg(Color::Gray)));
            }
        }

        let info_para = Paragraph::new(wrapped_info_lines)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: true });
        info_para.render(info_area, buf);

        // Bottom: Fun tip or status
        let tips = [
            "💡 Tip: You can continue browsing while downloading!",
            "💡 Tip: Large images may take longer to render.",
            "⁉️ Question: What came first, the chicken or the egg?",
            "💡 Tip: Try uploading smaller images for faster preview.",
            "🐛 Fun Fact: Bugs travel faster in production",
            "💡 Tip: The spinner is animated!",
            "🎭 Wisdom: There are 10 kinds of people: those who understand binary and those who don't."
        ];
        // Make the tip change 4-5x slower
        let tip = tips[(app.cache.counter as usize / 25) % tips.len()];

        // Wrap the tip text to fit the available width
        let wrapped_tip = wrap(&tip, bottom_area.width.saturating_sub(2) as usize);
        let tip_lines: Vec<Line> = wrapped_tip.iter()
            .map(|line| Line::from(line.to_string()).style(Style::default().fg(Color::Cyan)))
            .collect();

        let tip_para = Paragraph::new(tip_lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        tip_para.render(bottom_area, buf);
    }

    fn render_media_visualizer(app:&App, area:Rect, buf:&mut Buffer, ndpage_data: &NetSecondPage, selected_node: &ClientId, file_id:MediaId){
        match app.simulation_controller_data.net_cache.downloaded_medias.get(&file_id){
            None => {
                error!(target: "main", "[LEDRON-CLI] Couldn't find media id in new_downloaded_medias while rendering")
            }
            Some(media_entry) => {
                match &media_entry.cache{
                    NetCacheMedia::Downloading(_) |  NetCacheMedia::Rendering(_) => {
                        // Rendering Bar status widget
                        render_loading_gauge(app,area,buf,Some(&media_entry.cache),None, ndpage_data.selected_server)
                    }
                    NetCacheMedia::Downloaded(vec) => {
                        render_media_waiting_for_rendering(app, area, buf);
                    }
                    NetCacheMedia::Rendered(image_protocol) => {
                        // I render it externally ddso no need to do it again here,
                        // sopratutto perchè serve un riferimento mutabile a app per farlo :)))
                    }
                    NetCacheMedia::FatalErrorRendering() => {
                        render_fatal_rendering_error(app, area, buf);
                    }
                }
            }
        }
    }


    fn render_file_visualizer(app:&App, area:Rect, buf:&mut Buffer, ndpage_data: &NetSecondPage, selected_node: &ClientId, file_id:FileId){
        match app.simulation_controller_data.net_cache.downloaded_files.get(file_id.as_str()) {
            None => {}
            Some(item) => {
                match item.value() {
                            NetCacheFile::Downloading(x) => {
                                render_loading_gauge(app,area,buf,None, Some(item.value()), ndpage_data.selected_server)
                            }
                            NetCacheFile::Downloaded(_) | NetCacheFile::Rendering(_)=> {
                                render_file_waiting_for_rendering(app,area,buf);
                            }
                            NetCacheFile::Rendered(file) => {
                                match file {
                                    FileType::UTF8(lines) => {
                                        render_file_utf8(app, area, buf, lines, ndpage_data);
                                    }
                                    FileType::Other(file) => {
                                        render_file_not_viewable(app,area,buf);
                                    }
                                }
                            }
                        }
            }
        }
    }
    fn render_file_not_viewable(app:&App,area:Rect, buf: &mut Buffer) {
        let [top_area, center_area, bottom_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
            ])
            .areas(area);

        // Top: Animated file icon with "not viewable" styling
        let file_icon_canvas = Canvas::default()
            .background_color(POPUP_BG_COLOR)
            .x_bounds([0.0, 40.0])
            .y_bounds([0.0, 40.0])
            .paint(|ctx| {
                let time = app.cache.counter as f64 * 0.15;

                // Warning/error color scheme (red/orange tones)
                let warning_color = Color::Rgb(255, 100, 100);
                let error_color = Color::Rgb(200, 50, 50);
                let accent_color = Color::Rgb(255, 150, 50);

                // Pulsing warning ring
                let ring_radius = 20.0 + (time * 0.8).sin() * 3.0;
                let ring_alpha = (100 + (time * 50.0).sin() as u8).min(200);
                ctx.draw(&Circle {
                    x: 20.0, y: 20.0, radius: ring_radius,
                    color: Color::Rgb(ring_alpha, ring_alpha / 3, ring_alpha / 3),
                    ..Default::default()
                });

                // Main file icon with warning styling
                let scale = 0.8 + (time * 0.3).sin() * 0.1;
                let center_x = 20.0;
                let center_y = 20.0;

                // File body with warning colors
                let points = [
                    (8.0, 5.0), (32.0, 5.0), (32.0, 35.0), (8.0, 35.0)
                ];

                for i in 0..4 {
                    let (x1, y1) = points[i];
                    let (x2, y2) = points[(i + 1) % 4];

                    // Apply scaling
                    let scale_x1 = center_x + (x1 - center_x) * scale;
                    let scale_y1 = center_y + (y1 - center_y) * scale;
                    let scale_x2 = center_x + (x2 - center_x) * scale;
                    let scale_y2 = center_y + (y2 - center_y) * scale;

                    // Warning color cycling
                    let color_intensity = (150 + (time * 30.0 + i as f64 * 20.0).sin() as u8).min(255);
                    ctx.draw(&CanvasLine {
                        x1: scale_x1, y1: scale_y1, x2: scale_x2, y2: scale_y2,
                        color: Color::Rgb(color_intensity, color_intensity / 3, color_intensity / 3),
                        ..Default::default()
                    });
                }

                // Warning triangle overlay
                let triangle_points = [
                    (20.0, 8.0), (15.0, 18.0), (25.0, 18.0)
                ];
                for i in 0..3 {
                    let (x1, y1) = triangle_points[i];
                    let (x2, y2) = triangle_points[(i + 1) % 3];

                    let scale_x1 = center_x + (x1 - center_x) * scale;
                    let scale_y1 = center_y + (y1 - center_y) * scale;
                    let scale_x2 = center_x + (x2 - center_x) * scale;
                    let scale_y2 = center_y + (y2 - center_y) * scale;

                    ctx.draw(&CanvasLine {
                        x1: scale_x1, y1: scale_y1, x2: scale_x2, y2: scale_y2,
                        color: warning_color,
                        ..Default::default()
                    });
                }

                // Exclamation mark in triangle
                let exclamation_y = 14.0 + (time * 0.5).sin() * 0.5;
                let scale_exclamation_y = center_y + (exclamation_y - center_y) * scale;
                ctx.draw(&CanvasLine {
                    x1: 20.0, y1: scale_exclamation_y, x2: 20.0, y2: scale_exclamation_y + 2.0,
                    color: Color::White,
                    ..Default::default()
                });
                ctx.draw(&Circle {
                    x: 20.0, y: scale_exclamation_y + 3.5, radius: 0.3,
                    color: Color::White,
                    ..Default::default()
                });

                // Animated "X" marks around the file
                for i in 0..4 {
                    let angle = time * 0.5 + i as f64 * 1.57; // 90 degrees apart
                    let radius = 25.0 + (time * 0.3).sin() * 2.0;
                    let x = center_x + angle.cos() * radius;
                    let y = center_y + angle.sin() * radius;
                    let size = 0.8 + (time + i as f64).sin() * 0.3;

                    // Draw X mark
                    ctx.draw(&CanvasLine {
                        x1: x - size, y1: y - size, x2: x + size, y2: y + size,
                        color: error_color,
                        ..Default::default()
                    });
                    ctx.draw(&CanvasLine {
                        x1: x + size, y1: y - size, x2: x - size, y2: y + size,
                        color: error_color,
                        ..Default::default()
                    });
                }

                // Sparkle effects with warning colors
                for sparkle in 0..6 {
                    let sparkle_angle = time * 0.7 + sparkle as f64 * 1.047; // 60 degrees apart
                    let sparkle_radius = 15.0 + (time * 0.3).sin() * 2.0;
                    let sparkle_x = center_x + sparkle_angle.cos() * sparkle_radius;
                    let sparkle_y = center_y + sparkle_angle.sin() * sparkle_radius;
                    let sparkle_size = 0.3 + (time + sparkle as f64).sin() * 0.2;

                    let sparkle_hue = (time * 40.0 + sparkle as f64 * 60.0) % 60.0; // Red to orange range
                    let (r, g, b) = hsv_to_rgb(sparkle_hue, 0.9, 1.0);

                    ctx.draw(&Circle {
                        x: sparkle_x, y: sparkle_y, radius: sparkle_size,
                        color: Color::Rgb(r, g, b),
                        ..Default::default()
                    });
                }
            });

        file_icon_canvas.render(top_area, buf);

        // Center: Main message with wrapped text
        let main_message_lines = vec![
            Line::from("⚠️  File Not Viewable in Terminal").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::from("This file type cannot be displayed").style(Style::default().fg(Color::Gray)),
            Line::from("directly in the terminal interface.").style(Style::default().fg(Color::Gray)),
            Line::from(""),
            Line::from("💡 To view this file:").style(Style::default().fg(Color::Cyan)),
            Line::from("   1. Select the file in the list").style(Style::default().fg(Color::Cyan)),
            Line::from("   2. Press Enter to open it").style(Style::default().fg(Color::Cyan)),
            Line::from("   3. It will open in your default app").style(Style::default().fg(Color::Cyan)),
            Line::from(""),
            Line::from("📁 Supported terminal formats:").style(Style::default().fg(Color::Green)),
            Line::from("   • Text files (.txt, .md, .log)").style(Style::default().fg(Color::Green)),
            Line::from("   • Images (.jpg, .png, .gif)").style(Style::default().fg(Color::Green)),
        ];

        let main_para = Paragraph::new(main_message_lines)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: true });
        main_para.render(center_area, buf);

        // Bottom: Action prompt
        let [action_left, action_right] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .areas(bottom_area);

        // Left: File type info
        let file_type_text = "📄 Binary/Executable File";
        let file_type_para = Paragraph::new(file_type_text)
            .style(Style::default().fg(Color::Magenta))
            .alignment(Alignment::Center);
        file_type_para.render(action_left, buf);

        // Right: Action button hint
        let action_text = "🖱️  Press Enter to Open";
        let action_para = Paragraph::new(action_text)
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center);
        action_para.render(action_right, buf);
    }
    fn render_file_utf8(app: &App, area: Rect, buf: &mut Buffer, lines: &Vec<String>, ndpage_data: &NetSecondPage) {
        use tui_scrollview::{ScrollView, ScrollViewState};
        use ratatui::widgets::{Table, Row, Cell, Paragraph, Block, Borders, BorderType, Wrap};
        use ratatui::style::{Style, Color, Modifier};
        use ratatui::layout::{Constraint, Rect, Alignment};
        use textwrap::wrap;

        // --- Block Parsing ---
        #[derive(Debug)]
        enum BlockType {
            Table(Vec<Vec<String>>),
            Text(Vec<String>),
        }

        // Helper: is a line a table line?
        fn is_table_line(line: &str) -> bool {
            let seps = [',', '|', ';', '\t'];
            seps.iter().any(|&sep| line.contains(sep))
        }

        // Parse lines into blocks (table or text)
        let mut blocks = Vec::new();
        let mut current_block: Option<BlockType> = None;
        let mut push_block = |block: Option<BlockType>| {
            if let Some(b) = block {
                match &b {
                    BlockType::Table(rows) if rows.is_empty() => {},
                    BlockType::Text(lines) if lines.is_empty() => {},
                    _ => blocks.push(b),
                }
            }
        };
        for line in lines {
            let trimmed = line.trim_end();
            if trimmed.is_empty() {
                // Empty line: always a block break
                push_block(current_block.take());
                continue;
            }
            let is_table = is_table_line(trimmed);
            match &mut current_block {
                Some(BlockType::Table(rows)) if is_table => {
                    // Parse row
                    let row = if trimmed.contains('|') {
                        trimmed.split('|').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
                    } else if trimmed.contains('\t') {
                        trimmed.split('\t').map(|s| s.trim().to_string()).collect()
                    } else if trimmed.contains(',') {
                        trimmed.split(',').map(|s| s.trim().to_string()).collect()
                    } else if trimmed.contains(';') {
                        trimmed.split(';').map(|s| s.trim().to_string()).collect()
                    } else {
                        vec![trimmed.to_string()]
                    };
                    rows.push(row);
                }
                Some(BlockType::Text(lines)) if !is_table => {
                    lines.push(trimmed.to_string());
                }
                _ => {
                    // Block type switch
                    push_block(current_block.take());
                    if is_table {
                        let row = if trimmed.contains('|') {
                            trimmed.split('|').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
                        } else if trimmed.contains('\t') {
                            trimmed.split('\t').map(|s| s.trim().to_string()).collect()
                        } else if trimmed.contains(',') {
                            trimmed.split(',').map(|s| s.trim().to_string()).collect()
                        } else if trimmed.contains(';') {
                            trimmed.split(';').map(|s| s.trim().to_string()).collect()
                        } else {
                            vec![trimmed.to_string()]
                        };
                        current_block = Some(BlockType::Table(vec![row]));
                    } else {
                        current_block = Some(BlockType::Text(vec![trimmed.to_string()]));
                    }
                }
            }
        }
        push_block(current_block.take());

        // --- Rendering ---
        // Account for border width (2 characters on each side)
        let available_width = area.width.saturating_sub(4); // 2 for left border, 2 for right border
        let view_height = area.height.saturating_sub(2); // Account for potential borders
        let mut total_height = 0u16;
        
        // Precompute all block heights and ensure content fits width
        for block in &blocks {
            let h = match block {
                BlockType::Table(rows) => rows.len() as u16 + 2, // header + border
                BlockType::Text(lines) => {
                    let mut h = 0u16;
                    for l in lines {
                        let wrapped = wrap(l, available_width as usize);
                        h += wrapped.len() as u16;
                    }
                    h
                }
            };
            total_height += h;
        }

        // Create scroll view with constrained width to prevent horizontal scrolling
        let mut scroll_view = ScrollView::new(ratatui::layout::Size::new(available_width, total_height));
        let scroll_buf = scroll_view.buf_mut();
        
        // Fill background
        for y in 0..scroll_buf.area.height {
            for x in 0..scroll_buf.area.width {
                scroll_buf.get_mut(x, y).set_style(Style::default().bg(POPUP_BG_COLOR));
            }
        }

        // Render blocks into scroll buffer
        let mut y = 0u16;
        for block in &blocks {
            match block {
                BlockType::Table(rows) => {
                    // Compute max columns and ensure they fit
                    let max_cols = rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1);
                    let col_width = 100 / max_cols;
                    let constraints: Vec<_> = (0..max_cols).map(|_| Constraint::Percentage(col_width as u16)).collect();
                    
                    // Truncate cell content to fit available width
                    let table_rows: Vec<Row> = rows.iter().map(|row| {
                        let cells: Vec<Cell> = row.iter().map(|cell| {
                            let truncated = if cell.len() > available_width as usize / max_cols {
                                format!("{}...", &cell[..(available_width as usize / max_cols - 3)])
                            } else {
                                cell.clone()
                            };
                            Cell::from(truncated).style(Style::default().fg(Color::White))
                        }).collect();
                        Row::new(cells)
                    }).collect();
                    
                    let table = Table::new(table_rows, &constraints)
                        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded))
                        .style(Style::default().fg(Color::White));
                    let table_area = Rect::new(0, y, available_width, rows.len() as u16 + 2);
                    table.render(table_area, scroll_buf);
                    y += rows.len() as u16 + 2;
                }
                BlockType::Text(lines) => {
                    for l in lines {
                        let wrapped = wrap(l, available_width as usize);
                        for w in wrapped {
                            let para = Paragraph::new(w.to_string())
                                .style(Style::default().fg(Color::White))
                                .wrap(Wrap { trim: true });
                            let line_area = Rect::new(0, y, available_width, 1);
                            para.render(line_area, scroll_buf);
                            y += 1;
                        }
                    }
                }
            }
        }

        // Scroll state: use ndpage_data.scroll_offset
        let mut scroll_state = ScrollViewState::default();
        
        // Set scroll position based on offset
        // scroll_offset = 0 means at the top, increasing means scrolled down
        for _ in 0..ndpage_data.scroll_offset {
            scroll_state.scroll_down();
        }
        
        ratatui::widgets::StatefulWidget::render(scroll_view, area, buf, &mut scroll_state);
    }

    
    fn render_media_waiting_for_rendering(app: &App, area: Rect, buf: &mut Buffer) {
        let [top_area, center_area, bottom_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
            ])
            .areas(area);

        // Top: Animated spinner
        let spinner_chars = ['|', '/', '-', '\\'];
        let spinner = spinner_chars[(app.cache.counter as usize) % spinner_chars.len()];
        let spinner_line = Line::from(format!("  {}  Waiting for rendering...", spinner))
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        let spinner_para = Paragraph::new(spinner_line)
            .alignment(Alignment::Center);
        spinner_para.render(top_area, buf);

        // Center: Info
        let info_lines = vec![
            Line::from("Image downloaded, waiting for renderer thread...").style(Style::default().fg(Color::Gray)),
            Line::from("This may take a few seconds for large images.").style(Style::default().fg(Color::Gray)),
            Line::from(""),
            Line::from("💡 Tip: Try smaller images for faster preview.").style(Style::default().fg(Color::Cyan)),
        ];
        let info_para = Paragraph::new(info_lines)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::NONE));
        info_para.render(center_area, buf);

        // Bottom: Status
        let status_line = Line::from("Renderer is busy. Please wait...")
            .style(Style::default().fg(Color::Magenta));
        let status_para = Paragraph::new(status_line)
            .alignment(Alignment::Center);
        status_para.render(bottom_area, buf);
    }
    fn render_file_waiting_for_rendering(app: &App, area: Rect, buf: &mut Buffer) {
        let [top_area, center_area, bottom_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
            ])
            .areas(area);

        // Top: Animated spinner
        let spinner_chars = ['|', '/', '-', '\\'];
        let spinner = spinner_chars[(app.cache.counter as usize) % spinner_chars.len()];
        let spinner_line = Line::from(format!("  {}  Waiting for file rendering...", spinner))
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        let spinner_para = Paragraph::new(spinner_line)
            .alignment(Alignment::Center);
        spinner_para.render(top_area, buf);

        // Center: Info
        let info_lines = vec![
            Line::from("File downloaded, waiting for renderer thread...").style(Style::default().fg(Color::Gray)),
            Line::from("This may take a few seconds for large files.").style(Style::default().fg(Color::Gray)),
            Line::from(""),
            Line::from("💡 Tip: Try smaller files for faster preview.").style(Style::default().fg(Color::Cyan)),
        ];
        let info_para = Paragraph::new(info_lines)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::NONE));
        info_para.render(center_area, buf);

        // Bottom: Status
        let status_line = Line::from("File renderer is busy. Please wait...")
            .style(Style::default().fg(Color::Magenta));
        let status_para = Paragraph::new(status_line)
            .alignment(Alignment::Center);
        status_para.render(bottom_area, buf);
    }
    fn render_fatal_rendering_error(app: &App, area: Rect, buf: &mut Buffer) {
        let [top_area, center_area, bottom_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
            ])
            .areas(area);

        // Top: Error icon and title
        let error_line = Line::from("❌ Fatal Rendering Error")
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));
        let error_para = Paragraph::new(error_line)
            .alignment(Alignment::Center);
        error_para.render(top_area, buf);

        // Center: Error details and instructions
        let info_lines = vec![
            Line::from("The image renderer encountered a fatal error.").style(Style::default().fg(Color::Red)),
            Line::from("This usually happens when the terminal cannot").style(Style::default().fg(Color::Gray)),
            Line::from("properly handle the image format or size.").style(Style::default().fg(Color::Gray)),
            Line::from(""),
            Line::from("💡 Try these solutions:").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Line::from("• Use a smaller image file").style(Style::default().fg(Color::Cyan)),
            Line::from("• Try a different image format (PNG/JPG)").style(Style::default().fg(Color::Cyan)),
            Line::from("• Restart the application").style(Style::default().fg(Color::Cyan)),
            Line::from("• Check terminal compatibility").style(Style::default().fg(Color::Cyan)),
        ];
        let info_para = Paragraph::new(info_lines)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::NONE));
        info_para.render(center_area, buf);

        // Bottom: Retry suggestion
        let retry_line = Line::from("🔄 The image will be retried automatically on next access")
            .style(Style::default().fg(Color::Magenta));
        let retry_para = Paragraph::new(retry_line)
            .alignment(Alignment::Center);
        retry_para.render(bottom_area, buf);
    }
}
pub(super) mod settings_rendering{
    use ratatui::buffer::Buffer;
    use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::Line;
    use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Widget};
    use wg_2024::network::NodeId;
    use crate::frontend::app::App;
    use crate::frontend::app::models::general_types::Node;
    use crate::frontend::app::models::tab1_types::{ClientSettingsCommandsPopup, ClientSettingsPopup, DroneSettingsPopup};
    use crate::frontend::app::ui::tabs::MAIN_BLOCK_BORDERS;
    use crate::frontend::app::ui::tabs::tab1::{draw_map, render_add_sender_popup, render_remove_sender_popup};

    pub(super) fn render_settings_page(app:&App, area:Rect, buf: &mut Buffer, id:NodeId, set_page:&ClientSettingsPopup) {
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

        render_command_popup(app,area,buf,&set_page.command_popup);
    }
    fn render_command_popup(app: &App, area: Rect, buf: &mut Buffer, popup: &ClientSettingsCommandsPopup) {
        match popup {
            ClientSettingsCommandsPopup::AddSender(cmd) => render_add_sender_popup(app, area, buf, cmd),
            ClientSettingsCommandsPopup::RemoveSender(cmd) => render_remove_sender_popup(app, area, buf, cmd),
            ClientSettingsCommandsPopup::None => {},
        }
    }

}

// Helper function to convert HSV to RGB, I'm tired of converting palettes manually to rgb manually.
fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (((r + m) * 255.0) as u8, ((g + m) * 255.0) as u8, ((b + m) * 255.0) as u8)
}