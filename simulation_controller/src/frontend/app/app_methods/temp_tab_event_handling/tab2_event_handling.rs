use log::debug;
use crate::frontend::app::App;
use ratatui::crossterm::event::KeyCode;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use crate::frontend::app::models::main_types::{AppMode, InputMode};

impl App<'_> {
    pub(crate) fn handle_event_tab2(&mut self, key: KeyCode) {
        match key {
            KeyCode::Left => {
                if self.cache.cursor > 0 {
                    self.cache.cursor -= 1
                }
            }
            KeyCode::Right => {
                if self.cache.cursor + 1 < 2 {
                    self.cache.cursor += 1
                }
            }
            KeyCode::Up => {
                if self.cache.cursor == 1 {
                    // Navigate topology list
                    self.cache.tab2_data.topology_list_state.select_previous();
                }
            }
            KeyCode::Down => {
                if self.cache.cursor == 1 {
                    // Navigate topology list
                    self.cache.tab2_data.topology_list_state.select_next();
                }
            }
            KeyCode::Enter => match self.cache.cursor {
                0 => {
                    self.app_mode = AppMode::Input(InputMode::Zero);
                }
                1 => {
                    // Handle topology selection
                    if let Some(selected) = self.cache.tab2_data.topology_list_state.selected() {
                        self.load_topology_template(selected);
                    }
                }
                _ => (),
            },
            _ => (),
        }
    }
    fn load_topology_template(&mut self, topology_index: usize) {
        let topology_files = vec![
            "Personalized Network",
            "Star Network",
            "Sub-Stars Network",
            "Double Chain Network",
            "Tree Network",
            "Butterfly Network",
        ];

        if let Some(filename) = topology_files.get(topology_index) {
            let file_path = match *filename{
                "Personalized Network" => "topologies/personalized_network.toml",
                "Star Network" => "topologies/star_network.toml",
                "Double Chain Network"=> "topologies/double_chain_network.toml",
                "Sub-Stars Network" => "topologies/sub_stars_network.toml",
                "Tree Network" => "topologies/tree_network.toml",
                "Butterfly Network" => "topologies/butterfly_network.toml",
                _ => {
                    debug!(target: "main", "Topology not found");
                    return;
                }
            };

            match std::fs::read_to_string(&file_path) {
                Ok(content) => {
                    // Load the file content into the TOML text area
                    self.cache.text_buffer.toml_text_area = tui_textarea::TextArea::from(content.lines());

                    // Automatically save to chosen_topology.toml
                    self.save_text_buffer();

                }
                Err(err) => {
                    // Log error if file can't be read
                    debug!(target: "main", "Failed to load topology file '{}': {}", filename, err);
                }
            }
        }
    }
}

