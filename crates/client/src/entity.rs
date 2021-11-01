use entity::EntityId;
use gfx::StaticMesh;

use crate::Systems;

pub struct Entity {
    pub entity_id: EntityId,
    destructor: fn(EntityId, &mut Systems),
}

impl Entity {
    pub fn destroy(&self, systems: &mut Systems) {
        (self.destructor)(self.entity_id, systems);
    }
}

pub fn camera(entity_id: EntityId, systems: &mut Systems) -> Entity {
    systems.sim_camera.create_component(entity_id);
    systems.gfx_camera.create_component(entity_id);

    Entity {
        entity_id,
        destructor: |entity_id, systems| {
            systems.sim_camera.destroy_component(entity_id);
            systems.gfx_camera.destroy_component(entity_id);
        },
    }
}

pub fn static_mesh(
    entity_id: EntityId,
    systems: &mut Systems,
    static_mesh_id: StaticMesh,
) -> Entity {
    systems.sim_physics.create_component(entity_id);
    systems
        .gfx_static_mesh
        .create_component(entity_id, static_mesh_id);

    Entity {
        entity_id,
        destructor: |entity_id, systems| {
            systems.sim_physics.destroy_component(entity_id);
            systems.gfx_static_mesh.destroy_component(entity_id);
        },
    }
}
