use component::Component;
use data::ComponentArray;
use entity::EntityId;
use event::{push_event, EventListener};
use nalgebra_glm::{vec2_to_vec3, Vec3};
use system::{Timestamp, TIMESTEP_F32};
use task::run_slice;

struct ComponentData {
    location: Vec3,
    velocity: Vec3,
}

pub struct System {
    data: ComponentArray<ComponentData>,
}

impl System {
    pub fn new() -> Self {
        System {
            data: ComponentArray::new(),
        }
    }

    pub fn create_component(&mut self, entity_id: EntityId) {
        self.data.push(
            entity_id,
            ComponentData {
                location: Vec3::zeros(),
                velocity: Vec3::zeros(),
            },
        );
    }

    pub fn destroy_component(&mut self, entity_id: EntityId) {
        self.data.remove(entity_id);
    }

    pub async fn simulate(&mut self, timestamp: Timestamp) {
        run_slice(self.data.as_mut_slice(), |component| {
            component.data.location += component.data.velocity * TIMESTEP_F32;

            push_event(
                component.entity_id,
                Component::Location(component.data.location),
            );
        })
        .await;
    }
}

impl EventListener for System {
    fn receive_event(&mut self, entity_id: EntityId, component: &Component) {
        if entity_id > 0 && !self.data.contains_entity(entity_id) {
            return;
        }

        match component {
            Component::InputAcceleration(acceleration) => {
                for component in &mut self.data {
                    component.data.velocity = vec2_to_vec3(acceleration);
                }
            }
            Component::Location(location) => {
                self.data[entity_id].data.location = *location;
            }
            _ => {}
        }
    }
}
