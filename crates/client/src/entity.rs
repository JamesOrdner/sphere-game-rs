use entity::EntityID;

use crate::Systems;

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
    systems
        .gfx_static_mesh
        .create_component(entity_id, "suzanne");
    systems.sim_physics.create_component(entity_id);

    Entity {
        entity_id,
        destructor: destroy_static_mesh,
    }
}

fn destroy_static_mesh(entity_id: EntityID, systems: &mut Systems) {
    systems.gfx_static_mesh.destroy_component(entity_id);
    systems.sim_physics.destroy_component(entity_id);
}
