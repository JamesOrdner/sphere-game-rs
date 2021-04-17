use nalgebra_glm as glm;

pub type InputAcceleration = glm::Vec2;
pub type Location = glm::Vec3;

#[derive(Clone, Copy)]
pub enum ComponentType {
    InputAcceleration,
    Location,
}

pub enum Component {
    InputAcceleration(InputAcceleration),
    Location(Location),
}

pub enum ComponentRef<'a> {
    InputAcceleration(&'a InputAcceleration),
    Location(&'a Location),
}

impl Component {
    pub fn from_ref(component_ref: ComponentRef) -> Self {
        match component_ref {
            ComponentRef::InputAcceleration(a) => Self::InputAcceleration(*a),
            ComponentRef::Location(a) => Self::Location(*a),
        }
    }
}
