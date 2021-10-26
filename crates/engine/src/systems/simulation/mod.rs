use crate::{
    components::Component,
    entity::EntityID,
    state_manager::{EventSender, Listener},
};

use self::{camera::CameraSystem, physics::PhysicsSystem};

mod camera;
mod physics;

pub struct ClientSimulationSystem {
    pub camera: CameraSystem,
    pub physics: PhysicsSystem,
}

pub struct ServerSimulationSystem {
    pub physics: PhysicsSystem,
}

impl ClientSimulationSystem {
    pub fn new() -> Self {
        Self {
            camera: CameraSystem::new(),
            physics: PhysicsSystem::new(),
        }
    }

    pub async fn simulate(&mut self, event_sender: &EventSender) {
        let camera = self.camera.simulate(event_sender);
        let physics = self.physics.simulate(event_sender);

        camera.await;
        physics.await;
    }

    pub fn receive(&mut self, entity_id: EntityID, component: &Component) {
        self.camera.receive(entity_id, component);
        self.physics.receive(entity_id, component);
    }
}

impl ServerSimulationSystem {
    pub fn new() -> Self {
        Self {
            physics: PhysicsSystem::new(),
        }
    }

    pub async fn simulate(&mut self, event_sender: &EventSender) {
        let physics = self.physics.simulate(event_sender);

        physics.await;
    }

    pub fn receive(&mut self, entity_id: EntityID, component: &Component) {
        self.physics.receive(entity_id, component);
    }
}
