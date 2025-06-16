use std::collections::HashMap;
use ratatui::crossterm::terminal::DisableLineWrap;
use wg_2024::network::NodeId;
use crate::backend::simulation_controller::NetNode;
use crate::frontend::app::topology_algorithm::vectors::Vector;

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub enum DisplayedNodeStatus{
    Idle,
    Selected,
    Inactive
}
#[derive(Copy, Clone, Debug)]
pub struct DisplayedNode{
    pub node: NetNode,
    pub position: Vector,
    pub selected: DisplayedNodeStatus,
    pub inactive: bool,
}
impl DisplayedNode{
    pub fn new(node: NetNode) -> DisplayedNode {
        DisplayedNode{
            node,
            position: Vector::new(0.0,0.0),
            selected: DisplayedNodeStatus::Idle, inactive: false
        }
    }
    pub fn new_with_position(node: NetNode, position: Vector) -> DisplayedNode {
        let mut ret = DisplayedNode::new(node);
        ret.position = position;
        ret

    }
    pub fn get_id(&self) -> NodeId{
        self.node.id().clone()
    }
}

pub struct TopologyAlgorithm{
    pub status: TopologyStatus,
    // Node and its coordinates on the map
    pub map: HashMap<NodeId, DisplayedNode>,
    // Hashmap of all the links ( from <-> to (links are bidirectional))
    pub map_links: Vec<(NodeId,NodeId)>,


    // How many Dijkstra steps from NodeID to reach any other node
    pub nodes_dijkstra: Vec<(NodeId, HashMap<NodeId, u8>)>,

    // Nodes and their resultant vector that needs to get applied
    pub nodes_resultant_vec: HashMap<NodeId, Vector>,

    // All positions score, the smaller the better!
    // How do we compute it? We get the avg of all the resultant magnitudes.
    pub avg_positioning_score: f32,

    // More than K fatal errors and algorithm will stop trying to converge ( and re-run? idk )
    counter_errors: u8,
}


pub(crate) enum TopologyStatus{
    Stopped, ToInitialize, Computing, Converged
}

impl TopologyAlgorithm{
    pub fn default()->Self{
        Self{
            status: TopologyStatus::ToInitialize,
            map: Default::default(),
            map_links: vec![],
            nodes_dijkstra: vec![],
            nodes_resultant_vec: Default::default(),
            avg_positioning_score: 0.0,
            counter_errors: 0,
        }
    }
    pub fn new(map_links:Vec<(NodeId,NodeId)>) -> Self{
        Self{
            status: TopologyStatus::ToInitialize,
            nodes_dijkstra: vec![],
            nodes_resultant_vec: Default::default(),
            map: Default::default(),
            counter_errors: 0,
            avg_positioning_score: 0.0,
            map_links,
        }
    }
    pub fn initialize(&mut self, links:Vec<(NodeId,NodeId)>){
        *self = Self::new(links);
    }
    /// Function called when finally the algorithms converges, so when the avg positioning score
    pub fn computed(&mut self){
        self.status = TopologyStatus::Converged;

        self.counter_errors = 0;
        self.nodes_dijkstra = vec![];
    }
    pub fn increase_counter(&mut self, num:u8){
        self.counter_errors += num;
    }

    pub fn get_all_links(&mut self){

    }
}