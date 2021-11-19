use component::Component;
use entity::EntityId;
use event::EventListener;
use gfx::gfx_delegate;
use nalgebra_glm::{ortho_rh_zo, translate, Mat4, Vec3};

pub struct System {
    entity_id: Option<EntityId>,
    location: Vec3,
}

impl System {
    pub fn new() -> Self {
        Self {
            entity_id: None,
            location: Vec3::zeros(),
        }
    }

    pub fn create_component(&mut self, entity_id: EntityId) {
        debug_assert!(self.entity_id.is_none());
        self.entity_id = Some(entity_id);
    }

    pub fn destroy_component(&mut self, entity_id: EntityId) {
        debug_assert!(*self.entity_id.as_ref().unwrap() == entity_id);
        self.entity_id = None;
    }

    pub async fn render(&mut self) {
        let proj_matrix = ortho_rh_zo(-2.0, 2.0, 2.0, -2.0, -2.0, 2.0);
        let view_matrix = translate(&Mat4::identity(), &self.location);

        gfx_delegate().update_scene(view_matrix, proj_matrix);
    }
}

impl EventListener for System {
    fn receive_event(&mut self, entity_id: EntityId, component: &Component) {
        if self.entity_id.as_ref() == Some(&entity_id) {
            if let Component::RenderLocation(location) = component {
                self.location = *location;
            }
        }
    }
}
