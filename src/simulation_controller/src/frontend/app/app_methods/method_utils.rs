/*
In this module you will find general not-essential methods.
*/
use std::fs::File;
use std::{fs, io};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::Thread;
use log::LevelFilter;
use ratatui::text::Text;
use ratatui_image::thread::ResizeRequest;
use rodio::{Decoder, OutputStream, Sink};
use tui_logger::TuiWidgetState;
use tui_textarea::TextArea;
use wg_2024::network::NodeId;
use initializer_utils::utils::parse_and_check_config;
use crate::backend::sc_models::{FileRendererUpdate, LogType, NetCacheMedia, RendererCommand};
use crate::backend::simulation_controller::NetNode;
use crate::frontend::app::topology_algorithm::alg_models::{DisplayedNode, TopologyStatus};
use crate::frontend::app::topology_algorithm::vectors::Vector;
use crate::frontend::app::ui::tabs::SelectedTab;
use super::super::App;



fn read_from_file<'a>(file: File) -> io::Result<TextArea<'a>> {
    io::BufReader::new(file).lines().collect()
}
impl App<'_> {
    pub(crate) fn handle_downloaded_medias(&mut self){
        for (server_id, values) in &mut self.simulation_controller_data.net_cache.downloaded_medias{
            for (id,(channels,media)) in values{
                match media{

                    NetCacheMedia::Rendered(ref mut rendered) => {
                        match &channels.1{
                            None => {
                                // Should have a channel
                            }
                            Some(channel) => {
                                while let Ok(event) = channel.try_recv() {
                                    match event {
                                        FileRendererUpdate::Redraw(redraw) => {
                                            rendered.update_resized_protocol(redraw);
                                            self.simulation_controller_data.logs.push_sc_log(LogType::ERROR, "Updated resized protocol".to_string());
                                        }
                                        FileRendererUpdate::Error(err) => {
                                            self.simulation_controller_data.logs.push_sc_log(LogType::ERROR, err)
                                        }
                                    }
                                }
                            }
                        }
                    }
                    NetCacheMedia::FatalErrorRendering() => {
                        // Recovery mode?
                    }
                    _ => (),
                }
            }
        }
    }

    pub(crate) fn check_lists_initialized(&mut self){
        if !self.cache.tab1_data.netnode_list.initialized{
            let dronec = self.simulation_controller_data.drones.clone();
            let serverc = self.simulation_controller_data.servers.clone();
            let clientc = self.simulation_controller_data.clients.clone();

            self.cache.tab1_data.netnode_list.initialize(dronec, serverc, clientc);


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
    pub(super) fn clean_topology_inputs_buffer(&mut self){
        self.cache.text_buffer.inputfirst_text_area = TextArea::default();
        self.cache.text_buffer.inputsecond_text_area = TextArea::default();
        self.cache.text_buffer.inputthird_text_area = TextArea::default();
        self.cache.text_buffer.chat_text_area = TextArea::default();
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
                self.simulation_controller_data.logs.push_log(LogType::ERROR, e.to_string().as_str());
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
                        self.simulation_controller_data.logs.push_log(LogType::ERROR,format!("Error reading file: {:?}", e).as_str());
                    }
                }
            }
            Err(e) => {
                self.simulation_controller_data.logs.push_log(LogType::ERROR,format!("Error opening file: {:?}", e).as_str());
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

    /*pub(crate) fn play_sound(&mut self) -> io::Result<()>{
        std::process::Command::new("osascript")
            .arg("-e")
            .arg("set volume output volume 75") // 0 to 100
            .status()?;

        if let Ok((_stream, handle)) = OutputStream::try_default() {
            let sink = Sink::try_new(&handle).unwrap();
            sink.set_volume(75.0);
            let file = File::open("assets/beep.mp3").unwrap(); // your audio file here
            let source = Decoder::new(BufReader::new(file)).unwrap();
            sink.append(source);
            sink.detach(); // allows it to play asynchronously
           // Thread::slee
        }

        self.simulation_controller_data.logs.push_log(LogType::DEBUG, "play_sound");

        Ok(())
    }*/

    // Early version of the compiler, just calls parse_config from the ledron git
    // branch and pushes it directly to the compiler
    /*pub(crate) fn compiler_beta(file_path: &str) -> Result<(), String> {
        match parse_config(file_path) {
            Ok(config) => { check_config(&config) }
            Err(e) => {
                Err(e.to_string())
            }
        }
    }*/
}