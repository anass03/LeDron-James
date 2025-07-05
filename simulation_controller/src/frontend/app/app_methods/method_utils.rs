/*
In this module you will find general not-essential methods.
*/
use std::fs::File;
use std::io;
use std::io::{BufRead, Write};
use std::path::Path;
use log::LevelFilter;
use tui_logger::TuiWidgetState;
use tui_textarea::TextArea;
use crate::backend::sc_models::{FileRendererUpdate, LogType, NetCacheMedia, RendererCommand};
use super::super::App;



fn read_from_file<'a>(file: File) -> io::Result<TextArea<'a>> {
    io::BufReader::new(file).lines().collect()
}
impl App<'_> {
    pub(crate) fn handle_downloaded_medias(&mut self){
        for (server_id, values) in &mut self.simulation_controller_data.net_cache.downloaded_medias{
            for (id,(channels,media)) in values{
                match media{
                    /*NetCacheMedia::Downloaded(vec) => {
                        match channels.0.clone(){
                            None => {}
                            Some(ch_resize_req) => {
                                match self.simulation_controller_data.tx_to_renderer.clone(){
                                    None => {
                                        // Impossible
                                    }
                                    Some(ch_to_renderer) => {
                                        match ch_to_renderer.send(RendererCommand::Encode((ch_resize_req,server_id.clone(),id.clone(),vec.clone()))) {
                                            Ok(_) => {
                                                self.simulation_controller_data.logs.push_sc_log(LogType::ERROR, "Successfully sent encode request to the renderer".to_string())
                                            }
                                            Err(str) => {
                                                self.simulation_controller_data.logs.push_sc_log(LogType::ERROR, format!("Error sending encode request to the renderer [{:?}]", str.to_string()));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }*/
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