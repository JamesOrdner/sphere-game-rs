use crate::{components::Component, entity::EntityID, state_manager::Listener, thread_pool::Scope};
use camera::CameraSystem;
use input::InputSystem;
use physics::PhysicsSystem;
use static_mesh::StaticMeshSystem;
use winit::window::Window;

pub mod camera;
pub mod input;
pub mod physics;
pub mod static_mesh;

pub trait Updatable {
    fn update(&mut self, thread_pool_scope: &Scope);
}

pub trait Renderable {
    fn render(&mut self, thread_pool_scope: &Scope, delta_time: f32);
}

pub enum SystemType {
    Camera,
    Input,
    Physics,
}

pub struct CoreSystems {
    pub physics: PhysicsSystem,
}

impl CoreSystems {
    fn new() -> Self {
        Self {
            physics: PhysicsSystem::new(),
        }
    }
}

pub struct ClientSystems {
    pub camera: CameraSystem,
    pub static_mesh: StaticMeshSystem,
    pub input: InputSystem,
}

impl ClientSystems {
    fn new(window: Window) -> Self {
        Self {
            camera: CameraSystem::new(),
            static_mesh: StaticMeshSystem::new(window),
            input: InputSystem::new(),
        }
    }
}

pub struct Systems {
    pub core: CoreSystems,
    pub client: Option<ClientSystems>,
}

impl Systems {
    pub fn create_server_systems() -> Self {
        Self {
            core: CoreSystems::new(),
            client: None,
        }
    }

    pub fn create_client_systems(window: Window) -> Self {
        Self {
            core: CoreSystems::new(),
            client: Some(ClientSystems::new(window)),
        }
    }

    pub fn update(&mut self, thread_pool_scope: &Scope) {
        let core = &mut self.core;

        core.physics.update(&thread_pool_scope);
    }

    pub fn render(&mut self, thread_pool_scope: &Scope, delta_time: f32) {
        let core = &mut self.core;
        let client = self.client.as_mut().unwrap();

        core.physics.render(&thread_pool_scope, delta_time);
        client.camera.render(&thread_pool_scope, delta_time);
        client.static_mesh.render(&thread_pool_scope, delta_time);
    }

    pub fn receive_for_each_listener(&mut self, entity_id: EntityID, component: &Component) {
        self.core.physics.receive(entity_id, component);

        if let Some(client_systems) = &mut self.client {
            client_systems.camera.receive(entity_id, component);
            client_systems.static_mesh.receive(entity_id, component);
        }
    }
}
