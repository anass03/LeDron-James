use ratatui::crossterm::event::{KeyEvent, KeyEventKind};
use crate::backend::sc_models::LogType;
use super::super::App;
use super::super::ui;
use std::io::Write;
use log::*;
fn wrap_text(text: &str, min: usize, max: usize) -> String {
    let mut result = String::new();
    let mut buffer = String::new();

    for word in text.split_whitespace() {
        if !buffer.is_empty() {
            if buffer.len() + 1 + word.len() <= max {
                buffer.push(' ');
                buffer.push_str(word);
            } else {
                result.push_str(buffer.trim_end());
                result.push('\n');
                buffer.clear();
                buffer.push_str(word);
            }
        } else {
            buffer.push_str(word);
        }
    }

    if !buffer.is_empty() {
        result.push_str(buffer.trim_end());
        result.push('\n');
    }

    result
}
impl App<'_>{
    pub(crate) fn handle_event_normalmode(&mut self, key: KeyEvent){
        if key.kind == KeyEventKind::Press {
            match key.code.to_string().to_lowercase().as_str() {
                "a" => self.previous_tab(),
                "d" => self.next_tab(),
                "p" => {
                    let new_links = self.populate_topology_links();
                    self.simulation_controller_data.topology_algorithm_data.initialize(new_links);
                },
                "o" => {
                    error!(target:"main", "{:?}", wrap_text(format!("{:?}", self.simulation_controller_data.chat_cache.local_chats).as_str(),30,40 ))
                }
                "q" => self.stay = false,
                "i" => {
                    debug!(target:"main", "{}", format!("{:?} - \n - \n{:?}", self.simulation_controller_data.net_cache.downloaded_medias, self.simulation_controller_data.net_cache.known_media_list ));
                }
                _ => {
                    match self.selected_tab {
                        ui::tabs::SelectedTab::Overview => (self.handle_event_tab0(key.code)),
                        ui::tabs::SelectedTab::Topology => self.handle_event_tab1(key),
                        ui::tabs::SelectedTab::TOMLEditor => self.handle_event_tab2(key.code),
                        ui::tabs::SelectedTab::Settings => self.handle_event_tab3(key.code),
                    }
                }
            }
        }
    }
}