use std::num::Wrapping;

use component::Component;
use event::{EventListener, EventManager};
use system::{Timestamp, TIMESTEP};
use task::{run_parallel, Executor};

use crate::entity::Entity;

mod entity;

pub struct Server {
    event_manager: EventManager,
    task_executor: Executor,
    last_update: std::time::Instant,
    timestamp: Timestamp,
    entities: Vec<Entity>,
    systems: Systems,
}

impl Server {
    pub fn new() -> Self {
        let (task_executor, thread_ids) = Executor::new();
        let event_manager = EventManager::new(&thread_ids);

        Self {
            event_manager,
            task_executor,
            last_update: std::time::Instant::now(),
            timestamp: Wrapping(0),
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
        let time_now = std::time::Instant::now();
        while time_now.duration_since(self.last_update) > TIMESTEP {
            self.last_update += TIMESTEP;

            let mut frame_task = async {
                self.systems
                    .sim_network_server
                    .simulate(self.timestamp)
                    .await;

                let mut physics = self.systems.sim_physics.simulate(self.timestamp);

                run_parallel([&mut physics]).await;

                self.timestamp += Wrapping(1);
            };

            self.task_executor.execute_blocking(&mut frame_task);
        }
    }

    fn distribute_events(&mut self) {
        self.event_manager.distribute(|entity_id, component| {
            self.systems.receive_event(entity_id, component);
        });
    }

    fn load_level(&mut self) {
        self.entities
            .push(entity::static_mesh(0, &mut self.systems));
    }

    fn shutdown(&mut self) {
        for entity in self.entities.drain(..) {
            entity.destroy(&mut self.systems);
        }
    }
}

pub struct Systems {
    pub sim_network_server: sim_network_server::System,
    sim_physics: sim_physics::System,
}

impl Systems {
    pub fn new() -> Self {
        Self {
            sim_network_server: sim_network_server::System::new(),
            sim_physics: sim_physics::System::new(),
        }
    }
}

impl EventListener for Systems {
    fn receive_event(&mut self, entity_id: ::entity::EntityId, component: &Component) {
        self.sim_network_server.receive_event(entity_id, component);
        self.sim_physics.receive_event(entity_id, component);
    }
}
