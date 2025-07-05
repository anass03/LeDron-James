mod tab0;
mod tab3;
mod tab2;
mod tab1_old;
mod tab1;
mod tab1_client_popup;
mod tab1_server_popup;
mod tab1_drone_popup;
mod tab1_main_page;

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Widget, Wrap};
use crate::frontend::app::App;
use crate::frontend::app::models::main_types::AppMode;
use crate::frontend::app::models::tab1_types::{ClientNetPopup, ClientPopup, NodePopup, RenderingImage, SelectedFileMedia, ViewedTab1};
use std::io::Write;

pub const BG_COLOR: Color = Color::Rgb(13, 10, 28);
pub(super) const POPUP_BG_COLOR: Color = Color::Rgb(24,19,54);
pub(super) const MAIN_BLOCK_BORDERS: Color = Color::Rgb(89, 72, 198);

pub(super) const ALTERNATE_LIST_COLOR: Color = Color::Rgb(70, 57, 156);
pub(super) const SELECTED_LIST_COLOR: Color = Color::Rgb(51, 41, 113);

pub(super) const CANVAS_CLIENT_COLOR: Color  = Color::Rgb(108, 88, 241);
pub(super) const CANVAS_SERVER_COLOR: Color  = Color::Rgb(221, 102, 112);
pub(super) const CANVAS_DRONE_COLOR: Color = Color::Rgb(255, 166, 0);

impl App<'_>{
    pub(super) fn animation_color(&self) -> Color {
        match self.cache.colors_vec.1.get(self.cache.counter as usize%self.cache.colors_vec.0){
            Some(color) => Color::Rgb(color.0, color.1, color.2),
            None => Color::White,
        }
    }
}


#[derive(Default, Clone, Copy,PartialEq,Eq)]
pub(crate) enum SelectedTab {
    #[default]
    Overview,
    Topology,
    TOMLEditor,
    Settings,
}
impl SelectedTab {
    fn to_string(&self) -> String {
        match self {
            SelectedTab::Overview => "Overview".to_string(),
            SelectedTab::Topology => "Topology".to_string(),
            SelectedTab::TOMLEditor => "TOML Editor".to_string(),
            SelectedTab::Settings => "Settings".to_string(),
        }
    }

    fn menu_vec() -> Vec<String> {
        vec![
            "Overview".to_string(),
            "Topology".to_string(),
            "TOML Editor".to_string(),
            "Settings".to_string(),
        ]
    }

    pub(crate) fn next(self) -> Self {
        match self {
            SelectedTab::Overview => SelectedTab::Topology,
            SelectedTab::Topology => SelectedTab::TOMLEditor,
            SelectedTab::TOMLEditor => SelectedTab::Settings,
            SelectedTab::Settings => SelectedTab::Overview,
        }
    }

    pub(crate) fn prev(self) -> Self {
        match self {
            SelectedTab::Overview => SelectedTab::Settings,
            SelectedTab::Topology => SelectedTab::Overview,
            SelectedTab::TOMLEditor => SelectedTab::Topology,
            SelectedTab::Settings => SelectedTab::TOMLEditor,
        }
    }
}

