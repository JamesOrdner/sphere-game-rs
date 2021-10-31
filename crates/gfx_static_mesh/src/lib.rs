use component::Component;
use data::ComponentArray;
use entity::EntityId;
use event::EventListener;
use gfx::{GfxDelegate, StaticMesh};
use nalgebra_glm::{translate, Mat4, Vec3};

struct StaticMeshComponent {
    static_mesh: StaticMesh,
    location: Vec3,
}

pub struct System {
    components: ComponentArray<StaticMeshComponent>,
}

impl System {
    pub fn new() -> Self {
        Self {
            components: ComponentArray::new(),
        }
    }

    pub fn create_component(&mut self, entity_id: EntityId, static_mesh: StaticMesh) {
        self.components.push(
            entity_id,
            StaticMeshComponent {
                static_mesh,
                location: Vec3::zeros(),
            },
        );
    }

    pub fn destroy_component(&mut self, entity_id: EntityId) {
        self.components.remove(entity_id);
    }

    pub async fn render(&mut self, gfx_delegate: &GfxDelegate<'_>) {
        let identity = Mat4::identity();
        for component in &self.components {
            let model_matrix = translate(&identity, &component.data.location);
            gfx_delegate.update_static_mesh(&component.data.static_mesh, model_matrix);
        }

        for component in &self.components {
            gfx_delegate.draw_instance(&component.data.static_mesh);
        }
    }
}

impl EventListener for System {
    fn receive_event(&mut self, entity_id: EntityId, component: &Component) {
        if !self.components.contains_entity(entity_id) {
            return;
        }

        if let Component::Location(location) = component {
            self.components[entity_id].data.location = *location;
        }
    }
}
