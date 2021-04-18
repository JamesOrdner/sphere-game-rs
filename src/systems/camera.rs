use super::SystemType;
use crate::components::ComponentType;
use crate::entity::EntityID;
use crate::state_manager::Event;
use crate::thread_pool::Scope;
use crate::{common::ComponentArray, components::Component};
use nalgebra_glm as glm;

struct CameraComponent {
    location: glm::Vec3,
    velocity: glm::Vec3,
    acceleration: glm::Vec2,
}

pub struct CameraSystem {
    camera_components: ComponentArray<CameraComponent>,
}

impl CameraSystem {
    pub fn new() -> Self {
        CameraSystem {
            camera_components: ComponentArray::new(),
        }
    }

    pub fn create_component(&mut self, entity_id: EntityID) {
        self.camera_components.push(
            entity_id,
            CameraComponent {
                location: glm::vec3(0.0, 0.0, 5.0),
                velocity: glm::Vec3::zeros(),
                acceleration: glm::Vec2::zeros(),
            },
        );
    }

    pub fn destroy_component(&mut self, entity_id: EntityID) {
        self.camera_components.remove(entity_id);
    }
}

impl super::Renderable for CameraSystem {
    fn render(&mut self, thread_pool_scope: &Scope, delta_time: f32) {
        for component in &mut self.camera_components {
            thread_pool_scope.execute(|state_manager_sender| {
                component.data.velocity +=
                    glm::vec2_to_vec3(&component.data.acceleration) * delta_time;
                component.data.velocity *= 1.0 - delta_time;
                component.data.location += component.data.velocity * delta_time;

                state_manager_sender.push(Event {
                    entity_id: component.entity_id,
                    component_type: ComponentType::Location,
                    system: SystemType::Camera,
                });
            });
        }
    }
}

impl crate::state_manager::Listener for CameraSystem {
    fn receive(&mut self, _entity_id: EntityID, component: &Component) {
        match component {
            Component::InputAcceleration(acceleration) => {
                for component in &mut self.camera_components {
                    component.data.acceleration = *acceleration;
                }
            }
            _ => {}
        }
    }
}
