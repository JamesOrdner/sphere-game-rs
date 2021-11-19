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
    systems.simulation.camera.create_component(entity_id);
    systems.graphics.camera.create_component(entity_id);

    Entity {
        entity_id,
        destructor: |entity_id, systems| {
            systems.simulation.camera.destroy_component(entity_id);
            systems.graphics.camera.destroy_component(entity_id);
        },
    }
}

pub fn static_mesh(
    entity_id: EntityId,
    systems: &mut Systems,
    static_mesh_id: StaticMesh,
) -> Entity {
    systems
        .simulation
        .network_client
        .create_static_mesh_component(entity_id);
    systems.simulation.physics.create_component(entity_id);
    systems
        .graphics
        .static_mesh
        .create_component(entity_id, static_mesh_id);

    Entity {
        entity_id,
        destructor: |entity_id, systems| {
            systems
                .simulation
                .network_client
                .destroy_static_mesh_component(entity_id);
            systems.simulation.physics.destroy_component(entity_id);
            systems.graphics.static_mesh.destroy_component(entity_id);
        },
    }
}
