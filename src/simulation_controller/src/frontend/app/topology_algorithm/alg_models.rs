use std::collections::HashMap;
use wg_2024::network::NodeId;
use crate::frontend::app::topology_algorithm::vectors::Vector;

pub struct TopologyAlgorithm{
    pub status: TopologyStatus,
    // Node and its coordinates on the map
    pub map: HashMap<NodeId, Vector>,


    // How many Dijkstra steps from NodeID to reach any other node
    pub nodes_dijkstra: Vec<(NodeId, HashMap<NodeId, u8>)>,

    // Nodes and their resultant vector that needs to get applied
    pub nodes_resultant_vec: HashMap<NodeId, Vector>,

    // All positions score, the smaller the better!
    // How do we compute it? We get the avg of all the resultant magnitudes.
    avg_positioning_score: u16,

    // More than K fatal errors and algorithm will stop trying to converge ( and re-run? idk )
    counter_errors: u8,
}


enum TopologyStatus{
    ToInitialize, Computing, Converged
}

impl TopologyAlgorithm{
    pub fn default()-> Self{
        Self{
            status: TopologyStatus::ToInitialize,
            nodes_dijkstra: vec![],
            nodes_resultant_vec: Default::default(),
            map: Default::default(),
            counter_errors: 0,
            avg_positioning_score: 0,
        }
    }
    pub fn initialize(&mut self){
        self = &mut Self::default();
    }
    /// Function called when finally the algorithms converges, so when the avg positioning score
    pub fn computed(&mut self){
        self.status = TopologyStatus::Converged;

        self.counter_errors = 0;
        self.nodes_dijkstra = vec![];
        self.counter_errors = 0;
    }
}