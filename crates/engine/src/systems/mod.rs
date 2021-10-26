use winit::window::Window;

use crate::{components::Component, entity::EntityID, state_manager::EventSender};

use self::{
    input::InputSystem,
    render::RenderSystem,
    simulation::{ClientSimulationSystem, ServerSimulationSystem},
};

mod input;
mod render;
mod simulation;

// Variants lower on the list will have higher state priority
#[derive(PartialEq, PartialOrd)]
pub enum SubsystemType {
    Camera,
    Input,
    Physics,
}

pub struct ClientSystems {
    pub input: InputSystem,
    pub simulation: ClientSimulationSystem,
    pub render: RenderSystem,
}

pub struct ServerSystems {
    pub simulation: ServerSimulationSystem,
}

impl ClientSystems {
    pub fn new(window: Window) -> Self {
        Self {
            input: InputSystem::new(),
            simulation: ClientSimulationSystem::new(),
            render: RenderSystem::new(window),
        }
    }

    pub async fn simulate(&mut self, event_sender: &EventSender) {
        self.simulation.simulate(event_sender).await;
    }

    pub fn render(&mut self) {
        self.render
            .subsystems
            .for_each(|subsystem| subsystem.render());
    }

    pub fn receive(&mut self, entity_id: EntityID, component: &Component) {
        self.simulation.receive(entity_id, component);

        self.render
            .subsystems
            .for_each(|subsystem| subsystem.receive(entity_id, component));
    }
}
