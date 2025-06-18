mod app_methods;
mod ui;
pub mod topology_algorithm;
mod models;

use ui::tabs::{render_footer, render_inner, render_top};
use models::main_types::{AppMode, Cache, Settings};

use ratatui::{
    buffer::Buffer,
    layout::{
        Layout, Rect
    },
    widgets::Widget,
    DefaultTerminal
};

use std::{io, thread, time};
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::JoinHandle;
use std::time::Duration;
use time::Instant;
use rand::Rng;
use ratatui::style::Stylize;
use ratatui::widgets::Block;
use ratatui_image::errors::Errors;
use ratatui_image::thread::{ResizeRequest, ResizeResponse};
use initializer_utils::utils::parse_and_check_config;
use crate::backend::simulation_controller::SimulationControllerData;
use models::general_types::TOMLmessages;
use crate::backend::sc_models::FileRendererUpdate;
use crate::frontend::app::models::main_types::AppMode::FileExploring;
use crate::frontend::app::models::main_types::{RendererCommand, RendererResponse};
use crate::frontend::app::ui::tabs::{SelectedTab, BG_COLOR};
/*
  DA PENSARE A UN APPROCCIO RIGUARDO L'EVENT HANDLING E PIU'
  SPECIFICATAMENTE LO SCORRIMENTO DELLE SINGOLE TAB
  - SE FARLO GENERALE, ES. TUTTI GLI ELEMENTI DELLA PAGINA IN UN VEC CON CURSORE+
  - OGNI TAB IL SUO MECCANISMO E IL SUO CURSORE
 */

/*
IDEE:
 - Aggiungere una TAB External Network che permette di connettere una rete esterna,
 alla rete. Il simulation controller fungerà da intermediario network, ogni pacchetto ricevuto
 da una rete esterna, SC si preouccuperà di inoltrarlo ai relativi droni collegati.
 Per la rete interna, non farà alcuna differenza sapere dove si trovano,
 essendo l'SC l'intermediario che eseguirà le operazioni on-network.
 */
pub struct App<'a> {
    renderer: JoinHandle<()>,
    tx_to_renderer: Sender<RendererCommand>,
    rx_from_renderer: Receiver<RendererResponse>,


    simulation_controller_data: SimulationControllerData,
    stay: bool,
    app_mode: AppMode,
    tick_rate: Duration, // Moving inside settings?

    pub cache: Cache<'a>, // Contains data required by the app to work
    pub settings: Settings, // Setting page containing various config stuff
    pub selected_tab: SelectedTab,
}
impl App<'_>{
    pub fn default(simulation_controller_data: SimulationControllerData) -> Self {
        let ((tx_to_renderer, rx_from_renderer), renderer) = renderer_thread_gen();
        Self{
            renderer,
            tx_to_renderer,
            rx_from_renderer,

            simulation_controller_data,
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

        while self.stay {
            // Listening to channels
            self.simulation_controller_data.handle_channels();
            self.check_topology_status();


            // Terminal drawing and other basic app functions
            self.cache.update_counter();
            self.cache.frame = terminal.get_frame();
            terminal.draw(|f| f.render_widget(&*self, f.area()))?;

            self.handle_events()?;

            // Check if it is time to refresh some data
            if instant.elapsed() > self.tick_rate {
                instant = Instant::now();
                self.tick_refresh();
            }

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
    fn handle_events(&mut self) -> io::Result<()>{
        use std::time::Duration;
        use ratatui::crossterm::event::{self, Event};

        if event::poll(Duration::from_millis(50))? {
            let event = ratatui::crossterm::event::read()?;
            match event {
                Event::Key(key) => {
                    match self.app_mode{
                        AppMode::Normal => {
                            self.handle_event_normalmode(key);
                        },
                        AppMode::Input => {
                            self.handle_event_inputmode(event);
                        },
                        AppMode::Scrolling => {
                            self.handle_event_scrollingmode(key);
                        },
                        AppMode::FileExploring => {
                            self.handle_event_file_exploringmode(event)
                        },
                        _ => {
                            self.handle_event_normalmode(key);
                        }
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
        if self.cache.tab0_data.charts_data.sentmessages.len()>=100{
            self.cache.tab0_data.charts_data.sentmessages.pop();
            self.cache.tab0_data.charts_data.sentmessages.insert(0,rand::rng().random_range(0..230));
        }else{ self.cache.tab0_data.charts_data.sentmessages.insert(0,rand::rng().random_range(0..230)); }

        // We initialize the TOML buffer with the file indicated in the settings
        match self.cache.tab2_data.compiler_buf{
            TOMLmessages::FirstInizialized => {

                self.reload_file_to_buf();

                match parse_and_check_config(&self.settings.toml_path){
                    Ok(_) => {self.cache.tab2_data.compiler_buf = TOMLmessages::Correct},
                    Err(e) => {self.cache.tab2_data.compiler_buf = TOMLmessages::Error(e)}
                }
            }
            _ => ()
        }
        // Nodelist initialized check
        self.check_lists_initialized();


        //self.refresh_cache();
    }
}
impl Widget for &App<'_>{
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

fn renderer_thread_gen()->((Sender<RendererCommand>,Receiver<RendererResponse>), JoinHandle<()>){
    let (tx_cmd, rx_cmd) = mpsc::channel::<RendererCommand>();
    let (tx_rsp, rx_rsp) = mpsc::channel::<RendererResponse>();


    ((tx_cmd,rx_rsp),thread::spawn(move || {
        let mut channels: Vec<(Sender<FileRendererUpdate>, Receiver<ResizeRequest>)> = Vec::new();

        loop {
            // Ascolto il canale main
            if let Ok(request) = rx_cmd.recv(){
                match request {
                    RendererCommand::Stop => {
                        break
                    }
                    RendererCommand::AddChannel((tx,rx)) => {
                        channels.push((tx, rx));
                    },
                    _ => (),
                }
            }

            // Ascolto i canali dei file
            for (tx,rx) in &channels{
                if let Ok(request) = rx.recv() {
                    let rendered_maybe = request.resize_encode();
                    match rendered_maybe{
                        Ok(rendered) => {
                            tx.send(FileRendererUpdate::Redraw(rendered)).unwrap_or(());
                        }
                        Err(err_rendering) => {
                            tx.send(FileRendererUpdate::Error(err_rendering.to_string())).unwrap_or(());
                        }
                    }
                }
            }
        }
    }))
}
