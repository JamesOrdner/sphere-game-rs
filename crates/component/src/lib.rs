use nalgebra_glm::{Vec2, Vec3};

pub enum Component {
    InputAcceleration(Vec2),
    Location(Vec3),
}
