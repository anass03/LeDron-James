
pub(super) struct Vector{
    x: f32,
    y: f32,
}
impl Vector{
    pub(super) fn new(x: f32, y: f32) -> Vector {
        Vector{x, y}
    }
    /// 'dimx' is the max x value, 'dimy' is the max y value of the area
    pub fn random(dimx:u16, dimy:u16) -> Vector {
        Vector::new(
            rand::random_range(0f32..dimx as f32),
            rand::random_range(0f32..dimy as f32))
    }
    pub(super) fn check_if_equal(vec1:&Vector, vec2:&Vector) -> bool {
        if vec1.x == vec2.x && vec1.y == vec2.y {
            true
        }else{
            false
        }
    }
    pub(super) fn magnitude(&self)-> f32{
        f32::sqrt((self.x * self.x + self.y * self.y)as f32)
    }
    pub(super) fn scale(&self, scale_factor: f32) -> Vector{
        Vector::new(self.x*scale_factor, self.y*scale_factor)
    }
    pub(super) fn normalize(&self) -> Vector {
        let magnitude = self.magnitude();
        self.scale(magnitude)
    }
    pub(super) fn sub(&self, other: &Self) -> Vector {
        Vector::new(self.x - other.x, self.y - other.y)
    }
    pub(super) fn add(&self, other: &Self) -> Vector {
        Vector::new(self.x + other.x, self.y + other.y)
    }
    pub(super)  fn resulting_vec(total_vecs:Vec<Vector>)-> Vector {
        let mut ret_vec = Vector::new(0f32,0f32);
        for vec in total_vecs {
            ret_vec = ret_vec.add(&vec);
        }
        ret_vec
    }
}