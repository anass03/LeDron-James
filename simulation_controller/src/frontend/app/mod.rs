mod app_methods;
mod ui;
pub mod topology_algorithm;
mod models;

use ui::tabs::{render_footer, render_inner, render_top};
use models::main_types::{AppMode, Cache, Settings};

use ratatui::{buffer::Buffer, layout::{
    Layout, Rect
}, widgets::Widget, DefaultTerminal, Frame};

use std::{fs, io, io::Write, thread, time};
use std::cmp::PartialEq;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{mpsc, LockResult};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{JoinHandle, Thread};
use std::time::Duration;
use time::Instant;
use host::{ClientId, FileId, MediaId, ServerId};
use image::{DynamicImage, ImageBuffer, Rgb, RgbImage, Rgba};
use log::{debug, error, info, warn};
use rand::Rng;
use ratatui::style::Stylize;
use ratatui::widgets::{Block, Borders};
use ratatui_image::errors::Errors;
use ratatui_image::picker::Picker;
use ratatui_image::StatefulImage;
use ratatui_image::thread::{ResizeRequest, ResizeResponse, ThreadProtocol};
use wg_2024::network::NodeId;
use initializer_utils::utils::parse_and_check_config;
use crate::backend::simulation_controller::SimulationControllerData;
use models::general_types::TOMLMessages;
use server::{MediaSC, MessageMetaData, MessageSC};
use crate::backend::sc_models::{FileRendererUpdate, FileType, LogType, NetCacheFile, NetCacheMedia, RendererCommand, RendererResponse};
use crate::backend::sc_models::RendererCommand::ComputeSetOfTopologies;
use crate::backend::sc_models::RendererResponse::Computed;
use crate::frontend::app::models::main_types::AppMode::FileExploring;
use crate::frontend::app::models::tab1_types::{ClientNetPopup, ClientPopup, NodePopup, RenderingImage, ViewedTab1};
use crate::frontend::app::topology_algorithm::alg_models::{TopologyAlgorithm, TopologyStatus};
use crate::frontend::app::ui::tabs::{SelectedTab, BG_COLOR};

