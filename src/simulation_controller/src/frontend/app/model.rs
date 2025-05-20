use std::num::ParseIntError;
use std::str::FromStr;
use ratatui::widgets::{ListItem, ListState};
use tui_textarea::Key::Tab;
use tui_textarea::TextArea;
use crate::frontend::app::ui::tabs::SelectedTab;

use wg_2024::config::Drone as DroneConfig;
use wg_2024::config::Client as ClientConfig;
use wg_2024::config::Server as ServerConfig;
use wg_2024::network::NodeId;
use super::super::app::App;

pub(super) struct Settings{
    pub toml_path: String,
}
pub(super) enum AppMode{
    Normal, Input, Scrolling
}
pub(super) struct Cache<'a> {
    pub cursor: u8, // Cursor to know which menu on-tab is selected.
    pub colors_vec: (usize, Vec<(u8, u8, u8)>), // List of RGB codes colors fading from Yellow to Purple
    pub counter: u8, // Counting cycles, used to update colors in TUI

    pub text_buffer: TextBuffer<'a>,
    pub tab0_data: Tab0, // Data needed by tab0 encapsulated in a struct
    pub tab1_data: Tab1,
    pub tab2_data: Tab2
}
pub(super) struct TextBuffer<'a>{
    pub text_buffer: Vec<String>,
    pub toml_text_area: TextArea<'a>,
    pub inputfirst_text_area: TextArea<'a>,
    pub inputsecond_text_area: TextArea<'a>
}

// Tab structs needed to encapsulate their information and data

/// Tab n.00 data
pub(super) struct Tab0{
    pub selected_chart: usize,
    pub charts_data: ChartsData,
}

/// Tab n.01 data
pub(super) struct Tab1{
    pub nodelist: NodeList,
    pub selected_fn: FnList,
    pub selected_button: u8,
}
pub(super) struct FnList{
    pub(super) selected: u8,
    pub(super) state: ListState,
}

pub(super) struct NodeList {
    pub(super) items: Vec<(Node, bool)>,
    pub(super) selected: u8,
    pub(super) state: ListState,
    pub(super) initialized: bool,
}
impl FnList{
    fn default() -> Self {
        Self{
            selected:0,
            state: ListState::default(),
        }
    }

    /// Has NO Control Against overflow, this fn is used only in specific and controlled cases
    pub(super) fn select_next(&mut self){
            self.selected += 1;
            self.state.select(Some(self.selected as usize));
    }
    pub(super) fn select_prev(&mut self){
        if self.selected > 0 {
            self.selected -= 1;
            self.state.select(Some(self.selected as usize));
        }
    }
    pub(super) fn select_first(&mut self){
        self.selected = 0;
        self.state.select(Some(self.selected as usize));
    }
}
impl NodeList {
    fn default() -> Self {
        Self {
            items: vec![],
            initialized: false,
            selected: 0,
            state: ListState::default(),
        }
    }
    pub(super) fn initialize(&mut self, drones: Vec<DroneConfig>, servers: Vec<ServerConfig>, clients: Vec<ClientConfig>){
                
                let mut nodes = Vec::new();

                for d in drones{
                    nodes.push((Node::Drone(d),true))
                }
                for s in servers{
                    nodes.push((Node::Server(s),true))
                }
                for c in clients{
                    nodes.push((Node::Client(c),true))
                }
                self.items = nodes;
            self.selected= 0;
            self.state.select(Some(self.selected as usize));
            self.initialized= true;
    }
    fn retrieve_nodes(&mut self, drones: Vec<DroneConfig>, servers: Vec<ServerConfig>, clients: Vec<ClientConfig>){
        self.selected = 0;
        let mut nodes = Vec::new();

        for d in drones{
            nodes.push((Node::Drone(d),true))
        }
        for s in servers{
            nodes.push((Node::Server(s),true))
        }
        for c in clients{
            nodes.push((Node::Client(c),true))
        }

        self.items = nodes
    }

