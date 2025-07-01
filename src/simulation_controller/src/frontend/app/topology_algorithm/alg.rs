use std::collections::HashMap;
use ratatui::layout::Rect;
use wg_2024::network::NodeId;
use crate::backend::sc_models::LogType;
use crate::backend::simulation_controller::{NetNode, SimulationControllerData};
use crate::frontend::app::App;
use crate::frontend::app::topology_algorithm::vectors::{generate_force_vec, Vector};
use crate::frontend::app::topology_algorithm::alg_models::{DisplayedNode, TopologyAlgorithm, TopologyStatus};

impl App<'_>{
    /// Populate the TopologyAlgorithm.map_links field from the running_network,
    /// inserting each undirected edge exactly once.
    pub fn populate_topology_links(&mut self)-> Vec<(NodeId, NodeId)>{
        use std::collections::HashSet;
        // Collect unique, undirected links
        let mut links = HashSet::new();
        for (net_node, neighbors) in &self.simulation_controller_data.running_network {
            // Extract the NodeId from NetNode
            let id = match net_node {
                NetNode::Drone(id) | NetNode::Client(id) | NetNode::Server(id) => *id,
            };
            for &neighbor in neighbors {
                // Order the pair to avoid duplicates (min, max)
                let (a, b) = if id <= neighbor { (id, neighbor) } else { (neighbor, id) };
                links.insert((a, b));
            }
        }
        links.into_iter().collect()
    }
    pub fn initialize_node_map(&self, top_alg: &mut TopologyAlgorithm ,id:NodeId){
        top_alg.status = TopologyStatus::Computing;
        fn create_map(sim: &SimulationControllerData, top_alg:&mut TopologyAlgorithm, connected: &Vec<NodeId>){
            for id in connected{
                top_alg.map.insert(*id, DisplayedNode::new_with_position({
                                                                             if sim.running_network.contains_key(&NetNode::Drone(*id)){
                                                                                 NetNode::Drone(*id)
                                                                             }else if sim.running_network.contains_key(&NetNode::Server(*id)){
                                                                                 NetNode::Server(*id)
                                                                             }else{
                                                                                 NetNode::Client(*id)
                                                                             }
                                                                         }, Vector::random(100,80)));
            }
        }
        match &self.simulation_controller_data.running_network.get(&NetNode::Drone(id)){
            None => {
                match &self.simulation_controller_data.running_network.get(&NetNode::Server(id)){
                    None => {
                        match &self.simulation_controller_data.running_network.get(&NetNode::Client(id)) {
                            None => (),
                            Some(vec) => {
                                create_map(&self.simulation_controller_data, top_alg,vec);
                            }
                        }
                    }
                    Some(vec) => {
                        create_map(&self.simulation_controller_data, top_alg,vec);
                    }
                }
            }
            Some(vec) => {
                create_map(&self.simulation_controller_data, top_alg,vec);
            }
        }
        for (x,y) in &self.simulation_controller_data.topology_algorithm_data.map_links{
            if x.eq(&id){
                top_alg.map_links.push((*x, *y));
            }else if y.eq(&id){
                top_alg.map_links.push((*x, *y));
            }
        }

    }
    pub fn check_topology_status(&mut self){
        match self.simulation_controller_data.topology_algorithm_data.status{
            TopologyStatus::Stopped => {
                // Do NUN
            }
            TopologyStatus::ToInitialize => {
                // Initialize each necessary field
                self.topology_status_initialize();
            }
            TopologyStatus::Computing => {
                // Keep going generating forces and iterating algorithm
                self.simulation_controller_data.topology_algorithm_data.check_if_nan();
                self.topology_status_computing();
            }
            TopologyStatus::Converged => {
                // Do nothing
            }
        }
    }
    fn topology_status_initialize(&mut self){
        let mut top_alg = &mut self.simulation_controller_data.topology_algorithm_data;
        top_alg.status = TopologyStatus::Computing;
        // Generate random positions for network
        for (node,_) in self.simulation_controller_data.running_network.clone(){
            top_alg.map.insert(node.id().clone(), DisplayedNode::new_with_position(node, Vector::random(100,80)));
        }
    }
    fn topology_status_computing(&mut self){
        match self.generate_all_forces(){
            Ok(hsh) => {
                for element in hsh{
                    self.simulation_controller_data.topology_algorithm_data.nodes_resultant_vec.insert(element.0,element.1);
                }
            }
            Err(str) => {
                self.simulation_controller_data.topology_algorithm_data.increase_counter(1);
                self.simulation_controller_data.logs.push_sc_log(LogType::ERROR, str);
            }
        }
        match self.apply_all_forces(){
            Ok(u) => {
                self.simulation_controller_data.topology_algorithm_data.increase_counter(u);
            }
            Err(str) => {
                self.simulation_controller_data.topology_algorithm_data.increase_counter(1);
                self.simulation_controller_data.logs.push_sc_log(LogType::ERROR, str);
            }
        }
        self.compute_and_set_avg_magnitude();

        if self.simulation_controller_data.topology_algorithm_data.avg_positioning_score < 1.5 {
            self.simulation_controller_data.topology_algorithm_data.status = TopologyStatus::Converged;
            self.simulation_controller_data.topology_algorithm_data.computed();
        }
    }
    fn generate_all_forces(&self)->Result<HashMap<NodeId, Vector>,String>{
        let mut ret_hash:HashMap<NodeId, Vector> = HashMap::new();

        for (node_id, displayable) in &self.simulation_controller_data.topology_algorithm_data.map{
            match displayable.clone().node{
                NetNode::Drone(id) => {
                    let res = self.generate_fundamental_force(id, displayable.position)?;
                    ret_hash.insert(id, res);
                }
                NetNode::Client(id) => {
                    let res = self.generate_fundamental_force(id, displayable.position)?;
                    ret_hash.insert(id, res);
                }
                NetNode::Server(id) => {
                    let res = self.generate_fundamental_force(id, displayable.position)?;
                    ret_hash.insert(id, res);
                }
            }
         }

        Ok(ret_hash)
    }
    fn apply_all_forces(&mut self)->Result<u8,String>{
        let mut error_counter:u8 = 0;
        for (ids,res) in &self.simulation_controller_data.topology_algorithm_data.nodes_resultant_vec{
            match self.simulation_controller_data.topology_algorithm_data.map.get_mut(ids){
                Some(displayable) => {
                   displayable.position = displayable.position.add(&res.scale(0.025));
                },
                None => {
                    error_counter += 1;
                }
            }
        }
        Ok(error_counter)
    }
    fn compute_and_set_avg_magnitude(&mut self){
        let mut sum_magnitude = 0.0;
        let mut counter: u16 = 0;
        for (_,vec) in self.simulation_controller_data.topology_algorithm_data.nodes_resultant_vec.clone(){
            sum_magnitude += vec.magnitude();
            counter += 1;
        }
        self.simulation_controller_data.topology_algorithm_data.avg_positioning_score = sum_magnitude/counter as f32;
    }

}


