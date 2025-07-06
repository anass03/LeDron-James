use ratatui::crossterm::event::{KeyEvent, KeyEventKind};
use crate::backend::sc_models::LogType;
use super::super::App;
use super::super::ui;
use std::io::Write;
use log::*;
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
                    for (x,y,z) in self.cache.tab1_data.netnode_list.items.clone(){
                        error!(target: "main", "{:?}\n{:?}",z.map, z.map_links)
                    }
                }
                "q" => self.stay = false,
                "i" => {
                    debug!(target:"main", "{}", format!("{:?} - \n - \n{:?}", self.simulation_controller_data.net_cache.new_downloaded_medias, self.simulation_controller_data.net_cache.known_media_list ));
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