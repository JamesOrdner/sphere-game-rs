use crate::engine::{Systems, ClientSystems};

pub type EntityID = u16;

pub struct Entity {
    pub entity_id: EntityID,
    destructor: fn(EntityID, &mut Systems, Option<&mut ClientSystems>),
}

impl Entity {
    pub fn destroy(&self, systems: &mut Systems, client_systems: Option<&mut ClientSystems>) {
        (self.destructor)(self.entity_id, systems, client_systems);
    }
}

pub fn create_static_mesh(entity_id: EntityID, systems: &mut Systems, client_systems: Option<&mut ClientSystems>) -> Entity {
    systems.physics_system.create_component(entity_id);

    if let Some(client_systems) = client_systems {
        client_systems.graphics_system.create_static_mesh_component(entity_id, "suzanne");
    }

    Entity {
        entity_id,
        destructor: destroy_static_mesh,
    }
}

fn destroy_static_mesh(entity_id: EntityID, systems: &mut Systems, client_systems: Option<&mut ClientSystems>) {
    systems.physics_system.destroy_component(entity_id);

    if let Some(client_systems) = client_systems {
        client_systems.graphics_system.destroy_static_mesh_component(entity_id);
    }
}

pub fn create_camera(entity_id: EntityID, client_systems: Option<&mut ClientSystems>) -> Entity {
    if let Some(client_systems) = client_systems {
        client_systems.camera_system.create_component(entity_id);
        client_systems.graphics_system.create_camera_component(entity_id);
    }

    Entity {
        entity_id,
        destructor: destroy_camera,
    }
}

fn destroy_camera(entity_id: EntityID, _systems: &mut Systems, client_systems: Option<&mut ClientSystems>) {
    if let Some(client_systems) = client_systems {
        client_systems.camera_system.destroy_component(entity_id);
        client_systems.graphics_system.destroy_camera_component(entity_id);
    }
}
