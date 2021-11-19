use component::Component;
use data::ComponentArray;
use entity::EntityId;
use event::EventListener;
use gfx::{gfx_delegate, StaticMesh};
use nalgebra_glm::{translate, Mat4, Vec3};
use task::run_slice;

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

    pub async fn render(&mut self) {
        run_slice(self.components.as_slice(), |component| {
            let gfx_delegate = gfx_delegate();
            let model_matrix = translate(&Mat4::identity(), &component.data.location);
            gfx_delegate.update_static_mesh(&component.data.static_mesh, model_matrix);
        })
        .await;

        run_slice(self.components.as_slice(), |component| {
            let gfx_delegate = gfx_delegate();
            gfx_delegate.draw_instance(&component.data.static_mesh);
        })
        .await;
    }
}

impl EventListener for System {
    fn receive_event(&mut self, entity_id: EntityId, component: &Component) {
        if !self.components.contains_entity(entity_id) {
            return;
        }

        if let Component::RenderLocation(location) = component {
            self.components[entity_id].data.location = *location;
        }
    }
}
