use crate::{
    common::ComponentArray,
    components::{Component, Location},
    engine::UPDATE_INTERVAL,
    entity::EntityID,
    state_manager::{Event, Listener},
    systems::{game::GameSubsystem, SubsystemType},
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

impl GameSubsystem for PhysicsSystem {
    fn update(&mut self, thread_pool_scope: &Scope) {
        for component in &mut self.data {
            thread_pool_scope.execute(move |state_manager_sender| {
                component.data.location += component.data.velocity * UPDATE_INTERVAL.as_secs_f32();

                state_manager_sender.push(Event {
                    entity_id: component.entity_id,
                    component: Component::Location(component.data.location),
                    system_type: SubsystemType::Physics,
                });
            });
        }
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
