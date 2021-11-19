use component::Component;
use entity::EntityId;
use event::{push_event, EventListener};
use nalgebra_glm::{vec3, Vec3};

pub struct System {
    entity_id: Option<EntityId>,
    location: Vec3,
    target: Option<Target>,
}

struct Target {
    entity_id: EntityId,
    location: Vec3,
}

impl System {
    pub fn new() -> Self {
        Self {
            entity_id: None,
            location: Vec3::zeros(),
            target: None,
        }
    }

    pub fn create_component(&mut self, entity_id: EntityId) {
        debug_assert!(self.entity_id.is_none());
        self.entity_id = Some(entity_id);
        self.location = vec3(0.0, 0.0, 5.0);
    }

    pub fn destroy_component(&mut self, entity_id: EntityId) {
        debug_assert!(*self.entity_id.as_ref().unwrap() == entity_id);
        self.entity_id = None;
    }

    pub fn set_target(&mut self, target_entity_id: EntityId) {
        self.target = Some(Target {
            entity_id: target_entity_id,
            location: Vec3::zeros(),
        })
    }

    pub async fn render(&mut self, delta_time: f32) {
        if let (Some(entity_id), Some(target)) = (self.entity_id, self.target.as_ref()) {
            self.location = target.location;

            push_event(entity_id, Component::RenderLocation(self.location));
        }
    }
}

impl EventListener for System {
    fn receive_event(&mut self, entity_id: EntityId, component: &Component) {
        let target = match self.target.as_mut() {
            Some(target) => target,
            None => return,
        };

        if target.entity_id == entity_id {
            if let Component::RenderLocation(location) = component {
                target.location = *location;
            }
        }
    }
}
