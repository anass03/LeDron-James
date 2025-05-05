mod tab0;
mod tab3;
mod tab2;
mod tab1;

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Padding, Paragraph, Tabs, Widget, Wrap};
use crate::frontend::app::App;


#[derive(Default, Clone, Copy)]
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
        .borders(Borders::ALL)
        .border_style(Style::new().add_modifier(Modifier::BOLD))
        .title("LeDron CLI").style(Style::default().fg({
        let temp = app.cache.colors_vec.1.get(app.cache.counter as usize%app.cache.colors_vec.0).unwrap().clone();
        Color::Rgb(temp.0, temp.1, temp.2)
    }));
    let titles: Vec<Line> = SelectedTab::menu_vec()
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let style = if i == app.selected_tab as usize {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Line::from(vec![Span::styled(t.clone(), style)])
        })
        .collect();
    let tabs = Tabs::new(titles)
        .block(main_block)
        .select(app.selected_tab as usize)
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
        .style(Style::default().fg(Color::White));
    tabs.render(area, buf);
}
pub(crate) fn render_inner(app:&App, area: Rect, buf : &mut Buffer){
    let mut main_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .border_style(Style::default().add_modifier(Modifier::BOLD))
        .border_type(BorderType::Double);
    let inner_block = main_block.inner(area);
    main_block.render(area, buf);

    match app.selected_tab{
        SelectedTab::Overview => tab0::render_tab0(app, inner_block, buf),
        SelectedTab::Topology => tab1::render_tab1(app, inner_block, buf),
        SelectedTab::TOMLEditor => tab2::render_tab2(app, inner_block, buf),
        SelectedTab::Settings => tab3::render_tab3(app, inner_block, buf),
    }
}


//  ADD MENU FOR SPECIFIC CURSOR CASE, TEXT EDITOR ETC. ETC.
pub(crate) fn render_footer(app:&App, area: Rect, buf : &mut Buffer){
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().add_modifier(Modifier::BOLD))
        .title_alignment(Alignment::Center)
        .style(Style::default().fg({
            let temp = app.cache.colors_vec.1.get(app.cache.counter as usize % app.cache.colors_vec.0).unwrap();
            Color::Rgb(temp.0, temp.1, temp.2)
        }));
    if app.input_mode{
        block = block.title(" [INPUT MODE] ESC to quit input-mode & save the text | F11 to quit input-mode without saving ")
    }else{
        block = block.title(" Press a - d to change tabs | ↑ - ↓ to change tab options, ↳ to select a field | q to quit ")
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

    let packets = Paragraph::new("Total packets exchanged: 34198").style(style).alignment(Alignment::Center);
    let conns = Paragraph::new("Total active connections: 12").style(style).alignment(Alignment::Center);
    let net = Paragraph::new("Network reliability: 95%").style(style).alignment(Alignment::Center);

    packets.render(columns[0], buf);
    conns.render(columns[1], buf);
    net.render(columns[2], buf);
}