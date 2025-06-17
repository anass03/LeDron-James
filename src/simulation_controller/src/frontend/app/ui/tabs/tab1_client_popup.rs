use std::cmp::PartialEq;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::buffer::Buffer;
use wg_2024::network::NodeId;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Widget};
use ratatui::widgets::canvas::{Canvas, Circle, Line as CanvasLine, Map};
use std::ops::Add;
use ratatui::style::{Color, Modifier, Style, Styled, Stylize};
use ratatui::text::ToSpan;
use crate::backend::simulation_controller::NetNode;
use crate::frontend::app::App;
use crate::frontend::app::models::tab1_types::{ClientPopup, PopupSelection};
use crate::frontend::app::ui::tabs::POPUP_BG_COLOR;
use crate::frontend::app::ui::tabs::tab1::{get_data_canvas, get_settings_canvas};
use crate::frontend::app::ui::tabs::tab1_client_popup::chat_rendering::render_chat_options;
use crate::frontend::app::ui::tabs::tab1_client_popup::net_rendering::render_net_options;

pub(in crate::frontend::app::ui::tabs) fn render_client_popup(app:&App, area: Rect, buf : &mut Buffer, client_popup: &ClientPopup, id:NodeId){
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
    render_client_options(app, opt_inner, buf, client_popup,&id);
}

fn render_client_options(app:&App, area:Rect, buf: &mut Buffer, client_popup: &ClientPopup, selected_node: &NodeId) {
    // Now here based on the information we have in the struct we draw the menu's
    match client_popup{
        ClientPopup::ChatPage(chat_page_data) => {
            render_chat_options(app,area, buf, chat_page_data, selected_node);
        }
        ClientPopup::InternetPage(net_page_data) => {
            render_net_options(app, area, buf, net_page_data, selected_node);
        }
        ClientPopup::DataPage(data_page_data) => {

        }
        ClientPopup::SettingsPage(_) => {

        }
    }

}
pub(super) mod chat_rendering{
    use std::cmp::PartialEq;
    use wg_2024::network::NodeId;
    use crate::frontend::app::App;
    use crate::frontend::app::models::tab1_types::{ChatThirdPageSelection, ClientChatPopup};
    use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
    use ratatui::buffer::{Buffer, Cell};

