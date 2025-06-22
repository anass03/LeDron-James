use ratatui::crossterm::event::{KeyEvent, KeyEventKind};
use crate::backend::sc_models::LogType;
use super::super::App;
use super::super::ui;
impl App<'_>{
    pub(crate) fn handle_event_normalmode(&mut self, key: KeyEvent){
        if key.kind == KeyEventKind::Press {
            match key.code.to_string().to_lowercase().as_str() {
                "a" => self.previous_tab(),
                "d" => self.next_tab(),
                "p" => {
                    let new_links = self.simulation_controller_data.populate_topology_links();
                    self.simulation_controller_data.topology_algorithm_data.initialize(new_links);
                },
                "o" => {
                    self.simulation_controller_data.logs.push_sc_log(LogType::INFO, format!("Links: {:?}\n{:?}", self.simulation_controller_data.topology_algorithm_data.map_links, self.simulation_controller_data.topology_algorithm_data.map))
                }
                "q" => self.stay = false,
                "i" => {
                    self.simulation_controller_data.logs.push_sc_log(LogType::INFO, format!("{:?}", self.simulation_controller_data.chat_cache.local_chats ));
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