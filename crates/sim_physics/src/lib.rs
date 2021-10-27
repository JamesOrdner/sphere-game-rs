use component::{Component, Location};
use data::ComponentArray;
use entity::EntityID;
use event::{push_event, EventListener};
use nalgebra_glm as glm;
use system::SubsystemType;
use task::run_slice;

struct ComponentData {
    location: Location,
    velocity: glm::Vec3,
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
            component.data.location +=
                component.data.velocity * std::time::Duration::from_micros(16_666).as_secs_f32();

            push_event(
                component.entity_id,
                Component::Location(component.data.location),
                SubsystemType::Physics,
            );
        })
        .await;
    }
}

impl EventListener for System {
    fn receive_event(&mut self, entity_id: EntityID, component: &Component) {
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
