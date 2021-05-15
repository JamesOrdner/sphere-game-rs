use nalgebra_glm as glm;

pub type InputAcceleration = glm::Vec2;
pub type Location = glm::Vec3;

pub enum Component {
    InputAcceleration(InputAcceleration),
    Location(Location),
}
