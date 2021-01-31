use crate::common::{ComponentArray, EntityID};
use crate::engine::UPDATE_INTERVAL;
use crate::message_bus::Message;
use crate::thread_pool::Scope;

struct ComponentData {
    location_x: f32,
    location_y: f32,
    velocity_x: f32,
    velocity_y: f32,
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
}

impl super::Componentable for PhysicsSystem {
    fn create_component(&mut self, entity_id: EntityID) {
        self.component_array.push(
            entity_id,
            ComponentData {
                location_x: 0.0,
                location_y: 0.0,
                velocity_x: 0.0,
                velocity_y: 0.0,
            },
        );
    }

    fn destroy_component(&mut self, entity_id: EntityID) {
        self.component_array.remove(entity_id);
    }
}

impl super::Updatable for PhysicsSystem {
    fn update(&mut self, thread_pool_scope: &Scope) {
        for component in &mut self.component_array {
            thread_pool_scope.execute(|message_bus_sender| {
                component.data.location_x +=
                    component.data.velocity_x * UPDATE_INTERVAL.as_secs_f32();
                component.data.location_y +=
                    component.data.velocity_y * UPDATE_INTERVAL.as_secs_f32();

                message_bus_sender.push(Message::Location {
                    entity_id: component.entity_id,
                    x: component.data.location_x,
                    y: component.data.location_y,
                });
            });
        }
    }
}

impl super::Renderable for PhysicsSystem {
    fn render(&mut self, _thread_pool_scope: &Scope) {}
}

impl crate::message_bus::Receiver for PhysicsSystem {
    fn receive(&mut self, messages: &[Message]) {
        for message in messages {
            match message {
                Message::InputAcceleration { x, y } => {
                    for component in &mut self.component_array {
                        component.data.velocity_x = *x;
                        component.data.velocity_y = *y;
                    }
                }
                _ => {}
            }
        }
    }
}
