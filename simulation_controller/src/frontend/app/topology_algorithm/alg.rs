use crate::backend::sc_models::NetNode;
use crate::backend::sc_models::{LogType, RendererCommand};
use crate::backend::simulation_controller::SimulationControllerData;
use crate::frontend::app::models::general_types::Node;
use crate::frontend::app::topology_algorithm::alg_models::{
    DisplayedNode, TopologyAlgorithm, TopologyStatus,
};
use crate::frontend::app::topology_algorithm::vectors::{generate_force_vec, Vector};
use crate::frontend::app::App;
use log::{error, info, warn};
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use wg_2024::network::NodeId;

impl App<'_> {
    /// Populate the TopologyAlgorithm.map_links field from the running_network,
    /// inserting each undirected edge exactly once.
    pub fn populate_topology_links(&mut self) -> Vec<(NodeId, NodeId)> {
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
                let (a, b) = if id <= neighbor {
                    (id, neighbor)
                } else {
                    (neighbor, id)
                };
                links.insert((a, b));
            }
        }
        links.into_iter().collect()
    }

    /// Single Nodes Methods for popups
    pub fn initialize_node_pop_map(&mut self, id: NodeId) {
        fn create_map(
            id: NodeId,
            sim: &SimulationControllerData,
            top_alg: &mut TopologyAlgorithm,
            connected: &Vec<NodeId>,
        ) {
            top_alg.map.insert(
                id,
                DisplayedNode::new_with_position(
                    {
                        if sim.running_network.contains_key(&NetNode::Drone(id)) {
                            NetNode::Drone(id)
                        } else if sim.running_network.contains_key(&NetNode::Server(id)) {
                            NetNode::Server(id)
                        } else {
                            NetNode::Client(id)
                        }
                    },
                    Vector::random(100, 80),
                ),
            );
            for n_id in connected {
                top_alg.map.insert(
                    *n_id,
                    DisplayedNode::new_with_position(
                        {
                            if sim.running_network.contains_key(&NetNode::Drone(*n_id)) {
                                NetNode::Drone(*n_id)
                            } else if sim.running_network.contains_key(&NetNode::Server(*n_id)) {
                                NetNode::Server(*n_id)
                            } else {
                                NetNode::Client(*n_id)
                            }
                        },
                        Vector::random(100, 80),
                    ),
                );
            }
        }

        // First, determine the node type from the UI list
        let node_type = {
            let mut found_type = NetNode::Client(id); // fallback
            for (x, _, _) in &self.cache.tab1_data.net_node_list.items {
                match x {
                    Node::Client(cl) if cl.id == id => {
                        found_type = NetNode::Client(id);
                        break;
                    }
                    Node::Server(sr) if sr.id == id => {
                        found_type = NetNode::Server(id);
                        break;
                    }
                    Node::Drone(dr) if dr.id == id => {
                        found_type = NetNode::Drone(id);
                        break;
                    }
                    _ => continue,
                }
            }
            found_type
        };

        let mut maybe_alg = None;
        for (x, _y, z) in &mut self.cache.tab1_data.net_node_list.items {
            match x {
                Node::Client(cl) => {
                    if cl.id == id {
                        maybe_alg = Some(z);
                        break;
                    }
                }
                Node::Server(sr) => {
                    if sr.id == id {
                        maybe_alg = Some(z);
                        break;
                    }
                }
                Node::Drone(dr) => {
                    if dr.id == id {
                        maybe_alg = Some(z);
                        break;
                    }
                }
            }
        }
        match maybe_alg {
            None => {
                warn!(target: "topology", "[TOPOLOGY] - Node {} not found in netnode_list.items", id);
            }
            Some(top_alg) => {
                top_alg.status = TopologyStatus::Computing;
                for (x, y) in &self
                    .simulation_controller_data
                    .topology_algorithm_data
                    .map_links
                {
                    if x.eq(&id) {
                        top_alg.map_links.push((*x, *y));
                    } else if y.eq(&id) {
                        top_alg.map_links.push((*x, *y));
                    }
                }

                // Check if node exists in running_network
                let node_in_network = self
                    .simulation_controller_data
                    .running_network
                    .get(&NetNode::Drone(id))
                    .is_some()
                    || self
                        .simulation_controller_data
                        .running_network
                        .get(&NetNode::Server(id))
                        .is_some()
                    || self
                        .simulation_controller_data
                        .running_network
                        .get(&NetNode::Client(id))
                        .is_some();

                if !node_in_network {
                    warn!(target: "topology", "[TOPOLOGY] - Node {} not found in running_network, adding as standalone node", id);
                    // Add the node to the map even if it's not in running_network
                    top_alg.map.insert(
                        id,
                        DisplayedNode::new_with_position(node_type, Vector::random(100, 80)),
                    );
                } else {
                    // Original logic for connected nodes
                    match &self
                        .simulation_controller_data
                        .running_network
                        .get(&NetNode::Drone(id))
                    {
                        None => {
                            match &self
                                .simulation_controller_data
                                .running_network
                                .get(&NetNode::Server(id))
                            {
                                None => {
                                    match &self
                                        .simulation_controller_data
                                        .running_network
                                        .get(&NetNode::Client(id))
                                    {
                                        None => {
                                            warn!(target: "topology", "[TOPOLOGY] - Node {} found in running_network but no connections", id);
                                            // Add the node even if it has no connections
                                            top_alg.map.insert(
                                                id,
                                                DisplayedNode::new_with_position(
                                                    node_type,
                                                    Vector::random(100, 80),
                                                ),
                                            );
                                        }
                                        Some(vec) => {
                                            create_map(
                                                id,
                                                &self.simulation_controller_data,
                                                top_alg,
                                                vec,
                                            );
                                        }
                                    }
                                }
                                Some(vec) => {
                                    create_map(id, &self.simulation_controller_data, top_alg, vec);
                                }
                            }
                        }
                        Some(vec) => {
                            create_map(id, &self.simulation_controller_data, top_alg, vec);
                        }
                    }
                }

                match &self.simulation_controller_data.tx_to_renderer {
                    None => {}
                    Some(tx_cmd) => {
                        match tx_cmd.send(RendererCommand::ComputeSetOfTopologies(vec![(
                            id,
                            top_alg.clone(),
                        )])) {
                            Ok(_) => {}
                            Err(_) => {}
                        }
                    }
                }

                // Debug logging for map size
                info!(target: "topology", "[TOPOLOGY] - Node {} map initialized with {} nodes", id, top_alg.map.len());
                if top_alg.map.is_empty() {
                    warn!(target: "topology", "[TOPOLOGY] - Node {} map is empty! running_network size: {}", id, self.simulation_controller_data.running_network.len());
                }
            }
        }
    }

    /// Check Main Topology Status
    pub fn check_main_topology_status(&mut self) {
        match self
            .simulation_controller_data
            .topology_algorithm_data
            .status
        {
            TopologyStatus::Stopped => {
                // Do NUN
            }
            TopologyStatus::ToInitialize => {
                // Initialize each necessary field
                self.topology_main_status_initialize();
            }
            TopologyStatus::Computing => {
                // Keep going generating forces and iterating algorithm
                self.topology_main_status_computing();
                self.simulation_controller_data
                    .topology_algorithm_data
                    .check_if_nan();
            }
            TopologyStatus::Converged => {
                // Do nothing
            }
        }
    }
    fn topology_main_status_initialize(&mut self) {
        let top_alg = &mut self.simulation_controller_data.topology_algorithm_data;
        top_alg.status = TopologyStatus::Computing;
        // Generate random positions for network
        for (node, _) in self.simulation_controller_data.running_network.clone() {
            top_alg.map.insert(
                node.id().clone(),
                DisplayedNode::new_with_position(node, Vector::random(100, 80)),
            );
        }
    }
    fn topology_main_status_computing(&mut self) {
        match self.generate_all_forces() {
            Ok(hsh) => {
                for element in hsh {
                    self.simulation_controller_data
                        .topology_algorithm_data
                        .nodes_resultant_vec
                        .insert(element.0, element.1);
                }
            }
            Err(str) => {
                self.simulation_controller_data
                    .topology_algorithm_data
                    .increase_counter(1);
                error!(target: "topology", "[TOPOLOGY] - {str}")
            }
        }
        match self.apply_all_forces() {
            Ok(u) => {
                self.simulation_controller_data
                    .topology_algorithm_data
                    .increase_counter(u);
            }
            Err(str) => {
                self.simulation_controller_data
                    .topology_algorithm_data
                    .increase_counter(1);
                error!(target: "topology", "[TOPOLOGY] - {str}")
            }
        }
        self.compute_and_set_avg_magnitude();

        if self
            .simulation_controller_data
            .topology_algorithm_data
            .avg_positioning_score
            < 1.5
        {
            self.simulation_controller_data
                .topology_algorithm_data
                .status = TopologyStatus::Converged;
            self.simulation_controller_data
                .topology_algorithm_data
                .computed();
        }
    }
    fn generate_all_forces(&self) -> Result<HashMap<NodeId, Vector>, String> {
        let mut ret_hash: HashMap<NodeId, Vector> = HashMap::new();

        for (_node_id, displayable) in &self.simulation_controller_data.topology_algorithm_data.map
        {
            match displayable.clone().node {
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
    fn apply_all_forces(&mut self) -> Result<u8, String> {
        let mut error_counter: u8 = 0;
        for (ids, res) in &self
            .simulation_controller_data
            .topology_algorithm_data
            .nodes_resultant_vec
        {
            match self
                .simulation_controller_data
                .topology_algorithm_data
                .map
                .get_mut(ids)
            {
                Some(displayable) => {
                    displayable.position = displayable.position.add(&res.scale(0.025));
                }
                None => {
                    error_counter += 1;
                }
            }
        }
        Ok(error_counter)
    }
    fn compute_and_set_avg_magnitude(&mut self) {
        let mut sum_magnitude = 0.0;
        let mut counter: u16 = 0;
        for (_, vec) in self
            .simulation_controller_data
            .topology_algorithm_data
            .nodes_resultant_vec
            .clone()
        {
            sum_magnitude += vec.magnitude();
            counter += 1;
        }
        self.simulation_controller_data
            .topology_algorithm_data
            .avg_positioning_score = sum_magnitude / counter as f32;
    }
}
