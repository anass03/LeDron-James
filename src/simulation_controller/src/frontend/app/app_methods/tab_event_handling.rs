/*
In this module we will handle the specific events related to single tabs.
Our event handler from main App file, will delegate these methods those said events.
 */
use ratatui::crossterm::event::{KeyEvent, KeyCode};
use crate::frontend::app::model::AppMode;
use super::super::App;
impl App<'_>{
    pub(super) fn handle_event_tab0(&mut self,key: KeyCode){
        match key {
            KeyCode::Up => {
                if self.cache.cursor!=0{
                    //println!("zio");
                    self.cache.cursor -= 1
                }
            },
            KeyCode::Down => {
                if self.cache.cursor!=2{
                    //println!("zio");
                    self.cache.cursor += 1
                }
            },
            KeyCode::Enter => {
                match self.cache.cursor{
                    0 => {

                    },
                    1 => {
                        // TODO! Log scroll mode
                        /*
                            Add a log scroll mode
                            up or down to scroll menu, esc to exit

                            Menu OPTION to:
                            - Save logs out an external file


                         */

                        self.cache.tab0_data.log_scroll_offset += 1;
                    },
                    _ => (),
                }
            }
            _ => (),
        }
    }
    pub(super) fn handle_event_tab1(&mut self,key: KeyCode){
        match self.cache.cursor{
            0 => {
                match key{
                    KeyCode::Up => {
                        self.cache.tab1_data.nodelist.select_prev()
                    },
                    KeyCode::Down => {
                        self.cache.tab1_data.nodelist.select_next()
                    },
                    KeyCode::Enter => {
                        // Doing nothing or changing cursor to +=1
                        self.cache.cursor += 1;
                    },
                    // Missing KeyCodes to view the first one / last one, still gotta decide!
                    _ => () // Do nothing
                }
            },
            1 => {

            }
            _ => {}
        }
    }
    pub(super) fn handle_event_tab2(&mut self,key: KeyCode){
        match key {
            KeyCode::Up => {
                if self.cache.cursor!=0{
                    //println!("zio");
                    self.cache.cursor -= 1
                }
            },
            KeyCode::Down => {
                if self.cache.cursor!=2{
                    //println!("zio");
                    self.cache.cursor += 1
                }
            }
            KeyCode::Enter => {
                match self.cache.cursor {
                    0 => {
                        self.app_mode = AppMode::Input;
                    },
                    _ => (),
                }
            },
            _ => (),
        }
    }
    pub(super) fn handle_event_tab3(&mut self,key: KeyEvent){}
}