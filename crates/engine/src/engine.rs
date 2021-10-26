use std::pin::Pin;

use task::Executor;
use winit::window::Window;

use crate::entity;
use crate::entity::Entity;
use crate::state_manager::StateManager;
use crate::systems::ClientSystems;

pub const UPDATE_INTERVAL: std::time::Duration = std::time::Duration::from_micros(16_666);

pub struct Engine {
    state_manager: StateManager,
    task_executor: Executor,
    last_update: std::time::Instant,
    entities: Vec<Entity>,
    systems: ClientSystems,
}

impl Engine {
    pub fn create_client(event_loop: &winit::event_loop::EventLoop<()>) -> Self {
        let window = Window::new(&event_loop).unwrap();

        Engine {
            state_manager: StateManager::new(),
            task_executor: Executor::new(),
            last_update: std::time::Instant::now(),
            entities: Vec::new(),
            systems: ClientSystems::new(window),
        }
    }

    // pub fn create_server() -> Self {
    //     Engine {
    //         state_manager: StateManager::new(),
    //         task_executor: Executor::new(),
    //         last_update: std::time::Instant::now(),
    //         entities: Vec::new(),
    //         systems: Systems::create_server_systems(),
    //     }
    // }

    pub fn run_client(mut self, event_loop: winit::event_loop::EventLoop<()>) -> ! {
        use winit::{
            event::{Event, WindowEvent},
            event_loop::ControlFlow,
        };

        println!(
            "{}",
            std::mem::size_of_val(&self.systems.simulate(&self.state_manager.sender))
        );

        self.load_level();
        self.last_update = std::time::Instant::now();

        event_loop.run(move |event, _, control_flow| match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                self.shutdown();
                *control_flow = ControlFlow::Exit;
            }
            Event::MainEventsCleared => {
                if *control_flow == ControlFlow::Exit {
                    return;
                }

                self.systems
                    .input
                    .flush_input(&mut self.state_manager.sender);
                self.distribute_events();
                self.simulate();
                self.distribute_events();
                self.render();
            }
            event => {
                self.systems.input.handle_input(event);
            }
        });
    }

    pub fn run_server(mut self) {
        self.load_level();
        self.last_update = std::time::Instant::now();

        loop {
            self.simulate();
            self.distribute_events();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        // self.shutdown();
    }

    fn simulate(&mut self) {
        let time_now = std::time::Instant::now();
        while time_now.duration_since(self.last_update) > UPDATE_INTERVAL {
            self.last_update += UPDATE_INTERVAL;
            let mut update = self.systems.simulate(&self.state_manager.sender);
            let update = unsafe { Pin::new_unchecked(&mut update) };
            self.task_executor.execute_blocking(update);
        }
    }

    fn render(&mut self) {
        self.systems.render();
    }

    fn distribute_events(&mut self) {
        self.state_manager.distribute(&mut self.systems);
    }

    fn load_level(&mut self) {
        self.entities
            .push(entity::create_static_mesh(0, &mut self.systems));
        // self.entities
        //     .push(entity::create_camera(1, &mut self.systems));
    }

    fn shutdown(&mut self) {
        for entity in self.entities.drain(..) {
            entity.destroy(&mut self.systems);
        }
    }
}
