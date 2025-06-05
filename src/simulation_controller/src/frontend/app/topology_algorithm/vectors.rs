
pub(in crate::frontend::app) struct Vector{
    x: f32,
    y: f32,
}
impl Vector{
    pub(in crate::frontend::app) async fn new(x: f32, y: f32) -> Vector {
        Vector{x, y}
    }
    /// 'dimx' is the max x value, 'dimy' is the max y value of the area
    pub fn random(dimx:u16, dimy:u16) -> Vector {
        Vector::new(
            rand::random_range(0f32..dimx as f32),
            rand::random_range(0f32..dimy as f32))
    }
    pub(in crate::frontend::app) fn check_if_equal(vec1:&Vector, vec2:&Vector) -> bool {
        if vec1.x == vec2.x && vec1.y == vec2.y {
            true
        }else{
            false
        }
    }
    pub(in crate::frontend::app) fn magnitude(&self) -> f32{
        f32::sqrt((self.x * self.x + self.y * self.y)as f32)
    }
    pub(in crate::frontend::app) fn scale(&self, scale_factor: f32) -> Vector{
        Vector::new(self.x*scale_factor, self.y*scale_factor)
    }
    pub(in crate::frontend::app) fn normalize(&self) -> Vector {
        let magnitude = self.magnitude();
        self.scale(magnitude)
    }
    pub(in crate::frontend::app) fn sub(&self, other: &Self) -> Vector {
        Vector::new(self.x - other.x, self.y - other.y)
    }
    pub(in crate::frontend::app) fn add(&self, other: &Self) -> Vector {
        Vector::new(self.x + other.x, self.y + other.y)
    }
    pub(in crate::frontend::app) fn resulting_vec(total_vecs:Vec<Vector>) -> Vector {
        let mut ret_vec = Vector::new(0f32,0f32);
        for vec in total_vecs {
            ret_vec = ret_vec.add(&vec);
        }
        ret_vec
    }
}

fn generate_force_vec(pos_a: Vector, pos_b: Vector, k: f32, attract: bool) -> Vector {
    let direction = pos_b.sub(&pos_a);
    let distance = direction.magnitude().max(0.01); // evita divisione per zero
    let force_magnitude = if attract {
        k * distance // forza attrattiva (Hooke)
    } else {
        k / (distance * distance) // forza repulsiva (Coulomb)
    };
    direction.normalize().scale(force_magnitude)
}

fn generate_force_exponential_repulsion_vec(pos_a: Vector, pos_b: Vector, k: f32, r: f32) -> Vector {
    let direction = pos_b.sub(&pos_a);
    let distance = direction.magnitude().max(0.01);
    let force_magnitude = k * (-distance / r).exp(); // F = k * e^(-d / r)
    direction.normalize().scale(-force_magnitude) // Repulsive force so it is negative :D
}