/*
In this module you will find general not-essential methods.
*/
use std::fs::File;
use std::{io, vec};
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::Path;
use std::process::{id, Command};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use dashmap::mapref::one::Ref;
use log::{info, error, LevelFilter};
use ratatui_image::thread::ResizeRequest;
use tui_logger::TuiWidgetState;
use tui_textarea::TextArea;
use crate::backend::sc_models::{FileRendererUpdate, LogType, MediaEntry, NetCacheFile, NetCacheMedia, RendererCommand, RendererResponse};
use crate::frontend::app::models::general_types::Node;
use crate::frontend::app::topology_algorithm::alg_models::TopologyAlgorithm;
use super::super::App;
use wg_2024::config::{Client as ClientConfig, Drone as DroneConfig, Server as ServerConfig};
use host::{FileId, MediaId, ServerId};

fn read_from_file<'a>(file: File) -> io::Result<TextArea<'a>> {
    io::BufReader::new(file).lines().collect()
}
impl App<'_> {

    /// - Media Renderer Handler
    pub fn handle_rx_renderer(&mut self){
        match &self.simulation_controller_data.rx_from_renderer{
            None => {}
            Some(channel) => {
                while let Ok(event) = channel.try_recv() {
                    match event{
                        RendererResponse::EncodedMedia((media_id, media_status,update_rx)) => {
                            info!(target: "main", "[LEDRON-CLI] Received media update for: {}", media_id);
                            
                            // Update the media entry in new_downloaded_medias
                            if let Some(mut entry) = self.simulation_controller_data.net_cache.downloaded_medias.get_mut(&media_id) {
                                entry.update_cache_media(media_status);
                                // Update channels if provided
                                if let Some(rx) = update_rx {
                                    entry.rx = Some(rx);
                                }
                            } else {
                                // Create new entry if it doesn't exist
                                let mut media_entry = MediaEntry::new(media_status);
                                // Set channels if provided
                                if let Some(rx) = update_rx {
                                    media_entry.rx = Some(rx);
                                }
                                self.simulation_controller_data.net_cache.downloaded_medias.insert(media_id, media_entry);
                            }
                        },
                        RendererResponse::EncodedFile((file_id, file_status)) => {
                            info!(target: "main", "[LEDRON-CLI] Received file update for: {}", file_id);
                            // Handle file updates if needed
                            let mut rendered:bool = false;
                            match self.simulation_controller_data.net_cache.downloaded_files.get(&file_id){
                                None => (),
                                Some(item) => {
                                    match item.value(){
                                        NetCacheFile::Rendered(_) => rendered=true,
                                        _ => (),
                                    }
                                }
                            }
                            if !rendered{
                                let _ = self.simulation_controller_data.net_cache.downloaded_files.insert(file_id, file_status);
                            }
                        }
                        RendererResponse::Computed((ret_vec)) => {
                            for (id,top) in ret_vec{
                                'inner: for (node,act,ref mut current_alg) in &mut self.cache.tab1_data.net_node_list.items{
                                    let maybe_target = match node {
                                        Node::Client(cl) => cl.id,
                                        Node::Server(sr) => sr.id,
                                        Node::Drone(dr) => dr.id,
                                    };

                                    if maybe_target == id{
                                        *current_alg = top;
                                        break 'inner;
                                    }
                                }
                            }
                        },
                        RendererResponse::EncodedMediaChats(retvec) => {
                            for (id,top) in retvec{
                                self.simulation_controller_data.downloaded_chat_files.insert(id, top);
                            }
                        },
                    }
                }
            }
        }
    }

    /// Handle resize requests for rendered media
    pub(crate) fn handle_media_resize_requests(&mut self) {
        for mut entry in self.simulation_controller_data.net_cache.downloaded_medias.iter_mut() {
            // Collect updates first to avoid double mutable borrow
            let mut updates = Vec::new();
            if let Some(rx) = &mut entry.rx {
                while let Ok(update) = rx.try_recv() {
                    updates.push(update);
                }
            }
            for update in updates {
                if let NetCacheMedia::Rendered(protocol) = &mut entry.cache {
                    match update {
                        FileRendererUpdate::Redraw(resize_response) => {
                            protocol.update_resized_protocol(resize_response);
                            info!(target: "main", "[LEDRON-CLI] Updated resized protocol");
                        }
                        FileRendererUpdate::Error(err) => {
                            error!(target: "main", "[LEDRON-CLI] {}", err);
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn check_lists_initialized(&mut self){
        if !self.cache.tab1_data.net_node_list.initialized{
            let dronec = self.simulation_controller_data.drones.clone();
            let serverc = self.simulation_controller_data.servers.clone();
            let clientc = self.simulation_controller_data.clients.clone();

            self.initialize_netnodelist(dronec, serverc, clientc);

        }

        if !self.cache.main_log_state.0{
            self.cache.main_log_state.0 = true;
            let mut new_state = TuiWidgetState::default()
                .set_default_display_level(LevelFilter::Off)
                .set_level_for_target("main", LevelFilter::max());

            for (_,(target,_)) in &self.simulation_controller_data.host_log_state{
                new_state = new_state.set_level_for_target(target.as_str(), LevelFilter::max());
            }
            self.cache.main_log_state.1 = new_state;
        }


    }
    pub(in crate::frontend::app) fn initialize_netnodelist(&mut self, drones: Vec<DroneConfig>, servers: Vec<ServerConfig>, clients: Vec<ClientConfig>){

        let mut nodes = Vec::new();
        let mut ids = Vec::new();
        for d in drones{
            ids.push(d.id);
            nodes.push((Node::Drone(d),true, TopologyAlgorithm::default()));
        }
        for s in servers{
            ids.push(s.id);
            nodes.push((Node::Server(s),true, TopologyAlgorithm::default()));
        }
        for c in clients{
            ids.push(c.id);
            nodes.push((Node::Client(c),true, TopologyAlgorithm::default()));
        }

        // Set the items first
        self.cache.tab1_data.net_node_list.items = nodes;
        self.cache.tab1_data.net_node_list.selected= 0;
        self.cache.tab1_data.net_node_list.state.select(Some(self.cache.tab1_data.net_node_list.selected as usize));
        self.cache.tab1_data.net_node_list.initialized= true;

        // Debug logging for running_network
        info!(target: "main", "[INIT] - Running network contains {} nodes", self.simulation_controller_data.running_network.len());
        for (netnode, connections) in &self.simulation_controller_data.running_network {
            info!(target: "main", "[INIT] - Node {:?} has {} connections: {:?}", netnode, connections.len(), connections);
        }

        // Now initialize the topology maps for each node
        for id in ids{
            self.initialize_node_pop_map(id);
        }
    }
    pub(super) fn save_text_buffer(&mut self) {
        let lines = self.cache.text_buffer.toml_text_area.clone().into_lines();
        match File::create(Path::new(self.settings.toml_path.as_str())){
            Ok(mut file) => {
                for x in lines {
                    writeln!(file, "{}", x); // adds a newline after each string
                }
            }
            Err(e) => {
                error!(target: "main", "[LEDRON-CLI] {e}");
            }
        }

    }
    pub(crate) fn reload_file_to_buf(&mut self) {
        match File::open(Path::new(self.settings.toml_path.as_str())) {
            Ok(mut file) => {
                match read_from_file(file) {
                    Ok(text) => {
                        self.cache.text_buffer.toml_text_area = text;
                    }
                    Err(e) => {
                        error!(target: "main", "[LEDRON-CLI] Error reading file: {e})");
                    }
                }
            }
            Err(e) => {
                error!(target: "main", "[LEDRON-CLI] Error opening file: {:?}", e);
                // File doesn't exist
                // 1 - Create file
                // 2 - Link new created file path to settings
                // 3 - Test open, if it doesn't work AGAIN, set app status to failure
            }
        }
    }
    pub(crate) fn next_tab(&mut self) {
        self.cache.cursor = 0;
        self.selected_tab = self.selected_tab.next()
    }
    pub(crate) fn previous_tab(&mut self) {
        self.cache.cursor = 0;
        self.selected_tab = self.selected_tab.prev()
    }

}
pub(crate) fn open_file(path: &str) {
    // Open with system default application
    #[cfg(target_os = "windows")]
    {
        let path_str = path.to_string();
        //let quoted_path = format!("\"{}\"", path_str);
        Command::new("cmd")
            .args(&["/C", "start", "", &path_str])
            .spawn()
            .expect("Unable to open file on Windows");
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&path)
            .spawn()
            .expect("Unable to open file on macOS");
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .expect("Unable to open file on Linux");
    }
}
