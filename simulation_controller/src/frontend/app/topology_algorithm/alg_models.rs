use std::collections::HashMap;
use wg_2024::network::NodeId;
use crate::backend::sc_models::NetNode;
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
#[derive(Clone)]
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

#[derive(PartialEq, Eq, Clone)]
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

    pub fn check_if_nan(&mut self) {
        for (_,data_on_node) in self.map.iter_mut(){
            if data_on_node.position.x.is_nan() || data_on_node.position.y.is_nan(){
                let current_links = self.map_links.clone();
                self.initialize(current_links);
                return ();
            }
        }
    }

    pub fn get_all_links(&mut self){

    }
    /// General methods for single map nodes
    pub fn check_general_node_topology_status(&mut self) {
        match self.status {
            TopologyStatus::Stopped => {
                // Do nothing
            }
            TopologyStatus::ToInitialize => {
                self.general_status_initialize();
            }
            TopologyStatus::Computing => {
                self.check_if_nan();
                self.general_status_computing();
            }
            TopologyStatus::Converged => {
                // Do nothing
            }
        }
    }

    fn general_status_initialize(&mut self) {
        self.status = TopologyStatus::Computing;
        // Generate random positions for all nodes in map_links
        let mut unique_nodes = std::collections::HashSet::new();
        for (a, b) in &self.map_links {
            unique_nodes.insert(*a);
            unique_nodes.insert(*b);
        }
        for node_id in unique_nodes {
            // Use NetNode::Drone as a placeholder; in a real case, node type should be known
            self.map.insert(node_id, DisplayedNode::new_with_position(NetNode::Drone(node_id), Vector::random(100, 80)));
        }
    }

    fn general_status_computing(&mut self) {
        match self.general_generate_all_forces() {
            Ok(hsh) => {
                for (id, vec) in hsh {
                    self.nodes_resultant_vec.insert(id, vec);
                }
            }
            Err(_) => {
                self.increase_counter(1);
            }
        }
        match self.general_apply_all_forces() {
            Ok(u) => {
                self.increase_counter(u);
            }
            Err(_) => {
                self.increase_counter(1);
            }
        }
        self.general_compute_and_set_avg_magnitude();
        if self.avg_positioning_score < 1.5 {
            self.status = TopologyStatus::Converged;
            self.computed();
        }
    }

    fn general_generate_all_forces(&self) -> Result<HashMap<NodeId, Vector>, String> {
        let mut ret_hash: HashMap<NodeId, Vector> = HashMap::new();
        for (node_id, displayable) in &self.map {
            let res = self.general_generate_fundamental_force(*node_id, displayable.position)?;
            ret_hash.insert(*node_id, res);
        }
        Ok(ret_hash)
    }

    fn general_apply_all_forces(&mut self) -> Result<u8, String> {
        let mut error_counter: u8 = 0;
        for (ids, res) in &self.nodes_resultant_vec {
            match self.map.get_mut(ids) {
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

    fn general_compute_and_set_avg_magnitude(&mut self) {
        let mut sum_magnitude = 0.0;
        let mut counter: u16 = 0;
        for (_, vec) in self.nodes_resultant_vec.iter() {
            sum_magnitude += vec.magnitude();
            counter += 1;
        }
        self.avg_positioning_score = if counter > 0 {
            sum_magnitude / counter as f32
        } else {
            0.0
        };
    }

    fn general_generate_fundamental_force(&self, id: NodeId, vector: Vector) -> Result<Vector, String> {
        // will generate
        // Get Vector Positioning
        let mut forces = vec![];

        forces.push(self.general_generate_center_attraction_force(vector)?);
        forces.push(self.general_generate_repulsion_force(id)?);
        forces.push(self.general_generate_spring_force(id)?);
        forces.push(self.general_generate_link_repulsion_force(id)?);
        forces.push(self.general_generate_border_repulsion_force(vector)?);

        // Intersection repulsion per node
        //forces.push(self.general_generate_intersection_force(id)?);

        let mut resultant = Vector::new(0.0, 0.0);

        for f in forces {
            resultant = resultant.add(&f);
        }

        Ok(resultant)
    }

    fn general_generate_center_attraction_force(&self, vector: Vector) -> Result<Vector, String> {
        // Weak spring pull toward the center (50,40) in 100×80 world
        let center = Vector::new(50.0, 40.0);
        let k_center: f32 = 0.4;
        Ok(crate::frontend::app::topology_algorithm::vectors::generate_force_vec(&vector, &center, k_center, true))
    }

    fn general_generate_repulsion_force(&self, id: NodeId) -> Result<Vector, String> {
        // Exponential repulsion: very strong at short range, decays over r
        let k_repulsion: f32 = 340.0;
        let r: f32 = 8.75;

        let pos_a = self.map.get(&id).ok_or("Node position not found")?;
        let mut total_force = Vector::new(0.0, 0.0);

        for (other_id, pos_b) in &self.map {
            if other_id == &id {
                continue;
            }
            // repulse this node away from neighbor
            let force = crate::frontend::app::topology_algorithm::vectors::generate_force_exponential_repulsion_vec(pos_a.position, pos_b.position, k_repulsion, r);
            total_force = total_force.add(&force);
        }

        Ok(total_force)
    }

    fn general_generate_spring_force(&self, id: NodeId) -> Result<Vector, String> {
        // Hooke's law spring attraction along edges
        let k_spring = 3.75;
        
        // Find neighbors from map_links
        let mut neighbors = Vec::new();
        for (a, b) in &self.map_links {
            if *a == id {
                neighbors.push(*b);
            } else if *b == id {
                neighbors.push(*a);
            }
        }
        
        let pos_a = self.map.get(&id).ok_or("Node position not found")?;
        let mut total_force = Vector::new(0.0, 0.0);
        for neighbor_id in neighbors {
            let pos_b = self.map.get(&neighbor_id).ok_or("Neighbor position not found")?;
            // generate_force_vec(pos_a, pos_b, k, attract=true)
            let force = crate::frontend::app::topology_algorithm::vectors::generate_force_vec(&pos_a.position, &pos_b.position, k_spring, true);
            total_force = total_force.add(&force);
        }
        Ok(total_force)
    }

    fn general_generate_border_repulsion_force(&self, vector: Vector) -> Result<Vector, String> {
        // Default map size
        let map_width: f32 = 100.0;
        let map_height: f32 = 80.0;

        // Distances from borders
        let dx_left = vector.x;
        let dx_right = map_width - vector.x;
        let dy_top = vector.y;
        let dy_bottom = map_height - vector.y;

        // Exponential repulsion from each border to keep nodes inside the map
        let k_border: f32 = 240.0;   // overall strength of the border repulsion
        let r: f32 = 2.5;         // characteristic decay distance

        // Exponential repulsive forces from borders
        let force_left = k_border * (-dx_left / r).exp();     // pushes right
        let force_right = -k_border * (-dx_right / r).exp();  // pushes left
        let force_top = k_border * (-dy_top / r).exp();       // pushes down
        let force_bottom = -k_border * (-dy_bottom / r).exp();// pushes up

        let fx = force_left + force_right;
        let fy = force_top + force_bottom;

        Ok(Vector::new(fx, fy))
    }

    fn general_generate_link_repulsion_force(&self, id: NodeId) -> Result<Vector, String> {
        // Keep nodes off other links by perpendicular repulsion
        let threshold: f32 = 30.0;     // influence distance from a link
        let k_link: f32 = 80.0;       // strength at zero distance

        // Position of this node
        let node = self.map.get(&id).ok_or("Node position not found")?;
        let pos = node.position;
        let mut total_force = Vector::new(0.0, 0.0);

        // Iterate all links
        for &(a, b) in &self.map_links {
            // Skip links incident to this node
            if a == id || b == id {
                continue;
            }
            let na = self.map.get(&a).unwrap();
            let nb = self.map.get(&b).unwrap();
            let pa = na.position;
            let pb = nb.position;

            // Project pos onto segment pa-pb
            let ab = pb.sub(&pa);
            let ap = pos.sub(&pa);
            let ab_len2 = ab.x * ab.x + ab.y * ab.y;
            if ab_len2 == 0.0 {
                continue;
            }
            let t = ((ap.x * ab.x + ap.y * ab.y) / ab_len2).clamp(0.0, 1.0);
            let closest = pa.add(&ab.scale(t));

            // Compute repulsion if within threshold
            let dir = pos.sub(&closest);
            let dist = dir.magnitude();
            if dist < threshold {
                let strength = k_link * (1.0 - dist / threshold);
                let force = dir.normalize().scale(strength);
                total_force = total_force.add(&force);
            }
        }
        Ok(total_force)
    }

    /// Compute the repulsion force on a single node due to intersecting links
    fn general_generate_intersection_force(&self, id: NodeId) -> Result<Vector, String> {
        let intersections = self.general_generate_intersection_forces()?;
        // Return the force for this id or zero if none
        Ok(intersections.get(&id).cloned().unwrap_or(Vector::new(0.0, 0.0)))
    }

    /// Compute repulsion forces for all nodes due to intersecting links.
    fn general_generate_intersection_forces(&self) -> Result<std::collections::HashMap<NodeId, Vector>, String> {
        let mut result = std::collections::HashMap::new();
        let links = &self.map_links;
        let map = &self.map;

        // Examine each unique pair of links
        for i in 0..links.len() {
            for j in (i + 1)..links.len() {
                let (a, b) = links[i];
                let (c, d) = links[j];
                // Skip if they share a node
                if a == c || a == d || b == c || b == d {
                    continue;
                }
                let p1 = map.get(&a).ok_or("Node position not found")?.position;
                let p2 = map.get(&b).ok_or("Node position not found")?.position;
                let p3 = map.get(&c).ok_or("Node position not found")?.position;
                let p4 = map.get(&d).ok_or("Node position not found")?.position;

                // Check intersection
                fn ccw(u: &Vector, v: &Vector, w: &Vector) -> bool {
                    (w.y - u.y) * (v.x - u.x) > (v.y - u.y) * (w.x - u.x)
                }
                if ccw(&p1, &p3, &p4) != ccw(&p2, &p3, &p4)
                    && ccw(&p1, &p2, &p3) != ccw(&p1, &p2, &p4)
                {
                    // Compute intersection point
                    let denom = (p1.x - p2.x) * (p3.y - p4.y) - (p1.y - p2.y) * (p3.x - p4.x);
                    if denom.abs() < f32::EPSILON {
                        continue;
                    }
                    let x_num = (p1.x * p2.y - p1.y * p2.x) * (p3.x - p4.x)
                        - (p1.x - p2.x) * (p3.x * p4.y - p3.y * p4.x);
                    let y_num = (p1.x * p2.y - p1.y * p2.x) * (p3.y - p4.y)
                        - (p1.y - p2.y) * (p3.x * p4.y - p3.y * p4.x);
                    let ix = x_num / denom;
                    let iy = y_num / denom;
                    let intersection = Vector::new(ix, iy);

                    // Repel each endpoint
                    let k: f32 = 5.0;
                    for &(node_id, pos) in &[(a, p1), (b, p2), (c, p3), (d, p4)] {
                        let dir = pos.sub(&intersection);
                        let dist = dir.magnitude().max(0.1);
                        let force = dir.normalize().scale(k / dist);
                        result
                            .entry(node_id)
                            .and_modify(|v: &mut Vector| *v = v.add(&force))
                            .or_insert(force);
                    }
                }
            }
        }
        Ok(result)
    }
}