    pub(super) fn set_inactive_node(&mut self, id: NodeId){
        for (n,b) in &mut self.items{
            match n{
                Node::Client(c) => {
                    if c.id == id{
                        *b = false;
                    }
                }
                Node::Server(s) => {
                    if s.id == id{
                        *b = false;
                    }
                }
                Node::Drone(d) => {
                    if d.id == id{
                        *b = false;
                    }
                }
            }
        }
    }
    pub(super) fn select_next(&mut self){
        if self.selected + 1 < (self.items.len()) as u8{
            self.selected += 1;
            self.state.select(Some(self.selected as usize));
        }
    }
    pub(super) fn select_prev(&mut self){
        if self.selected > 0 {
            self.selected -= 1;
            self.state.select(Some(self.selected as usize));
        }
    }
    pub(super) fn select_first(&mut self){
        self.selected = 0;
        self.state.select(Some(self.selected as usize));
    }
    fn select_last(&mut self){
        self.selected = self.items.len() as u8;
        self.state.select(Some(self.selected as usize));
    }
}
#[derive(Debug)]
pub(super) enum Node{
    Client(ClientConfig), Server(ServerConfig), Drone(DroneConfig)
}


/// Tab n.02 data
pub(super) struct Tab2{
    pub compiler_buf: TOMLmessages,
}
/// Tab n.03 data
pub(super) struct Tab3{

}

pub(super) struct ChartsData{ // Each CONST sec data, NOT TOTAL
    pub droppedmessages: Vec<u64>, // We will show max n gauges
    pub sentmessages: Vec<u64>,
}
impl Settings{
    pub(super) fn load()->Self{
        Self{
            toml_path:"config.toml".to_string(),
        }
    }
}
impl<'a> TextBuffer<'_>{
    fn default()->Self{
        Self{
            text_buffer: Vec::new(),
            toml_text_area: Default::default(),
            inputfirst_text_area: Default::default(),
            inputsecond_text_area: Default::default(),
        }
    }
    /// 1 - Get ID ( u8 ) on the first buffer
    /// 2 - Get ID and Message ( String )
    fn get_parsed_data(&self, n:u8)->Result<ParsedBuffer, String>{
        match n{
            1 => {
                match self.inputfirst_text_area.clone().into_lines().first(){
                    None => {
                        Err("Nothing to parse".to_string())
                    }
                    Some(mystr) => {
                        match u8::from_str(mystr){
                            Ok(x) => {Ok(ParsedBuffer::OneParsed(x))}
                            Err(x) => {Err(format!("Cannot parse an u8 [{:?}]", x))}
                        }
                    }
                }
            },
            2 => {
                match self.inputfirst_text_area.clone().into_lines().first(){
                    None => {
                        Err("Nothing to parse on I buffer".to_string())
                    }
                    Some(mystr) => {
                        match u8::from_str(mystr){
                            Ok(x) => {
                                // Now we parse the 2nd one
                                match self.inputfirst_text_area.clone().into_lines().first(){
                                    None => {
                                        Err("Nothing to parse on II buffer".to_string())
                                    }
                                    Some(mystr2) => {
                                        Ok(ParsedBuffer::TwoParsed(x, mystr2.to_string()))
                                    }
                                }
                            }
                            Err(x) => {Err(format!("Cannot parse an u8 on I buffer [{:?}]", x))}
                        }
                    }
                }
            },
            _ => Err(format!("Error requesting parsed data [{:?} is not a valid option]", n))
        }
    }
}
enum ParsedBuffer {
    OneParsed(u8), TwoParsed(u8, String)
}
impl ChartsData {
    fn default()->Self{
        Self{
            droppedmessages:Vec::new(),
            sentmessages:Vec::new(),
        }
    }
    fn plusdroppedmessages(){}
    fn plussentmessages(){}
    fn reset(){}
}

// TAB IMPLS
/// Tab0 Impl fns
impl Tab0 {
    fn default() -> Tab0 {
        Tab0 {
            charts_data:ChartsData::default(),
            selected_chart:0,
        }
    }
    fn next_chart(&mut self){
        if self.selected_chart != 2{
            self.selected_chart+=1;
        }
    }
    fn prev_chart(&mut self){
        if self.selected_chart != 0{
            self.selected_chart -= 1;
        }
    }
}

