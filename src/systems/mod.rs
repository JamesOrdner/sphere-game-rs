use crate::thread_pool::Scope;
use camera::CameraSystem;
use graphics::GraphicsSystem;
use input::InputSystem;
use physics::PhysicsSystem;
use winit::window::Window;

pub mod camera;
pub mod graphics;
pub mod input;
pub mod physics;

pub trait Updatable {
    fn update(&mut self, thread_pool_scope: &Scope);
}

pub trait Renderable {
    fn render(&mut self, thread_pool_scope: &Scope, delta_time: f32);
}

pub enum SystemType {
    Camera,
    Graphics,
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
    pub graphics: GraphicsSystem,
    pub input: InputSystem,
}

impl ClientSystems {
    fn new(window: Window) -> Self {
        ClientSystems {
            camera: CameraSystem::new(),
            graphics: GraphicsSystem::new(window),
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
}
