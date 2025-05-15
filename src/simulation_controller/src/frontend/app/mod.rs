mod app_methods;
mod ui;
mod model;

use ui::tabs::{render_footer, render_inner, render_top};
use model::Cache;

use ratatui::{
    buffer::Buffer,
    DefaultTerminal,
    layout::{
        Rect,Layout
    },
    widgets::Widget
};

use std::{io, time};
use std::time::Duration;
use time::Instant;
use rand::Rng;
use initializer_utils::utils::{parse_config};
use crate::frontend::app::model::{Settings, TOMLmessages};
use crate::frontend::app::ui::tabs::SelectedTab;
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
    stay: bool,
    input_mode: bool,
    tick_rate: Duration, // Moving inside settings?

    pub cache: Cache<'a>, // Contains data required by the app to work
    pub settings: Settings, // Setting page containing various config stuff
    pub selected_tab: SelectedTab,
}
impl App<'_>{
    pub fn default() -> Self {
        Self{
            stay:true,
            input_mode:false,
            tick_rate:Duration::from_millis(100),

            settings: Settings::load(),
            selected_tab: SelectedTab::default(),
            cache: Cache::default()
        }
    }
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()>{
        let mut instant = Instant::now();
        while self.stay {
            self.cache.update_counter();
            terminal.draw(|f| f.render_widget(&*self, f.area()))?;
            self.handle_events()?;
            if self.stay == false{
                // QUITTING MODE: We should ask again to be sure if the user wants to quit
                // To do: Design a Quitting page with loading bar

                // 1 - ReCALLING terminal.draw(|f| f.render_widget(&*self, f.area()))?;
                // GOTTA MODIFY RENDER_WIDGET to DRAW frame accordly to what was being shown before and adding a tab over it

                // 2 - ReCALLING self.handle_events() that will process only 2 inputs (arrow up / down) to choose YES or NO
                // on the "Are you sure to quit the app" tab
                // GOTTA MODIFY HANDLE_EVENTS to HANDLE QUITTING MODE
             }

            // Check if it is time to refresh some data
            if instant.elapsed() > self.tick_rate {
                instant = Instant::now();
                self.tick_refresh();
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
                    if self.input_mode{
                        self.handle_event_inputmode(event);
                    }else{
                        self.handle_event_normalmode(key);
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
    fn tick_refresh(&mut self) {
        if self.cache.tab0_data.charts_data.sentmessages.len()>=100{
            self.cache.tab0_data.charts_data.sentmessages.pop();
            self.cache.tab0_data.charts_data.sentmessages.insert(0,rand::rng().random_range(0..230));
        }else{ self.cache.tab0_data.charts_data.sentmessages.insert(0,rand::rng().random_range(0..230)); }

        match self.cache.tab2_data.compiler_buf{
            TOMLmessages::FirstInizialized => {

                self.reload_file_to_buf();

                match parse_config(self.settings.toml_path.as_str()){
                    Ok(_) => {self.cache.tab2_data.compiler_buf = TOMLmessages::Correct},
                    Err(e) => {self.cache.tab2_data.compiler_buf = TOMLmessages::Error(e)}
                }

            }
            _ => ()
        }
        //self.refresh_cache();
    }
}

impl Widget for &App<'_>{
    fn render(mut self, area: Rect, buf: &mut Buffer)
    {
        use ratatui::layout::Constraint::{Length, Min};
        use ui::tabs::{render_top,render_inner,render_footer};

        let vertical = Layout::vertical([Length(3), Min(0), Length(3)]);
        let [top_area, inner_area, footer_area] = vertical.areas(area);

        render_top(self, top_area, buf);
        render_inner(self, inner_area, buf);
        render_footer(self, footer_area, buf);
    }
}
