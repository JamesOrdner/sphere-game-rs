use nalgebra_glm::{Vec2, Vec3};
use system::Timestamp;

pub enum Component {
    InputAcceleration(Vec2),
    Location(Vec3),
    Timestamp(Timestamp),
}
