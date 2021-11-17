use nalgebra_glm::{Vec2, Vec3};
use system::Timestamp;

pub enum Component {
    InputAcceleration(Vec2),
    Location(Vec3),
    NetStaticMeshLocation {
        timestamp: Timestamp,
        location: Vec3,
    },
    NetStaticMeshVelocity {
        timestamp: Timestamp,
        velocity: Vec3,
    },
    Timestamp(Timestamp),
    Velocity(Vec3),
}
