use std::thread;

use nalgebra_glm as glm;

use crate::{
    common::ComponentArray,
    components::Component,
    entity::EntityID,
    state_manager::{push_event, Listener},
    systems::SubsystemType,
};

struct CameraComponent {
    location: glm::Vec3,
    velocity: glm::Vec3,
    acceleration: glm::Vec2,
}

pub struct CameraSystem {
    data: ComponentArray<CameraComponent>,
}

impl CameraSystem {
    pub fn new() -> Self {
        CameraSystem {
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
                thread::current().id(),
            );
        }
    }
}

impl Listener for CameraSystem {
    fn receive(&mut self, entity_id: EntityID, component: &Component) {
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
