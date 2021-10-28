use component::Component;
use data::ComponentArray;
use entity::EntityId;
use event::{push_event, EventListener};
use nalgebra_glm::{vec2_to_vec3, Vec3};
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

    pub async fn simulate(&mut self, delta_time: f32) {
        run_slice(self.data.as_mut_slice(), |component| {
            component.data.location += component.data.velocity * delta_time;

            push_event(
                component.entity_id,
                Component::Location(component.data.location),
            );
        })
        .await;
    }
}

impl EventListener for System {
    fn receive_event(&mut self, _: EntityId, component: &Component) {
        match component {
            Component::InputAcceleration(acceleration) => {
                for component in &mut self.data {
                    component.data.velocity = vec2_to_vec3(acceleration);
                }
            }
            _ => {}
        }
    }
}
