use std::pin::Pin;

use component::Component;
use event::{EventListener, EventManager};
use task::Executor;

use crate::entity::Entity;

mod entity;

pub struct Server {
    event_manager: EventManager,
    task_executor: Executor,
    last_update: std::time::Instant,
    entities: Vec<Entity>,
    systems: Systems,
}

impl Server {
    pub fn new() -> Self {
        let (task_executor, thread_ids) = Executor::new();
        let event_manager = EventManager::new(thread_ids);

        Self {
            event_manager,
            task_executor,
            last_update: std::time::Instant::now(),
            entities: Vec::new(),
            systems: Systems::new(),
        }
    }

    pub fn run(mut self) {
        self.load_level();
        self.last_update = std::time::Instant::now();

        loop {
            self.simulate();
            self.distribute_events();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        self.shutdown();
    }

    fn simulate(&mut self) {
        const UPDATE_INTERVAL: std::time::Duration = std::time::Duration::from_micros(16_666);

        let time_now = std::time::Instant::now();
        while time_now.duration_since(self.last_update) > UPDATE_INTERVAL {
            self.last_update += UPDATE_INTERVAL;
            let mut task = self.systems.simulate();
            self.task_executor.execute_blocking(&mut task);
        }
    }

    fn distribute_events(&mut self) {
        self.event_manager.distribute(&mut self.systems);
    }

    fn load_level(&mut self) {
        self.entities
            .push(entity::create_static_mesh(0, &mut self.systems));
    }

    fn shutdown(&mut self) {
        for entity in self.entities.drain(..) {
            entity.destroy(&mut self.systems);
        }
    }
}

pub struct Systems {
    physics: sim_physics::System,
}

impl Systems {
    pub fn new() -> Self {
        Self {
            physics: sim_physics::System::new(),
        }
    }

    pub async fn simulate(&mut self) {
        let physics = self.physics.simulate();

        physics.await;
    }
}

impl EventListener for Systems {
    fn receive_event(&mut self, entity_id: ::entity::EntityID, component: &Component) {
        self.physics.receive_event(entity_id, component);
    }
}
