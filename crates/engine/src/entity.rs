use crate::systems::ClientSystems;

pub type EntityID = u16;

pub struct Entity {
    pub entity_id: EntityID,
    destructor: fn(EntityID, &mut ClientSystems),
}

impl Entity {
    pub fn destroy(&self, systems: &mut ClientSystems) {
        (self.destructor)(self.entity_id, systems);
    }
}

pub fn create_static_mesh(entity_id: EntityID, systems: &mut ClientSystems) -> Entity {
    systems.simulation.physics.create_component(entity_id);

    systems
        .render
        .subsystems
        .static_mesh
        .create_component(entity_id, "suzanne");

    Entity {
        entity_id,
        destructor: destroy_static_mesh,
    }
}

fn destroy_static_mesh(entity_id: EntityID, systems: &mut ClientSystems) {
    systems.simulation.physics.destroy_component(entity_id);

    systems
        .render
        .subsystems
        .static_mesh
        .destroy_component(entity_id);
}
