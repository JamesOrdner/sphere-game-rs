use std::thread;

use nalgebra_glm as glm;
use task::run_slice;

use crate::{
    common::ComponentArray,
    components::{Component, Location},
    engine::UPDATE_INTERVAL,
    entity::EntityID,
    state_manager::{push_event, Listener},
    systems::SubsystemType,
};

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

    pub async fn simulate(&mut self) {
        run_slice(self.data.as_mut_slice(), |component| {
            component.data.location += component.data.velocity * UPDATE_INTERVAL.as_secs_f32();

            push_event(
                component.entity_id,
                Component::Location(component.data.location),
                SubsystemType::Physics,
                thread::current().id(),
            );
        })
        .await;
    }
}

impl Listener for PhysicsSystem {
    fn receive(&mut self, entity_id: EntityID, component: &Component) {
        if !self.data.contains_entity(entity_id) {
            return;
        }

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
