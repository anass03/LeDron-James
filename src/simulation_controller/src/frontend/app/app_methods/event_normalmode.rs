use ratatui::crossterm::event::{KeyEvent, KeyEventKind};
use super::super::App;
use super::super::ui;
impl App<'_>{
    pub(crate) fn handle_event_normalmode(&mut self, key: KeyEvent){
        if key.kind == KeyEventKind::Press {
            match key.code.to_string().to_lowercase().as_str() {
                "a" => self.previous_tab(),
                "d" => self.next_tab(),
                "p" => println!("{:?}", self.cache.text_buffer.text_area.clone().into_lines()),
                "q" => self.stay = false,
                _ => {
                    match self.selected_tab {
                        ui::tabs::SelectedTab::Overview => (self.handle_event_tab0(key.code)),
                        ui::tabs::SelectedTab::Topology => self.handle_event_tab1(key.code),
                        ui::tabs::SelectedTab::TOMLEditor => self.handle_event_tab2(key.code),
                        ui::tabs::SelectedTab::Settings => (),
                    }
                }
            }
        }
    }
}