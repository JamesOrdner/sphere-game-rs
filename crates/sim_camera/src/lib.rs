use component::Component;
use data::ComponentArray;
use entity::EntityID;
use event::{push_event, EventListener};
use nalgebra_glm as glm;
use system::SubsystemType;

struct CameraComponent {
    location: glm::Vec3,
    velocity: glm::Vec3,
    acceleration: glm::Vec2,
}

pub struct System {
    data: ComponentArray<CameraComponent>,
}

impl System {
    pub fn new() -> Self {
        Self {
            data: ComponentArray::new(),
        }
    }

    pub fn create_component(&mut self, entity_id: EntityID) {
        self.data.push(
            entity_id,
            CameraComponent {
                location: glm::vec3(0.0, 0.0, 5.0),
                velocity: glm::Vec3::zeros(),
                acceleration: glm::Vec2::zeros(),
            },
        );
    }

    pub fn destroy_component(&mut self, entity_id: EntityID) {
        self.data.remove(entity_id);
    }

    pub async fn simulate(&mut self) {
        for component in &mut self.data {
            let delta_time = 0.01; // temp
            component.data.velocity += glm::vec2_to_vec3(&component.data.acceleration) * delta_time;
            component.data.velocity *= 1.0 - delta_time;
            component.data.location += component.data.velocity * delta_time;

            push_event(
                component.entity_id,
                Component::Location(component.data.location),
                SubsystemType::Camera,
            );
        }
    }
}

impl EventListener for System {
    fn receive_event(&mut self, entity_id: EntityID, component: &Component) {
        if !self.data.contains_entity(entity_id) {
            return;
        }

        match component {
            Component::InputAcceleration(acceleration) => {
                for component in &mut self.data {
                    component.data.acceleration = *acceleration;
                }
            }
            _ => {}
        }
    }
}
