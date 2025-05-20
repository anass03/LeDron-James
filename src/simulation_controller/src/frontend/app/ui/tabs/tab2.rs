use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Padding, Paragraph, Tabs, Widget, Wrap};
use crate::frontend::app::App;
use crate::frontend::app::model::TOMLmessages;

const SELECTED_STYLE: Style = Style::new().bg(Color::Blue).add_modifier(Modifier::BOLD);

/// Renders TOML Editor tab-page
pub(super) fn render_tab2(app:&App, area: Rect, buf : &mut Buffer){
    let inner_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70),
            Constraint::Percentage(30),
        ]).split(area);


    let toml_view_area = inner_columns[0];
    let menu_view_area = inner_columns[1];


    let mut toml_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        //.border_type(BorderType::)
        .title(" TOML FILE VIEW ")
        .title_alignment(Alignment::Center);

    if app.cache.cursor == 0{
        toml_block = toml_block.border_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    }

    let inner_toml_block = toml_block.inner(toml_view_area);

    let inner_toml_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(inner_toml_block);

    let compiler_area = inner_toml_layout[0];

    // Area which will be used for text editor
    let editor_area = inner_toml_layout[1];

    let compiler_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::White))
        .border_type(BorderType::Double);
    let inner_compiler_block = compiler_block.inner(compiler_area);
    let mut compiler_status = Paragraph::default();
    match &app.cache.tab2_data.compiler_buf{
        TOMLmessages::Correct => {
            compiler_status = Paragraph::new("Topology validation passed!")
                .style(Style::default().fg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD|Modifier::ITALIC))
                .centered();
        }
        TOMLmessages::Error(str) => {
            compiler_status = Paragraph::new(format!("Error compiling: {}",str))
                .style(Style::default().fg(Color::Red)
                    .add_modifier(Modifier::BOLD|Modifier::ITALIC))
                .centered();
        }
        TOMLmessages::FirstInizialized => {
            compiler_status = Paragraph::new("Compile manually to check TOML validity")
                .style(Style::default().fg(Color::Gray)
                    .add_modifier(Modifier::BOLD| Modifier::ITALIC))
                .centered();
        }
    }

    let mut menu_view_area_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .title(" Choose your TOML function ")
        .title_alignment(Alignment::Center);
    let inner_menu_view_area = menu_view_area_block.inner(menu_view_area);
    /*
    - Generate some demo toml topologies
    - help page with all help with shortcuts etc.
    */
    if app.cache.cursor >= 1{
        menu_view_area_block = menu_view_area_block.border_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    }

    /*
    let list_menu_view= vec!["Delete toml content",
                             "Generate random network",
                             "Help"];
    let items:Vec<ListItem> = list_menu_view
        .iter()
        .enumerate()
        .map(| (i,&item)|{
            ListItem::from(item)
        }).collect();

    let list = List::new(items)
        .highlight_style(SELECTED_STYLE)
        .highlight_symbol(">");
     */



    //list.render(inner_menu_view_area, buf);
    menu_view_area_block.render(menu_view_area, buf);
    app.cache.text_buffer.toml_text_area.render(editor_area, buf);
    compiler_status.render(inner_compiler_block, buf);
    compiler_block.render(compiler_area,buf);
    toml_block.render(toml_view_area, buf);
}
fn editor_engine(){

}