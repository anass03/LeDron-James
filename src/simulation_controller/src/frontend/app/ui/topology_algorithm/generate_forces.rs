use crate::frontend::app::ui::topology_algorithm::vectors::Vector;

fn generate_force_vec(pos_a: Vector, pos_b: Vector, k: f32, attract: bool) -> Vector {
    let direction = pos_b.sub(&pos_a);
    let distance = direction.magnitude().max(1.0); // evita divisione per zero
    let force_magnitude = if attract {
        k * distance // forza attrattiva (Hooke)
    } else {
        k / (distance * distance) // forza repulsiva (Coulomb)
    };

    direction.normalize().scale(force_magnitude)
}

fn apply_exponential_repulsion(pos_a: Vector, pos_b: Vector, k: f32, r: f32) -> Vector {
    let direction = pos_b.sub(&pos_a);
    let distance = direction.magnitude().max(0.01);
    let force_magnitude = k * (-distance / r).exp(); // F = k * e^(-d / r)
    direction.normalize().scale(-force_magnitude) // Repulsive force so it is negative :D
}