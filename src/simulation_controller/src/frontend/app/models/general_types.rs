use tui_textarea::TextArea;
use ratatui::widgets::ListState;
use wg_2024::config::{Client as ClientConfig, Drone as DroneConfig, Server as ServerConfig};
use wg_2024::network::NodeId;
use std::str::FromStr;

pub(in crate::frontend::app) struct TextBuffer<'a>{
    pub text_buffer: Vec<String>,
    pub toml_text_area: TextArea<'a>,
    pub chat_text_area: TextArea<'a>,
    pub inputfirst_text_area: TextArea<'a>,
    pub inputsecond_text_area: TextArea<'a>,
    pub inputthird_text_area: TextArea<'a>
}

pub(in crate::frontend::app) struct FnList{
    pub(in crate::frontend::app) selected: u8,
    pub(in crate::frontend::app) state: ListState,
}

pub(in crate::frontend::app) struct NodeList {
    pub(in crate::frontend::app) items: Vec<(Node, bool)>,
    pub(in crate::frontend::app) selected: u8,
    pub(in crate::frontend::app) state: ListState,
    pub(in crate::frontend::app) initialized: bool,
}

impl FnList{
    pub(crate) fn default() -> Self {
        Self{
            selected:0,
            state: ListState::default(),
        }
    }

    pub(in crate::frontend::app) fn select_next(&mut self){
            self.selected += 1;
            self.state.select(Some(self.selected as usize));
    }
    pub(in crate::frontend::app) fn select_prev(&mut self){
        if self.selected > 0 {
            self.selected -= 1;
            self.state.select(Some(self.selected as usize));
        }
    }
    pub(in crate::frontend::app) fn select_first(&mut self){
        self.selected = 0;
        self.state.select(Some(self.selected as usize));
    }
}

impl NodeList {
    pub(crate) fn default() -> Self {
        Self {
            items: vec![],
            initialized: false,
            selected: 0,
            state: ListState::default(),
        }
    }
    pub(in crate::frontend::app) fn initialize(&mut self, drones: Vec<DroneConfig>, servers: Vec<ServerConfig>, clients: Vec<ClientConfig>){

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

    pub(in crate::frontend::app) fn set_inactive_node(&mut self, id: NodeId){
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
    pub(in crate::frontend::app) fn select_next(&mut self){
        if self.selected + 1 < (self.items.len()) as u8{
            self.selected += 1;
            self.state.select(Some(self.selected as usize));
        }
    }
    pub(in crate::frontend::app) fn select_prev(&mut self){
        if self.selected > 0 {
            self.selected -= 1;
            self.state.select(Some(self.selected as usize));
        }
    }
    pub(in crate::frontend::app) fn select_first(&mut self){
        self.selected = 0;
        self.state.select(Some(self.selected as usize));
    }
    fn select_last(&mut self){
        self.selected = self.items.len() as u8;
        self.state.select(Some(self.selected as usize));
    }
}

#[derive(Debug)]
pub(in crate::frontend::app) enum Node{
    Client(ClientConfig), Server(ServerConfig), Drone(DroneConfig)
}

impl<'a> TextBuffer<'_>{
    pub(crate) fn default() ->Self{
        Self{
            text_buffer: Vec::new(),
            toml_text_area: Default::default(),
            chat_text_area: Default::default(),
            inputfirst_text_area: Default::default(),
            inputsecond_text_area: Default::default(),
            inputthird_text_area: Default::default()
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

pub(in crate::frontend::app) enum TOMLmessages{
    Correct,
    Error(String),
    FirstInizialized
}