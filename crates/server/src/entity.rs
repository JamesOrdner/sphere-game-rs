use entity::EntityId;

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

pub fn static_mesh(entity_id: EntityId, systems: &mut Systems) -> Entity {
    systems
        .sim_network_server
        .create_static_mesh_component(entity_id);
    systems.sim_physics.create_component(entity_id);

    Entity {
        entity_id,
        destructor: |entity_id, systems| {
            systems
                .sim_network_server
                .destroy_static_mesh_component(entity_id);
            systems.sim_physics.destroy_component(entity_id);
        },
    }
}
