use crate::systems::Systems;

pub type EntityID = u16;

pub struct Entity {
    pub entity_id: EntityID,
    destructor: fn(EntityID, &mut Systems),
}

impl Entity {
    pub fn destroy(&self, systems: &mut Systems) {
        (self.destructor)(self.entity_id, systems);
    }
}

pub fn create_static_mesh(entity_id: EntityID, systems: &mut Systems) -> Entity {
    systems.core.physics.create_component(entity_id);

    if let Some(client_systems) = &mut systems.client {
        client_systems
            .static_mesh
            .create_component(entity_id, "suzanne");
    }

    Entity {
        entity_id,
        destructor: destroy_static_mesh,
    }
}

fn destroy_static_mesh(entity_id: EntityID, systems: &mut Systems) {
    systems.core.physics.destroy_component(entity_id);

    if let Some(client_systems) = &mut systems.client {
        client_systems.static_mesh.destroy_component(entity_id);
    }
}
