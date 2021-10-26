use crate::state_manager::Listener;
use static_mesh::StaticMeshSystem;
use winit::window::Window;

pub mod static_mesh;

pub trait RenderSubsystem: Listener {
    fn render(&mut self);
}

pub struct RenderSubsystems {
    pub static_mesh: StaticMeshSystem,
}

impl RenderSubsystems {
    pub fn create(window: Window) -> Self {
        Self {
            static_mesh: StaticMeshSystem::new(window),
        }
    }

    pub fn for_each<T>(&mut self, func: T)
    where
        T: Fn(&mut dyn RenderSubsystem),
    {
        func(&mut self.static_mesh);
    }
}
