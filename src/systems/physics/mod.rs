use crate::common::ComponentArray;
use crate::engine::UPDATE_INTERVAL;
use crate::entity::EntityID;
use crate::message_bus::Message;
use crate::thread_pool::Scope;
use nalgebra_glm as glm;

struct ComponentData {
    location: glm::Vec3,
    velocity: glm::Vec3,
}

pub struct PhysicsSystem {
    component_array: ComponentArray<ComponentData>,
}

impl PhysicsSystem {
    pub fn new() -> Self {
        PhysicsSystem {
            component_array: ComponentArray::new(),
        }
    }

    pub fn create_component(&mut self, entity_id: EntityID) {
        self.component_array.push(
            entity_id,
            ComponentData {
                location: glm::Vec3::zeros(),
                velocity: glm::Vec3::zeros(),
            },
        );
    }

    pub fn destroy_component(&mut self, entity_id: EntityID) {
        self.component_array.remove(entity_id);
    }
}

impl super::Updatable for PhysicsSystem {
    fn update(&mut self, thread_pool_scope: &Scope) {
        for component in &mut self.component_array {
            thread_pool_scope.execute(|message_bus_sender| {
                component.data.location += component.data.velocity * UPDATE_INTERVAL.as_secs_f32();

                message_bus_sender.push(Message::Location {
                    entity_id: component.entity_id,
                    location: component.data.location,
                });
            });
        }
    }
}

impl super::Renderable for PhysicsSystem {
    fn render(&mut self, _thread_pool_scope: &Scope, _delta_time: f32) {}
}

impl crate::message_bus::Receiver for PhysicsSystem {
    fn receive(&mut self, messages: &[Message]) {
        for message in messages {
            match message {
                Message::InputAcceleration { acceleration } => {
                    for component in &mut self.component_array {
                        component.data.velocity.x = acceleration.x;
                        component.data.velocity.y = acceleration.y;
                    }
                }
                _ => {}
            }
        }
    }
}