/// Tab1 Impl fns
impl Tab1{
    fn default()->Self{
        Self{
            nodelist: NodeList::default(),
            selected_fn: FnList::default(),
            selected_button: 0,
        }
    }
}
/// Tab2 Impl fns
impl Tab2{
    fn default()->Self{
        Self{
            compiler_buf:TOMLmessages::FirstInizialized,
        }
    }
}
pub(super) enum TOMLmessages{
    Correct,
    Error(String),
    FirstInizialized
}
impl<'a> Cache<'a> {
    pub(super) fn default() -> Cache<'a> {
        Cache {
            text_buffer:TextBuffer::default(),
            cursor:0,
            tab0_data: Tab0::default(),
            tab1_data: Tab1::default(),
            tab2_data: Tab2::default(),
            colors_vec: {
                let vec = vec![(253, 185, 39),
                               (249, 181, 40),
                               (246, 178, 42),
                               (242, 175, 44),
                               (239, 172, 46),
                               (235, 169, 48),
                               (232, 166, 50),
                               (229, 163, 52),
                               (225, 160, 54),
                               (222, 157, 55),
                               (218, 154, 57),
                               (215, 151, 59),
                               (211, 148, 61),
                               (208, 145, 63),
                               (205, 142, 65),
                               (201, 139, 67),
                               (198, 136, 69),
                               (194, 133, 70),
                               (191, 130, 72),
                               (187, 127, 74),
                               (184, 124, 76),
                               (181, 121, 78),
                               (177, 118, 80),
                               (174, 115, 82),
                               (170, 112, 84),
                               (167, 109, 85),
                               (163, 106, 87),
                               (160, 103, 89),
                               (157, 100, 91),
                               (153, 97, 93),
                               (150, 94, 95),
                               (146, 91, 97),
                               (143, 88, 99),
                               (139, 85, 100),
                               (136, 82, 102),
                               (133, 79, 104),
                               (129, 76, 106),
                               (126, 73, 108),
                               (122, 70, 110),
                               (119, 67, 112),
                               (115, 64, 114),
                               (112, 61, 115),
                               (109, 58, 117),
                               (105, 55, 119),
                               (102, 52, 121),
                               (98, 49, 123),
                               (95, 46, 125),
                               (91, 43, 127),
                               (88, 40, 129),
                               (85, 37, 131),
                               (88, 40, 129),
                               (91, 43, 127),
                               (95, 46, 125),
                               (98, 49, 123),
                               (102, 52, 121),
                               (105, 55, 119),
                               (109, 58, 117),
                               (112, 61, 115),
                               (115, 64, 114),
                               (119, 67, 112),
                               (122, 70, 110),
                               (126, 73, 108),
                               (129, 76, 106),
                               (133, 79, 104),
                               (136, 82, 102),
                               (139, 85, 100),
                               (143, 88, 99),
                               (146, 91, 97),
                               (150, 94, 95),
                               (153, 97, 93),
                               (157, 100, 91),
                               (160, 103, 89),
                               (163, 106, 87),
                               (167, 109, 85),
                               (170, 112, 84),
                               (174, 115, 82),
                               (177, 118, 80),
                               (181, 121, 78),
                               (184, 124, 76),
                               (187, 127, 74),
                               (191, 130, 72),
                               (194, 133, 70),
                               (198, 136, 69),
                               (201, 139, 67),
                               (205, 142, 65),
                               (208, 145, 63),
                               (211, 148, 61),
                               (215, 151, 59),
                               (218, 154, 57),
                               (222, 157, 55),
                               (225, 160, 54),
                               (229, 163, 52),
                               (232, 166, 50),
                               (235, 169, 48),
                               (239, 172, 46),
                               (242, 175, 44),
                               (246, 178, 42),
                               (249, 181, 40)];
                (vec.len(),vec)
            },
            counter: 0,
        }
    }
    pub(super) fn update_counter(&mut self) {
        self.counter = self.counter.wrapping_add(1);
    }
}