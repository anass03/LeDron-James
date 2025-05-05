/*
In this module we will handle the specific events related to single tabs.
Our event handler from main App file, will delegate these methods those said events.
 */
use ratatui::crossterm::event::{KeyEvent, KeyCode};
use super::super::App;
impl App<'_>{
    pub(super) fn handle_event_tab0(&mut self,key: KeyEvent){

    }
    pub(super) fn handle_event_tab1(&mut self,key: KeyEvent){}
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
                        self.input_mode = true;
                    },
                    _ => (),
                }
            },
            _ => (),
        }
    }
    pub(super) fn handle_event_tab3(&mut self,key: KeyEvent){}
}