pub(crate) fn render_top(app:&App, area: Rect, buf : &mut Buffer){
    use ratatui::text::{Span};

    let main_block = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .border_type(BorderType::Plain)
        .border_style(Style::new().fg(MAIN_BLOCK_BORDERS))
        .title("LeDron CLI").title_style(Style::default().fg(app.animation_color()))
        .title_alignment(Alignment::Center);
    let inner_main_block = main_block.inner(area);
    main_block.render(area, buf);

    let mut line = Line::from(
        SelectedTab::menu_vec()
            .iter()
            .enumerate()
            .flat_map(|(i, title)| {
                let is_selected = i == app.selected_tab as usize;
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                } else {
                    Style::default().fg(Color::White)
                };
                let mut spans = vec![Span::styled(title.clone(), style)];
                if i + 1 != SelectedTab::menu_vec().len() {
                    spans.push(Span::raw("   ")); // separator
                }
                spans
            })
            .collect::<Vec<Span>>()
    );
    line = line.alignment(Alignment::Center);
    line.render(inner_main_block,buf)

}
pub(crate) fn render_inner(app:&mut App, area: Rect, buf : &mut Buffer){
    let mut main_block = Block::default()
        .border_style(Style::default().fg(Color::Yellow))
        .border_style(Style::default().add_modifier(Modifier::BOLD))
        .style(Style::default().bg(BG_COLOR));
    let inner_block = main_block.inner(area);
    main_block.render(area, buf);

    match app.selected_tab{
        SelectedTab::Overview => {
            tab0::render_tab0(app, inner_block, buf)
        },
        SelectedTab::Topology => {
            match tab1::render_tab1(app, inner_block, buf){
                None => {}
                Some(file_vis_area) => {
                    match &mut app.cache.tab1_data.viewing {
                        ViewedTab1::Popup((node_id,ndpop)) => {
                            match ndpop {
                                NodePopup::Client(clpop)=> {
                                    match clpop {
                                        ClientPopup::InternetPage(netpage) => {
                                            match netpage {
                                                ClientNetPopup::SecondPage(ndpage) => {
                                                    match &mut ndpage.selected_file {
                                                        SelectedFileMedia::Media(id) => {
                                                            ndpage.image_rendering_data = RenderingImage::True((file_vis_area,node_id.clone(),ndpage.selected_server,id.clone() ))
                                                        }
                                                        _ => (),
                                                    }
/*                                                    match &mut ndpage.selected_file{
                                                        SelectedFileMedia::Media(id) => {
                                                            app.cache.tab1_data.image_rendering_data = RenderingImage::True((file_vis_area,*node_id,ndpage.selected_server,id.clone() ))
                                                        }
                                                        _ => (),
                                                    }*/
                                                }
                                                _ => (),
                                            }
                                        }
                                        _ => (),
                                    }
                                }
                                _ => (),
                            }
                        }
                        _ => (),
                    }
                }
            }
        },
        SelectedTab::TOMLEditor => tab2::render_tab2(app, inner_block, buf),
        SelectedTab::Settings => tab3::render_tab3(app, inner_block, buf),
    }
}


//  ADD MENU FOR SPECIFIC CURSOR CASE, TEXT EDITOR ETC. ETC.
pub(crate) fn render_footer(app:&App, area: Rect, buf : &mut Buffer){
    let mut block = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .border_type(BorderType::Plain)
        .border_style(Style::new().fg(MAIN_BLOCK_BORDERS))
        .title_alignment(Alignment::Center);

    match app.app_mode{
        AppMode::Normal => {
            // Now some specific tab gotta have a different message that we will choose
            block = block.title(" Press a - d to change tabs | ↑ - ↓ to change tab options, ↳ to select a field | q to quit ")
        }
        AppMode::Input(_) => {
            block = block.title(" [INPUT MODE] ESC to quit input-mode & save the text | F1 to quit input-mode without saving ")
        }
        AppMode::Scrolling => {
            block = block.title(" [SCROLLING MODE] ESC to quit scrolling-mode | ↑ - ↓ to scroll ")
        },

        AppMode::FileExploring => {
            block = block.title(" Press a - d to change tabs | ↑ - ↓ to change tab options, ↳ to select a field | q to quit ")
        }
    }

    // Render block and get inner space
    let inner_area = block.inner(area);
    block.render(area, buf);

    // Create 3 columns
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(inner_area);

    let style = Style::default().fg(Color::White).add_modifier(Modifier::ITALIC);

    let packets = Paragraph::new(format!("Total packets exchanged: {}",app.simulation_controller_data.stats.total_packet_exchanged)).style(style).alignment(Alignment::Center);
    let conns = Paragraph::new(format!("Total active connections: {}", app.simulation_controller_data.stats.total_active_connections)).style(style).alignment(Alignment::Center);
    let mut net;
    if app.simulation_controller_data.stats.total_packet_exchanged < 1000 {
        net = Paragraph::new("Network reliability: Calculating ...").style(style).alignment(Alignment::Center)
    } else {
    net = Paragraph::new(format!("Network reliability: {}%", app.simulation_controller_data.stats.network_reliability)).style(style).alignment(Alignment::Center)
    };

    packets.render(columns[0], buf);
    conns.render(columns[1], buf);
    net.render(columns[2], buf);
}

pub(super) fn process_log_paragraph<'a>(app:&'a App<'a>) ->Paragraph<'a>{
    let lines: Vec<Line> = app
        .simulation_controller_data
        .logs
        .logs_memory
        .iter()
        .map(|(_,msg)| {
            Line::from(Span::raw(msg)).style(Style::default().fg(Color::Gray))
        }
        )
        .collect();

    let mut log_block = Block::default()
        .title("Logs")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    if app.cache.cursor == 1 && app.selected_tab==SelectedTab::Overview{
        log_block = log_block.border_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    }
    if app.cache.cursor == 4 && app.selected_tab==SelectedTab::Topology{
        log_block = log_block.border_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    }


    let a = Paragraph::new(lines)
        .block(
            log_block
        )
        .wrap(Wrap { trim: true })
        .scroll((app.simulation_controller_data.logs.log_scroll_offset as u16, 0));
    a
}