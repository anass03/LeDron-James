use std::collections::HashMap;
use wg_2024::network::NodeId;
use crate::frontend::app::ui::topology_algorithm::vectors::Vector;

struct TopologyAlgorithm{
    pub status: TopologyStatus,
    // How many Dijkstra steps from NodeID to reach any other node
    pub nodes_dijkstra: Vec<(NodeId, HashMap<NodeId, u8>)>,

    // Node and its coordinates on the map
    pub map: HashMap<NodeId, Vector>,

    // More than K fatal errors and algorithm will stop trying to converge
    counter_errors: u8,
}


enum TopologyStatus{
    ToInitialize, Computing, Converged
}

impl TopologyAlgorithm{
    fn default()-> Self{
        Self{
            status: TopologyStatus::ToInitialize,
            nodes_dijkstra: vec![],
            map: Default::default(),
            counter_errors: 0,
        }
    }
}