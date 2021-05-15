use crate::{state_manager::Listener, thread_pool::Scope};

use camera::CameraSystem;
use physics::PhysicsSystem;

mod camera;
mod physics;

pub trait GameSubsystem: Listener {
    fn update(&mut self, thread_pool_scope: &Scope);
}

pub struct GameCoreSubsystems {
    pub physics: PhysicsSystem,
}

impl GameCoreSubsystems {
    pub fn create() -> Self {
        Self {
            physics: PhysicsSystem::new(),
        }
    }

    pub fn for_each<T>(&mut self, func: T)
    where
        T: Fn(&mut dyn GameSubsystem),
    {
        func(&mut self.physics);
    }
}

pub struct GameClientSubsystems {
    pub camera: CameraSystem,
}

impl GameClientSubsystems {
    pub fn create() -> Self {
        Self {
            camera: CameraSystem::new(),
        }
    }

    pub fn for_each<T>(&mut self, func: T)
    where
        T: Fn(&mut dyn GameSubsystem),
    {
        func(&mut self.camera);
    }
}