pub struct App<'a> {
    renderer: JoinHandle<()>,


    simulation_controller_data: SimulationControllerData,
    stay: bool,
    app_mode: AppMode,
    tick_rate: Duration, // Moving inside settings?

    pub cache: Cache<'a>, // Contains data required by the app to work
    pub settings: Settings, // Setting page containing various config stuff
    pub selected_tab: SelectedTab,
}
impl App<'_>{
    pub fn default(mut simulation_controller_data: SimulationControllerData) -> Self {
        let ((tx_to_renderer, rx_from_renderer), renderer) = renderer_thread_gen();
        Self{
            renderer,

            simulation_controller_data: {
                simulation_controller_data.tx_to_renderer = Some(tx_to_renderer);
                simulation_controller_data.rx_from_renderer = Some(rx_from_renderer);
                simulation_controller_data
            },
            stay:true,
            app_mode:AppMode::Normal,
            tick_rate:Duration::from_millis(200),

            settings: Settings::load(),
            selected_tab: SelectedTab::default(),
            cache: Cache::default()
        }
    }
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()>{
        let mut instant = Instant::now();

        while self.stay.clone() {

            // Listening to channels
            self.simulation_controller_data.handle_channels();
            self.handle_rx_renderer();
            self.check_main_topology_status();

            // Terminal drawing and other basic app functions
            self.cache.update_counter();


            // Combined rendering in a single frame to prevent flashing
            terminal.draw(|f| {
                // First render the main widget
                f.render_widget(&mut *self, f.area());
                
                // Then render image if active - simplified and optimized
                match self.selected_tab{
                    SelectedTab::Topology => {
                        match self.cache.tab1_data.viewing.clone() {
                            ViewedTab1::Popup((_,popup)) => {
                                match popup{
                                    NodePopup::Client(clpop) => {
                                        match clpop {
                                            ClientPopup::InternetPage(netpop) => {
                                                match netpop{
                                                    ClientNetPopup::SecondPage(ndpage) => {
                                                        match ndpage.image_rendering_data {
                                                            RenderingImage::True(data) => {
                                                                Self::render_image_internal(f, self, data);
                                                            }
                                                            _ => {}
                                                        }
                                                    }
                                                    _ => {}
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => (),
                }
            })?;

            self.handle_events()?;


            // Check if it is time to refresh some data
            if instant.elapsed() > self.tick_rate {
                instant = Instant::now();
                self.tick_refresh();
            }

            // Handle media resize requests
            self.handle_media_resize_requests();


            if self.stay == false{
                // QUITTING MODE: We should ask again to be sure if the user wants to quit
                // To do: Design a Quitting page with loading bar

                // 1 - ReCALLING terminal.draw(|f| f.render_widget(&*self, f.area()))?;
                // GOTTA MODIFY RENDER_WIDGET to DRAW frame accordly to what was being shown before and adding a tab over it

                // 2 - ReCALLING self.handle_events() that will process only 2 inputs (arrow up / down) to choose YES or NO
                // on the "Are you sure to quit the app" tab
                // GOTTA MODIFY HANDLE_EVENTS to HANDLE QUITTING MODE
             }
        }
        Ok(())
    }
    fn render_image(f: &mut Frame<'_>, app: &mut App, data: (Rect,ClientId, ServerId, MediaId)){
        Self::render_image_internal(f, app, data);
    }
    
    fn render_image_internal(f: &mut Frame<'_>, app: &mut App, data: (Rect,ClientId, ServerId, MediaId)){
        // Early return if area is invalid
        if data.0.width == 0 || data.0.height == 0 {
            return;
        }
        
        // Check if media exists in new_downloaded_medias
        if let Some(mut media_entry) = app.simulation_controller_data.net_cache.downloaded_medias.get_mut(&data.3) {
            if let NetCacheMedia::Rendered(protocol) = &mut media_entry.cache {
                // Create a StatefulImage widget and render it properly
                let image_widget = StatefulImage::new();
                f.render_stateful_widget(image_widget, data.0, protocol);
                return;
            }
        }
    }
    fn handle_events(&mut self) -> io::Result<()>{
        use std::time::Duration;
        use ratatui::crossterm::event::{self, Event};

        if event::poll(Duration::from_millis(40))? {
            let event = ratatui::crossterm::event::read()?;
            match event {
                Event::Key(key) => {
                    match self.app_mode{
                        AppMode::Normal => {
                            self.handle_event_normalmode(key);
                        },
                        AppMode::Input(_) => {
                            self.handle_event_inputmode(event);
                        },
                        AppMode::Scrolling => {
                            self.handle_event_scrollingmode(key);
                        },
                        AppMode::FileExploring => {
                            self.handle_event_file_exploringmode(event)
                        },
                    }
                },
                _ => {}
            }
            /*
            if let Event::(key) = crossterm::event::read()? { // Blocking fn
                if self.input_mode{
                    self.handle_event_inputmode(key);
                }else{
                    self.handle_event_normalmode(key);
                }
            }

             */
        }
        Ok(())
    }

    /// Update cache & tabs data
    ///
    /// Gotta add a refresher that based on the currently selected option does some requests / refreshes.
    fn tick_refresh(&mut self) {
        // Update dropped packets chart
        let total_paket_dropped =  self.simulation_controller_data.stats.total_packet_dropped;
        let graph_width = self.cache.tab0_data.charts_data.dropped_packets_chart_width;
        self.cache.tab0_data.charts_data.update_dropped_per_tick(total_paket_dropped, graph_width);

        let total_packets_sent = self.simulation_controller_data.stats.total_packet_exchanged;
        self.cache.tab0_data.charts_data.update_sent_per_tick(total_packets_sent, graph_width);

        // We initialize the TOML buffer with the file indicated in the settings
        match self.cache.tab2_data.compiler_buf{
            TOMLMessages::FirstInitialized => {
                self.reload_file_to_buf();

                match parse_and_check_config(&self.settings.toml_path){
                    Ok(_) => {self.cache.tab2_data.compiler_buf = TOMLMessages::Correct},
                    Err(e) => {self.cache.tab2_data.compiler_buf = TOMLMessages::Error(e)}
                }
            }
            _ => ()
        }

        // Nodelist initialized check
        self.check_lists_initialized();

        //self.refresh_cache();
    }
}
impl Widget for &mut App<'_>{
    // Each tab render function returns to this fn
    fn render(mut self, area: Rect, buf: &mut Buffer)
    {
        use ratatui::layout::Constraint::{Length, Min};
        use ui::tabs::{render_top,render_inner,render_footer};

        let app_colored_area = Block::default()
            .bg(BG_COLOR);
        let inner_colored = app_colored_area.inner(area);
        app_colored_area.render(area, buf);

        let vertical = Layout::vertical([Length(3), Min(0), Length(3)]);
        let [top_area, inner_area, footer_area] = vertical.areas(inner_colored);

        render_top(self, top_area, buf);
        render_inner(self, inner_area, buf);
        render_footer(self, footer_area, buf);
    }
}

fn renderer_thread_gen() ->((Sender<RendererCommand>, Receiver<RendererResponse>), JoinHandle<()>){
    let (tx_cmd, rx_cmd) = mpsc::channel::<RendererCommand>();
    let (tx_rsp, rx_rsp) = mpsc::channel::<RendererResponse>();

    // Performance optimized renderer thread:
    // - Removed 1.5s sleep that was causing lag
    // - Optimized channel processing for better performance
    // - Simplified image processing flow
    // - Removed unnecessary frame rate limiting

    ((tx_cmd,rx_rsp),thread::spawn(move || {
        let mut download_folder = PathBuf::from("assets");
        let mut channels = Vec::new();

        let mut picker = Picker::from_query_stdio();
        let mut total_requested_picker = 0;

        let mut already_downloaded_files:HashSet<MediaId> = HashSet::new();

        loop {
            // Ascolto il canale main
            if let Ok(request) = rx_cmd.try_recv() {
                match request {
                    RendererCommand::Stop => {
                        break
                    }
                    RendererCommand::EncodeMedia((media_id, image)) => {
                        let extension = media_id.rsplitn(2, '.').next().filter(|&ext| ext != media_id);
                        // Construct path safely
                        let path = download_folder.join(media_id.clone());
                        // Ensure download directory exists
                        if let Some(parent) = path.parent() {
                            _ = fs::create_dir_all(parent);
                        }
                        // Create image buffer without cloning if possible
                        let opt_rgba_buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(image.width, image.height, image.data);
                        match extension{
                            None => {}
                            Some(ext) => {
                                match opt_rgba_buffer{
                                    None => {}
                                    Some(rgba_buffer) => {
                                        let dynamic_image = DynamicImage::ImageRgba8(rgba_buffer);
                                        // Save based on extension
                                        match ext.to_lowercase().as_str() {
                                            "jpg" | "jpeg" => {
                                                // Convert to RGB to drop alpha channel for JPEG
                                                let rgb_image: RgbImage = dynamic_image.to_rgb8();
                                                _ = rgb_image.save(&path);
                                            },
                                            _ => {
                                                // For PNG and other formats that support alpha
                                                _ = dynamic_image.save(&path);
                                            }
                                        };
                                        
                                        // Send downloaded status first
                                        match tx_rsp.send(RendererResponse::EncodedMedia((media_id.clone(), NetCacheMedia::Downloaded(path), None))) {
                                            Ok(_) => {
                                                info!(target: "main", "[RENDERER] - Successfully sent downloaded status");
                                            }
                                            Err(str) => {
                                                error!(target: "main", "[RENDERER] - Error sending downloaded status: {:?}", str);
                                            }
                                        }
                                        
                                        // Now render the image
                                        let max_retries = 3;
                                        let mut retry_count = 0;
                                        let mut render_success = false;
                                        
                                        while retry_count < max_retries && !render_success {
                                            match &picker{
                                                Ok(pick) => {
                                                    // Create channels for resize requests
                                                    let (resize_tx, resize_rx) = mpsc::channel::<ResizeRequest>();
                                                    let (update_tx, update_rx) = mpsc::channel::<FileRendererUpdate>();
                                                    
                                                    // Create the ThreadProtocol with the resize sender
                                                    let return_item = ThreadProtocol::new(resize_tx.clone(), Some(pick.new_resize_protocol(dynamic_image.clone())));

                                                    channels.push((update_tx,resize_rx));

                                                    // Send the rendered status with the channels
                                                    match tx_rsp.send(RendererResponse::EncodedMedia((media_id.clone(), NetCacheMedia::Rendered(return_item), Some(update_rx)))) {
                                                        Ok(_) => {
                                                            info!(target: "main", "[RENDERER] - Successfully sent rendered status");
                                                            render_success = true;
                                                        }
                                                        Err(str) => {
                                                            error!(target: "main", "[RENDERER] - Error sending rendered status: {:?}", str);
                                                            retry_count += 1;
                                                        }
                                                    }
                                                }
                                                Err(err) => {
                                                    total_requested_picker +=1;
                                                    retry_count += 1;
                                                    if err.to_string().contains("No response from stdin") {
                                                        warn!(target: "main", "[RENDERER] - No response from stdin, retry {}/{}", retry_count, max_retries);
                                                        // Small delay before retry
                                                        thread::sleep(Duration::from_millis(100));
                                                    } else {
                                                        error!(target: "main", "[RENDERER] - Error rendering the image-buffer to picker-image [{:?}]", err.to_string());
                                                        break; // Don't retry for non-stdin errors
                                                    }
                                                }
                                            }
                                            if picker.is_err() && total_requested_picker<10{
                                                picker = Picker::from_query_stdio();
                                            }
                                        }
                                        
                                        // If all retries failed, send fatal rendering error
                                        if !render_success {
                                            error!(target: "main", "[RENDERER] - Failed to render image after {} retries, sending fatal error", max_retries);
                                            match tx_rsp.send(RendererResponse::EncodedMedia((media_id, NetCacheMedia::FatalErrorRendering(), None))) {
                                                Ok(_) => {
                                                    info!(target: "main", "[RENDERER] - Sent fatal rendering error response");
                                                }
                                                Err(str) => {
                                                    error!(target: "main", "[RENDERER] - Error sending fatal rendering response: {:?}", str);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    RendererCommand::ComputeSetOfTopologies(top_algs) => {
                        let mut ret_vec: Vec<(NodeId, TopologyAlgorithm)> = Vec::new();
                            for (id, mut top_alg) in top_algs {
                                let mut iteration_count = 0;
                                const MAX_ITERATIONS: u32 = 1000; // Safety limit
                                let mut attempts = 0;
                                const MAX_ATTEMPTS: u32 = 3; // Maximum reinitialization attempts for convergence issues

                                'convergence_loop: loop {
                                    while top_alg.status != TopologyStatus::Converged && iteration_count < MAX_ITERATIONS {
                                        top_alg.check_general_node_topology_status();
                                        iteration_count += 1;
                                    }

                                    // Check if the map is valid (no NaN, no points outside boundaries)
                                    let is_valid = top_alg.is_map_valid();

                                    if top_alg.status == TopologyStatus::Converged && is_valid {
                                        // Perfect! Converged and valid
                                        break 'convergence_loop;
                                    } else if is_valid && top_alg.status != TopologyStatus::Converged {
                                        // Valid but didn't converge - limited attempts
                                        attempts += 1;
                                        if attempts >= MAX_ATTEMPTS {
                                            warn!(target: "topology", "[RENDERER] - Node {} valid but didn't converge after {} attempts, keeping current state", id, MAX_ATTEMPTS);
                                            break 'convergence_loop;
                                        }
                                        warn!(target: "topology", "[RENDERER] - Node {} valid but didn't converge after {} iterations, reinitializing (attempt {}/{})", id, MAX_ITERATIONS, attempts, MAX_ATTEMPTS);
                                    } else {
                                        // Invalid map - keep trying indefinitely
                                        warn!(target: "topology", "[RENDERER] - Node {} has invalid map (NaN or out of bounds), reinitializing", id);
                                    }

                                    // Reinitialize the topology algorithm
                                    let current_links = top_alg.map_links.clone();
                                    top_alg.initialize(current_links);
                                    iteration_count = 0;
                                }

                                ret_vec.push((id,top_alg));
                            }
                            match tx_rsp.send(Computed(ret_vec)){
                                Ok(_) => {
                                    info!(target: "topology", "[RENDERER] - Successfully sent computed topologies");
                                },
                                Err(err) => {
                                    error!(target: "topology", "{}", format!("[RENDERER] - Error sending computed topologies [{:?}]", err.to_string()));
                                }
                            }
                        }
                    RendererCommand::EncodeFile((file_id,file_buf)) => {
                        // Construct path safely
                        let path = download_folder.join(file_id.clone());
                        // Ensure download directory exists
                        if let Some(parent) = path.parent() {
                            _ = fs::create_dir_all(parent);
                        }
                        match fs::write(&path, &file_buf) {
                            Ok(_) => info!(target: "main", "[LEDRON-CLI] File {file_id} downloaded in : {path:?}"),
                            Err(_) => error!(target: "main", "[RENDERER] Couldn't download {file_id} in {path:?}"),
                        }
                        match tx_rsp.send(RendererResponse::EncodedFile((file_id.clone(), NetCacheFile::Downloaded(path.clone())))){
                            Ok(_) => (),
                            Err(str) => {
                                error!(target: "main", "[RENDERER] - Error sending encoded file:\n{:?}", str);
                            }
                        }

                        match std::str::from_utf8(&file_buf) {
                            Ok(valid_str) => {
                                let ret_utf = valid_str.to_string().lines().map(|str| str.to_string()).collect::<Vec<String>>();
                                info!(target: "main", "[RENDERER] Successfully verified that file is correctly formatted as UTF-8");
                                match tx_rsp.send(RendererResponse::EncodedFile((file_id, NetCacheFile::Rendered(FileType::UTF8(ret_utf))))){
                                    Ok(_) => {}
                                    Err(_) => {}
                                }
                            }
                            Err(_) => {
                                warn!(target: "main", "[RENDERER] File is not formatted as UTF-8, use an external renderer");
                                match tx_rsp.send(RendererResponse::EncodedFile((file_id, NetCacheFile::Rendered(FileType::Other(path))))){
                                    Ok(_) => (),
                                    Err(str) => {
                                        error!(target: "main", "[RENDERER] - Error sending encoded file:\n{:?}", str);
                                    }
                                }
                            }
                        }
                    },
                    RendererCommand::ChangeDownloadFolder(path) => {
                        download_folder = path;
                    },
                    RendererCommand::SaveMediaFromChatList(ch_ptr) => {
                        let mut retvec:Vec<(MediaId,PathBuf)> = Vec::new();
                        match ch_ptr.read(){
                            Ok(ch) => {
                                for (msg,_) in &ch.chat{
                                    match msg{
                                        MessageSC::Media(media) => {
                                            match media{
                                                MediaSC::Image(media_id, image) => {
                                                    if !already_downloaded_files.contains(media_id.as_str()) {
                                                    let extension = media_id.rsplitn(2, '.').next().filter(|&ext| ext != media_id);
                                                    // Construct path safely
                                                    let path = download_folder.join(media_id.clone());
                                                    // Ensure download directory exists
                                                    if let Some(parent) = path.parent() {
                                                        _ = fs::create_dir_all(parent);
                                                    }
                                                    // Create image buffer without cloning if possible
                                                    let opt_rgba_buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(image.width, image.height, image.data.clone());
                                                    match extension {
                                                        None => {}
                                                        Some(ext) => {
                                                            match opt_rgba_buffer {
                                                                None => {}
                                                                Some(rgba_buffer) => {
                                                                    let dynamic_image = DynamicImage::ImageRgba8(rgba_buffer);
                                                                    // Save based on extension
                                                                    match ext.to_lowercase().as_str() {
                                                                        "jpg" | "jpeg" => {
                                                                            // Convert to RGB to drop alpha channel for JPEG
                                                                            let rgb_image: RgbImage = dynamic_image.to_rgb8();
                                                                            match rgb_image.save(&path) {
                                                                                Ok(_) => {
                                                                                    retvec.push((media_id.clone(), path));
                                                                                }
                                                                                _ => (),
                                                                            }
                                                                        },
                                                                        _ => {
                                                                            // For PNG and other formats that support alpha
                                                                            match dynamic_image.save(&path) {
                                                                                Ok(_) => {
                                                                                    retvec.push((media_id.clone(), path));
                                                                                }
                                                                                _ => (),
                                                                            }
                                                                        }
                                                                    };
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            MediaSC::File(file_id, file_buf) => {
                                                if !already_downloaded_files.contains(file_id.as_str()) {
                                                let path = download_folder.join(file_id.clone());
                                                // Ensure download directory exists
                                                if let Some(parent) = path.parent() {
                                                    _ = fs::create_dir_all(parent);
                                                }
                                                match fs::write(&path, &file_buf) {
                                                    Ok(_) => {
                                                        info!(target: "main", "[RENDERER] File {file_id} downloaded in : {path:?}");
                                                        retvec.push((file_id.clone(), path));
                                                    },
                                                    Err(_) => error!(target: "main", "[RENDERER] Couldn't download {file_id} in {path:?}"),
                                                }
                                            }
                                            }
                                        }
                                        }
                                        _ => (),
                                    }
                                }
                            }
                            Err(_) => (),
                        }
                        let is_or_no = retvec.is_empty();
                        if !retvec.is_empty(){
                            match tx_rsp.send(RendererResponse::EncodedMediaChats(retvec)){
                                Ok(_) => (),
                                Err(str) => {
                                    error!(target: "main", "[RENDERER] - Error sending encoded media chats:\n{:?}", str);
                                }
                            }
                        }
                    }
                }
            }
            // Time to check for redraw-requests
            for (tx,rx) in &channels{
                while let Ok(update) = rx.try_recv() {
                    match update.resize_encode(){
                        Ok(updated) => {
                            match tx.send(FileRendererUpdate::Redraw(updated)){
                                Ok(_) => {}
                                Err(err) => {
                                    error!(target: "main", "[RENDERER] - {}", err.to_string());
                                }
                            }
                        }
                        Err(err) => {
                            error!(target: "main", "[RENDERER] - {}", err.to_string());
                        }
                    }
                }
            }


            // Add a small sleep to prevent busy loop and reduce CPU usage
            thread::sleep(Duration::from_millis(10));

        }
    }))
}