    use ratatui::prelude::{Color, Line, Modifier, Style, Text, Widget};
    use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};
    use toml::value::Time;
    use textwrap::wrap;
    use ratatui::layout::Direction::Vertical;
    use ratatui::widgets::canvas::{Canvas, Circle, Context, Line as CanvasLine};
    use std::ops::Add;
    use ratatui::style::{Styled, Stylize};
    use ratatui::text::ToSpan;
    use crate::backend::sc_models::{Chat, Message};
    use crate::frontend::app::models::main_types::AppMode;
    use crate::frontend::app::models::tab1_types::{ChSecondPage, ChThirdPage};
    use crate::frontend::app::ui::tabs::tab1::centered_popup_area;

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
                    app.cache.file_explorer.widget().render(centered_popup_area(area), buf);
                }
            }
        }
    }

    // First Menù rendering ( Choose the server )
    fn render_chat_first_menu(app: &App, area: Rect, buf: &mut Buffer, selected_cl:NodeId) {

        match app.simulation_controller_data.chat_cache.known_chat_servers.get(&selected_cl){
            None => {
                render_no_chatservers_available(app, area, buf);
            }
            Some(known_servers) => {
                render_chatservers_available(app,area,buf,known_servers);
            }
        }
    }

    fn render_chatservers_available(app:&App, area:Rect, buf:&mut Buffer, known_servers: &Vec<NodeId>){
        let [_,first_second, last] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1),Constraint::Percentage(100), Constraint::Min(1)])
            /*.horizontal_margin(1)*/
            .areas(area);
        let [first,second] = Layout::default()
            .direction(Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .horizontal_margin(1)
            .areas(first_second);

        let mut first_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);
        if app.cache.tab1_data.popup_cursor % 2 == 0{
            first_block = first_block.border_style(Style::default().fg(app.animation_color()));
        }

        let [first_canvas_area, first_paragraph_area] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            /*.vertical_margin(1)*/
            .areas(first_block.inner(first));
        let first_canvas = draw_server_canvas();


        let vec_len = known_servers.len();
        // Now depending on how many server do we have we scroll the tabs.
        if usize::from(app.cache.tab1_data.popup_cursor)>=vec_len{
            // We're overflowing the Vec
            // Should be unreachable as increasing / decreasing cursor is pretty firm.
        }else{
            let tab_num = app.cache.tab1_data.popup_cursor / 2;
            let total_tabs_num = (known_servers.len()-1)/2;

            let [left_last,right_last] = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(last);
            let page = Line::from(format!("{}/{}", tab_num, total_tabs_num))
                .style(Style::default().add_modifier(Modifier::BOLD | Modifier::ITALIC))
                .alignment(Alignment::Right);
            page.render(right_last, buf);

            let info = Line::from("Press 'r' to find new server chats in the network!")
                .style(Style::default().add_modifier(Modifier::BOLD | Modifier::ITALIC))
                .alignment(Alignment::Left);
            info.render(left_last, buf);

            let first_server_info = Paragraph::new(format!("Server {:?}", known_servers.get(tab_num*2).unwrap_or(&255)))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            first_block.render(first,buf);
            first_canvas.render(first_canvas_area, buf);
            first_server_info.render(first_paragraph_area, buf);


            // Siamo in una lista di lunghezza dispari!
            if (vec_len % 2 == 0 && app.cache.tab1_data.popup_cursor.add(1) as usize == vec_len) || app.cache.tab1_data.popup_cursor as usize +1<vec_len{
                let mut second_block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded);
                if app.cache.tab1_data.popup_cursor % 2 == 1{
                    second_block = second_block.border_style(Style::default().fg(app.animation_color()));
                }
                let [second_canvas_area, second_paragraph_area] = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                    /*.vertical_margin(1)*/
                    .areas(second_block.inner(second));
                let second_canvas = draw_server_canvas();


                let second_server_info = Paragraph::new(format!("Server {:?}", known_servers.get(tab_num*2 + 1 ).unwrap_or(&255)))
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true });
                second_block.render(second,buf);
                second_canvas.render(second_canvas_area, buf);
                second_server_info.render(second_paragraph_area, buf);
            }
        }
    }

    fn render_no_chatservers_available(_app: &App, area: Rect, buf: &mut Buffer) {
        let text = Text::from(vec![
            Line::from("No Chat Server FOUND")
                .style(Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC | Modifier::BOLD)),
            Line::from("Press 'r' to send network request to find new chat-servers")
                .style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
        ]);

        Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }

    fn draw_server_canvas<'a>() -> Canvas<'a, fn(&mut Context)> {
        Canvas::default()
            .block(Block::default().borders(Borders::ALL).title("Chat Server"))
            .x_bounds([0.0, 20.0])
            .y_bounds([0.0, 20.0])
            .paint(|ctx| {
                // Front rectangle (base)
                ctx.draw(&CanvasLine { x1: 5.0, y1: 5.0, x2: 15.0, y2: 5.0, color: Color::White, ..Default::default() }); // Bottom edge
                ctx.draw(&CanvasLine { x1: 15.0, y1: 5.0, x2: 15.0, y2: 15.0, color: Color::White, ..Default::default() }); // Right edge
                ctx.draw(&CanvasLine { x1: 15.0, y1: 15.0, x2: 5.0, y2: 15.0, color: Color::White, ..Default::default() }); // Top edge
                ctx.draw(&CanvasLine { x1: 5.0, y1: 15.0, x2: 5.0, y2: 5.0, color: Color::White, ..Default::default() }); // Left edge

                // Back rectangle offset
                let dx = 3.0;
                let dy = -3.0;

                // Back rectangle (offset copy)
                ctx.draw(&CanvasLine { x1: 5.0 + dx, y1: 5.0 + dy, x2: 15.0 + dx, y2: 5.0 + dy, color: Color::Gray, ..Default::default() });
                ctx.draw(&CanvasLine { x1: 15.0 + dx, y1: 5.0 + dy, x2: 15.0 + dx, y2: 15.0 + dy, color: Color::Gray, ..Default::default() });
                ctx.draw(&CanvasLine { x1: 15.0 + dx, y1: 15.0 + dy, x2: 5.0 + dx, y2: 15.0 + dy, color: Color::Gray, ..Default::default() });
                ctx.draw(&CanvasLine { x1: 5.0 + dx, y1: 15.0 + dy, x2: 5.0 + dx, y2: 5.0 + dy, color: Color::Gray, ..Default::default() });

                // Connecting edges
                ctx.draw(&CanvasLine { x1: 5.0, y1: 5.0, x2: 5.0 + dx, y2: 5.0 + dy, color: Color::White, ..Default::default() });
                ctx.draw(&CanvasLine { x1: 15.0, y1: 5.0, x2: 15.0 + dx, y2: 5.0 + dy, color: Color::White, ..Default::default() });
                ctx.draw(&CanvasLine { x1: 15.0, y1: 15.0, x2: 15.0 + dx, y2: 15.0 + dy, color: Color::White, ..Default::default() });
                ctx.draw(&CanvasLine { x1: 5.0, y1: 15.0, x2: 5.0 + dx, y2: 15.0 + dy, color: Color::White, ..Default::default() });
            })
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
                        render_contacts_available(app,area,buf,sec_page_data,selected_cl, contacts);
                    }
                }
            }
        }
    }

    fn render_contacts_available(app:&App, area: Rect, buf: &mut Buffer, sec_page_data:&ChSecondPage, selected_cl:NodeId, contacts:&Vec<NodeId>){
        let [others, last] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(100), Constraint::Min(1)])
            /*.vertical_margin(1)*/
            .areas(area);

        let info = Line::from(" Press 'r' to find new server chats in the network! ")
            .style(Style::default().add_modifier(Modifier::ITALIC))
            .alignment(Alignment::Left);
        info.render(last, buf);

        let [first,second,third,fourth ] = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [Constraint::Length(others.height / 4); 4]
            )
            .vertical_margin(1)
            .areas(others);

        let selected_index = app.cache.tab1_data.popup_cursor;
        let offset = selected_index.saturating_sub(3);
        let visible_contacts = &contacts.iter().skip(offset).take(4).collect::<Vec<_>>();

        'outer: for (i, contact_id) in visible_contacts.iter().enumerate() {
            let is_selected = selected_index == offset + i;
            let block = match i {
                0 => first,
                1 => second,
                2 => third,
                3 => fourth,
                _ => continue,
            };

            let contact_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(if is_selected {Style::default().fg(app.animation_color())} else {Style::default()});
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

            let canvas = draw_profile_photo();
            canvas.render(canvas_area, buf);

            let id_text = Paragraph::new(format!("Client ID {}", contact_id))
                .alignment(Alignment::Left)
                .style(if is_selected { Style::default().add_modifier(Modifier::BOLD) } else { Style::default() });
            id_text.render(info_main, buf);

            match app.simulation_controller_data.chat_cache.local_chats.get(&sec_page_data.selected_server){
                None => {
                    // Server has no messages stored
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
                            // No messages are sent
                            let last_msg = Paragraph::new(Text::styled("Send the first message!", Style::default().add_modifier(Modifier::ITALIC).fg(Color::Gray)))
                                .alignment(Alignment::Left)
                                .wrap(Wrap { trim: true });
                            last_msg.render(info_msg, buf);
                        }
                        Some(ch) => {
                            match ch.get_last_message(){
                                None => {
                                    // No messages are sent
                                    let last_msg = Paragraph::new(Text::styled("Send the first message!", Style::default().add_modifier(Modifier::ITALIC).fg(Color::Gray)))
                                        .alignment(Alignment::Left)
                                        .wrap(Wrap { trim: true });
                                    last_msg.render(info_msg, buf);
                                }
                                Some((mess,messdata)) => {
                                    /*let [message_area, data_area] = Layout::default()
                                        .direction(Direction::Horizontal)
                                        .constraints([Constraint::Percentage(75), Constraint::Percentage(25)])*/
                                    if messdata.id_sender==selected_cl {
                                        match mess{
                                            Message::Message(str) => {
                                                let initial_mess = format!("{:?}:{:?} - YOU: ",messdata.timestamp.time.unwrap_or(Time{ hour: 0, minute: 0, second: 0, nanosecond: 0, }).hour,
                                                                           messdata.timestamp.time.unwrap_or(Time{ hour: 0, minute: 0, second: 0, nanosecond: 0, }).minute);
                                                let last_msg = Paragraph::new(Line::from(vec![initial_mess
                                                                                                  .set_style(Style::default().add_modifier(Modifier::ITALIC).fg(Color::Gray)),
                                                                                              str.to_span()]))
                                                    .alignment(Alignment::Left)
                                                    .wrap(Wrap { trim: true });
                                                last_msg.render(info_msg, buf);
                                            }
                                            Message::Media(med) => {
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
                                            Message::Message(str) => {
                                                let initial_mess = format!("{:?}:{:?} - ID {:?}: ",messdata.timestamp.time.unwrap_or(Time{ hour: 0, minute: 0, second: 0, nanosecond: 0, }).hour,
                                                                           messdata.timestamp.time.unwrap_or(Time{ hour: 0, minute: 0, second: 0, nanosecond: 0, }).minute,
                                                                           messdata.id_sender);
                                                let last_msg = Paragraph::new(Line::from(vec![initial_mess.set_style(Style::default().add_modifier(Modifier::ITALIC).fg(Color::Gray)),
                                                                                              str.to_span()]))
                                                    .alignment(Alignment::Left)
                                                    .wrap(Wrap { trim: true });
                                                last_msg.render(info_msg, buf);
                                            }
                                            Message::Media(med) => {
                                                let initial_mess = format!("{:?}:{:?} - ID {:?} sent a media 🖇",messdata.timestamp.time.unwrap_or(Time{ hour: 0, minute: 0, second: 0, nanosecond: 0, }).hour,
                                                                           messdata.timestamp.time.unwrap_or(Time{ hour: 0, minute: 0, second: 0, nanosecond: 0, }).minute
                                                                           ,messdata.id_sender);
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

                    /*'inner: for ((us1,us2),ch) in &chat_list.chats_stored_temp {
                        if (*us1 == selected_cl && *us2 == **contact_id) || (*us1 == **contact_id && *us2 == selected_cl){
                            match ch.get_last_message(){
                                None => {
                                    // No messages are sent
                                    let last_msg = Paragraph::new(Text::styled("Send the first message!", Style::default().add_modifier(Modifier::ITALIC).fg(Color::Gray)))
                                        .alignment(Alignment::Left)
                                        .wrap(Wrap { trim: true });
                                    last_msg.render(info_msg, buf);
                                }
                                Some((mess,messdata)) => {
                                    /*let [message_area, data_area] = Layout::default()
                                        .direction(Direction::Horizontal)
                                        .constraints([Constraint::Percentage(75), Constraint::Percentage(25)])*/
                                    if messdata.id_sender==selected_cl {
                                        match mess{
                                            Message::Message(str) => {
                                                let initial_mess = format!("{:?}:{:?} - YOU: ",messdata.timestamp.time.unwrap_or(Time{ hour: 0, minute: 0, second: 0, nanosecond: 0, }).hour,
                                                                           messdata.timestamp.time.unwrap_or(Time{ hour: 0, minute: 0, second: 0, nanosecond: 0, }).minute);
                                                let last_msg = Paragraph::new(Line::from(vec![initial_mess
                                                                                                  .set_style(Style::default().add_modifier(Modifier::ITALIC).fg(Color::Gray)),
                                                                                              str.to_span()]))
                                                    .alignment(Alignment::Left)
                                                    .wrap(Wrap { trim: true });
                                                last_msg.render(info_msg, buf);
                                            }
                                            Message::Media(med) => {
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
                                            Message::Message(str) => {
                                                let initial_mess = format!("{:?}:{:?} - ID {:?}: ",messdata.timestamp.time.unwrap_or(Time{ hour: 0, minute: 0, second: 0, nanosecond: 0, }).hour,
                                                                           messdata.timestamp.time.unwrap_or(Time{ hour: 0, minute: 0, second: 0, nanosecond: 0, }).minute,
                                                                           messdata.id_sender);
                                                let last_msg = Paragraph::new(Line::from(vec![initial_mess.set_style(Style::default().add_modifier(Modifier::ITALIC).fg(Color::Gray)),
                                                                                              str.to_span()]))
                                                    .alignment(Alignment::Left)
                                                    .wrap(Wrap { trim: true });
                                                last_msg.render(info_msg, buf);
                                            }
                                            Message::Media(med) => {
                                                let initial_mess = format!("{:?}:{:?} - ID {:?} sent a media 🖇",messdata.timestamp.time.unwrap_or(Time{ hour: 0, minute: 0, second: 0, nanosecond: 0, }).hour,
                                                                           messdata.timestamp.time.unwrap_or(Time{ hour: 0, minute: 0, second: 0, nanosecond: 0, }).minute
                                                                           ,messdata.id_sender);
                                                let last_msg = Paragraph::new(Text::styled(initial_mess, Style::default().add_modifier(Modifier::ITALIC).fg(Color::Gray)))
                                                    .alignment(Alignment::Left)
                                                    .wrap(Wrap { trim: true });
                                                last_msg.render(info_msg, buf);
                                            }
                                        }
                                    }
                                }
                            }
                            break 'inner;
                        }
                    }*/
                }
            }
        }
    }

    fn render_no_contacts_available(app:&App, area: Rect, buf: &mut Buffer){

        let text = Text::from(vec![
            Line::from("No Contacts Available")
                .style(Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC | Modifier::BOLD)),
            Line::from("Press 'r' to send network request to find new contacts")
                .style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
        ]);

        Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }

    // Util function for both second and third menù
    fn draw_profile_photo<'a>() -> Canvas<'a, fn(&mut Context)>{
        Canvas::default()
            .block(Block::default().borders(Borders::LEFT))
            .x_bounds([0.0, 20.0])
            .y_bounds([0.0, 20.0])
            .paint(|ctx| {
                // Front rectangle (base)
                ctx.draw(&Circle {
                    x: 10.0,
                    y: 10.0,
                    radius: 4.5,
                    color: Default::default(),
                }); // Bottom edge

            })
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
                match app.cache.tab1_data.chat_third_page_selection{
                    ChatThirdPageSelection::ScrollingMessages => Style::default().fg(Color::Rgb(89, 72, 198)),
                    ChatThirdPageSelection::BottomTab => {
                        if app.cache.tab1_data.popup_cursor==0{
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
                            match app.cache.tab1_data.chat_third_page_selection{
                                ChatThirdPageSelection::ScrollingMessages => Style::default().fg(Color::Rgb(89, 72, 198)),
                                ChatThirdPageSelection::BottomTab => {
                                    if app.cache.tab1_data.popup_cursor==1{
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
                                render_chats(app,chat_area,buf,selected_cl,third_page_data,ch)
                            }
                        }
                    }
                    Some(ch) => {
                        render_chats(app,chat_area,buf,selected_cl,third_page_data,ch)
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
        // First compute each row’s height
        let mut row_heights = Vec::with_capacity(visible.len());
        for (msg, _) in visible.iter() {
            let content = match msg {
                Message::Message(text) => text.clone(),
                Message::Media(_) => "[MEDIA] 🖇️ Select this message to open the file!".to_string(),
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
            let is_sent = meta.id_sender == selected_cl;
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
                Message::Message(text) => text.clone(),
                Message::Media(_) => "[media]".to_string(),
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
}
pub(super) mod net_rendering{
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use wg_2024::network::NodeId;
    use crate::frontend::app::App;
    use crate::frontend::app::models::tab1_types::ClientNetPopup;

    pub(super) fn render_net_options(app:&App, area:Rect, buf: &mut Buffer, client_popup: &ClientNetPopup, selected_node: &NodeId){
        match client_popup {
            ClientNetPopup::FirstPage(st) => {
                // Choose the Content Server

            }
            ClientNetPopup::SecondPage(nd) => {
                // See the file lists, renderize images and upload your file

            }
        }
    }
}