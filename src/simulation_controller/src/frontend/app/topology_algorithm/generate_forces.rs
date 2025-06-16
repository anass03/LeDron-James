use ratatui::layout::Rect;
use wg_2024::network::NodeId;
use crate::backend::simulation_controller::NetNode;
use crate::frontend::app::App;
use crate::frontend::app::topology_algorithm::vectors::{
    generate_force_vec,
    generate_force_exponential_repulsion_vec,
    Vector,
};
use std::collections::HashMap;

impl App<'_>{

    fn compute_best_positions()->Result<(),String>{
        todo!()
    }

    fn generate_client_force(&self, id:NodeId)->Result<(),String>{
        todo!()

    }
    fn generate_server_force(&self, id:NodeId)->Result<(),String>{
        todo!()

    }
    pub fn generate_fundamental_force(&self, id:NodeId, vector: Vector) ->Result<Vector,String>{
        // will generate
        // Get Vector Positioning
        let mut forces = vec![];

        forces.push(self.generate_center_attraction_force(vector)?);
        forces.push(self.generate_repulsion_force(id)?);
        forces.push(self.generate_spring_force(id)?);
        forces.push(self.generate_link_repulsion_force(id)?);
        forces.push(self.generate_border_repulsion_force(vector)?);

        // Intersection repulsion per node
        //forces.push(self.generate_intersection_force(id)?);

        let mut resultant = Vector::new(0.0,0.0);

        for f in  forces {
            resultant = resultant.add(&f);
        }

        Ok(resultant)
    }

    fn generate_center_attraction_force(&self, vector: Vector) -> Result<Vector, String> {
        // Weak spring pull toward the center (50,40) in 100×80 world
        let center = Vector::new(50.0, 40.0);
        let k_center: f32 = 0.4;
        Ok(generate_force_vec(&vector, &center, k_center, true))
    }

    fn generate_repulsion_force(&self, id: NodeId) -> Result<Vector, String> {
        // Exponential repulsion: very strong at short range, decays over r
        let k_repulsion: f32 = 340.0;
        let r: f32 = 8.75;

        let pos_a = self
            .simulation_controller_data
            .topology_algorithm_data
            .map
            .get(&id)
            .ok_or("Node position not found")?;
        let mut total_force = Vector::new(0.0, 0.0);

        for (other_id, pos_b) in &self.simulation_controller_data.topology_algorithm_data.map {
            if other_id == &id {
                continue;
            }
            // repulse this node away from neighbor
            let force = generate_force_exponential_repulsion_vec(pos_a.position, pos_b.position, k_repulsion, r);
            total_force = total_force.add(&force);
        }

        Ok(total_force)
    }

    fn generate_spring_force(&self, id: NodeId) -> Result<Vector, String> {
        // Hooke's law spring attraction along edges
        let k_spring = 3.75;
        // Determine this node as a NetNode variant

        let netnode = if self.simulation_controller_data.running_network.contains_key(&NetNode::Drone(id)) {
            NetNode::Drone(id)
        } else if self.simulation_controller_data.running_network.contains_key(&NetNode::Client(id)) {
            NetNode::Client(id)
        } else {
            NetNode::Server(id)
        };

        let neighbors = self
            .simulation_controller_data
            .running_network
            .get(&netnode)
            .ok_or("Node connections not found")?;
        let pos_a = self
            .simulation_controller_data
            .topology_algorithm_data
            .map
            .get(&id)
            .ok_or("Node position not found")?;
        let mut total_force = Vector::new(0.0, 0.0);
        for neighbor_id in neighbors {
            let pos_b = self
                .simulation_controller_data
                .topology_algorithm_data
                .map
                .get(neighbor_id)
                .ok_or("Neighbor position not found")?;
            // generate_force_vec(pos_a, pos_b, k, attract=true)
            let force = generate_force_vec(&pos_a.position, &pos_b.position, k_spring, true);
            total_force = total_force.add(&force);
        }
        Ok(total_force)
    }

    fn generate_border_repulsion_force(&self, vector: Vector) -> Result<Vector, String> {
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

    fn generate_link_repulsion_force(&self, id: NodeId) -> Result<Vector, String> {
        // Keep nodes off other links by perpendicular repulsion
        let threshold: f32 = 30.0;     // influence distance from a link
        let k_link: f32 = 80.0;       // strength at zero distance

        // Position of this node
        let node = self
            .simulation_controller_data
            .topology_algorithm_data
            .map
            .get(&id)
            .ok_or("Node position not found")?;
        let pos = node.position;
        let mut total_force = Vector::new(0.0, 0.0);

        // Iterate all links
        for &(a, b) in &self.simulation_controller_data.topology_algorithm_data.map_links {
            // Skip links incident to this node
            if a == id || b == id {
                continue;
            }
            let na = self
                .simulation_controller_data
                .topology_algorithm_data
                .map
                .get(&a)
                .unwrap();
            let nb = self
                .simulation_controller_data
                .topology_algorithm_data
                .map
                .get(&b)
                .unwrap();
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
    fn generate_intersection_force(&self, id: NodeId) -> Result<Vector, String> {
        let intersections = self.generate_intersection_forces()?;
        // Return the force for this id or zero if none
        Ok(intersections.get(&id).cloned().unwrap_or(Vector::new(0.0, 0.0)))
    }

    /// Compute repulsion forces for all nodes due to intersecting links.
    fn generate_intersection_forces(&self) -> Result<std::collections::HashMap<NodeId, Vector>, String> {
        let mut result = std::collections::HashMap::new();
        let links = &self.simulation_controller_data.topology_algorithm_data.map_links;
        let map = &self.simulation_controller_data.topology_algorithm_data.map;

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