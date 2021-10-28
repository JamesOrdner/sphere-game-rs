use component::Component;
use data::ComponentArray;
use entity::EntityId;
use event::{push_event, EventListener};
use nalgebra_glm::{vec2_to_vec3, vec3, Vec2, Vec3};

struct CameraComponent {
    location: Vec3,
    velocity: Vec3,
    acceleration: Vec2,
}

pub struct System {
    data: ComponentArray<CameraComponent>,
}

impl System {
    pub fn new() -> Self {
        Self {
            data: ComponentArray::new(),
        }
    }

    pub fn create_component(&mut self, entity_id: EntityId) {
        self.data.push(
            entity_id,
            CameraComponent {
                location: vec3(0.0, 0.0, 5.0),
                velocity: Vec3::zeros(),
                acceleration: Vec2::zeros(),
            },
        );
    }

    pub fn destroy_component(&mut self, entity_id: EntityId) {
        self.data.remove(entity_id);
    }

    pub async fn simulate(&mut self, delta_time: f32) {
        for component in &mut self.data {
            component.data.velocity += vec2_to_vec3(&component.data.acceleration) * delta_time;
            component.data.velocity *= 1.0 - delta_time;
            component.data.location += component.data.velocity * delta_time;

            push_event(
                component.entity_id,
                Component::Location(component.data.location),
            );
        }
    }
}

impl EventListener for System {
    fn receive_event(&mut self, _: EntityId, _: &Component) {}
}
