use crate::{components::Component, entity::EntityID, thread_pool::Scope};
use game::GameSystem;
use input::InputSystem;
use render::RenderSystem;

use winit::window::Window;

mod game;
mod input;
mod render;

// Variants lower on the list will have higher state priority
#[derive(PartialEq, PartialOrd)]
pub enum SubsystemType {
    Camera,
    Input,
    Physics,
}

pub struct Systems {
    pub input: Option<InputSystem>,
    pub game: GameSystem,
    pub render: Option<RenderSystem>,
}

impl Systems {
    pub fn create_server_systems() -> Self {
        Self {
            input: None,
            game: GameSystem::create_server(),
            render: None,
        }
    }

    pub fn create_client_systems(window: Window) -> Self {
        Self {
            input: Some(InputSystem::new()),
            game: GameSystem::create_client(),
            render: Some(render::create_system(window)),
        }
    }

    pub fn update(&mut self, thread_pool_scope: &Scope) {
        self.game
            .core_subsystems
            .for_each(|subsystem| subsystem.update(thread_pool_scope));

        if let Some(client_subsystems) = self.game.client_subsystems.as_mut() {
            client_subsystems.for_each(|subsystem| subsystem.update(thread_pool_scope));
        }
    }

    pub fn render(&mut self, thread_pool_scope: &Scope) {
        if let Some(render_system) = self.render.as_mut() {
            render_system
                .subsystems
                .for_each(|subsystem| subsystem.render(&thread_pool_scope));
        }
    }

    pub fn receive(&mut self, entity_id: EntityID, component: &Component) {
        self.game
            .core_subsystems
            .for_each(|subsystem| subsystem.receive(entity_id, component));

        if let Some(client_subsystems) = self.game.client_subsystems.as_mut() {
            client_subsystems.for_each(|subsystem| subsystem.receive(entity_id, component));
        }

        if let Some(render_system) = self.render.as_mut() {
            render_system
                .subsystems
                .for_each(|subsystem| subsystem.receive(entity_id, component));
        }
    }
}
