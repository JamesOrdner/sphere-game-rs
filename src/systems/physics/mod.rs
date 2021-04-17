use super::SystemType;
use crate::{
    common::ComponentArray,
    components::{Component, ComponentRef, ComponentType, Location},
    engine::UPDATE_INTERVAL,
    entity::EntityID,
    state_manager::{ComponentQueryable, Event, EventListener},
    thread_pool::Scope,
};
use nalgebra_glm as glm;

struct ComponentData {
    location: Location,
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

                message_bus_sender.push(Event {
                    entity_id: component.entity_id,
                    component_type: ComponentType::Location,
                    system: SystemType::Physics,
                });
            });
        }
    }
}

impl super::Renderable for PhysicsSystem {
    fn render(&mut self, _thread_pool_scope: &Scope, _delta_time: f32) {}
}

impl EventListener for PhysicsSystem {
    fn receive(&mut self, _entity_id: EntityID, component: &Component) {
        match component {
            Component::InputAcceleration(acceleration) => {
                for component in &mut self.component_array {
                    component.data.velocity.x = acceleration.x;
                    component.data.velocity.y = acceleration.y;
                }
            }
            _ => {}
        }
    }
}

impl ComponentQueryable for PhysicsSystem {
    fn get(&self, component_type: ComponentType, entity_id: EntityID) -> Option<ComponentRef> {
        match component_type {
            ComponentType::Location => Some(ComponentRef::Location(
                &self.component_array[entity_id].data.location,
            )),
            _ => None,
        }
    }
}
