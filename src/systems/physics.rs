use super::SystemType;
use crate::{
    common::ComponentArray,
    components::{Component, ComponentRef, ComponentType, Location},
    engine::UPDATE_INTERVAL,
    entity::EntityID,
    state_manager::{ComponentQuery, Event, Listener},
    thread_pool::Scope,
};
use nalgebra_glm as glm;

struct ComponentData {
    location: Location,
    velocity: glm::Vec3,
}

pub struct PhysicsSystem {
    data: ComponentArray<ComponentData>,
}

impl PhysicsSystem {
    pub fn new() -> Self {
        PhysicsSystem {
            data: ComponentArray::new(),
        }
    }

    pub fn create_component(&mut self, entity_id: EntityID) {
        self.data.push(
            entity_id,
            ComponentData {
                location: glm::Vec3::zeros(),
                velocity: glm::Vec3::zeros(),
            },
        );
    }

    pub fn destroy_component(&mut self, entity_id: EntityID) {
        self.data.remove(entity_id);
    }
}

impl Listener for PhysicsSystem {
    fn receive(&mut self, entity_id: EntityID, component: &Component) {
        if !self.data.contains_entity(entity_id) { return }

        match component {
            Component::InputAcceleration(acceleration) => {
                for component in &mut self.data {
                    component.data.velocity.x = acceleration.x;
                    component.data.velocity.y = acceleration.y;
                }
            }
            _ => {}
        }
    }
}

impl super::Updatable for PhysicsSystem {
    fn update(&mut self, thread_pool_scope: &Scope) {
        for component in &mut self.data {
            thread_pool_scope.execute(|state_manager_sender| {
                component.data.location += component.data.velocity * UPDATE_INTERVAL.as_secs_f32();

                state_manager_sender.push(Event {
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

impl ComponentQuery for PhysicsSystem {
    fn get(&self, component_type: ComponentType, entity_id: EntityID) -> Option<ComponentRef> {
        match component_type {
            ComponentType::Location => Some(ComponentRef::Location(
                &self.data[entity_id].data.location,
            )),
            _ => None,
        }
    }
}
