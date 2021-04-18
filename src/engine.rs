use crate::entity;
use crate::entity::Entity;
use crate::state_manager::StateManager;
use crate::systems::Systems;
use crate::systems::{Renderable, Updatable};
use crate::thread_pool::ThreadPool;
use winit::window::Window;

pub const UPDATE_INTERVAL: std::time::Duration = std::time::Duration::from_micros(16_666);

pub struct Engine {
    state_manager: StateManager,
    thread_pool: ThreadPool,
    last_update: std::time::Instant,
    entities: Vec<Entity>,
    systems: Systems,
}

impl Engine {
    pub fn create_client(event_loop: &winit::event_loop::EventLoop<()>) -> Self {
        let window = Window::new(&event_loop).unwrap();

        Engine {
            state_manager: StateManager::new(),
            thread_pool: ThreadPool::new(2),
            last_update: std::time::Instant::now(),
            entities: Vec::new(),
            systems: Systems::create_client_systems(window),
        }
    }

    pub fn create_server() -> Self {
        Engine {
            state_manager: StateManager::new(),
            thread_pool: ThreadPool::new(2),
            last_update: std::time::Instant::now(),
            entities: Vec::new(),
            systems: Systems::create_server_systems(),
        }
    }

    pub fn run_client(mut self, event_loop: winit::event_loop::EventLoop<()>) -> ! {
        use winit::{
            event::{Event, WindowEvent},
            event_loop::ControlFlow,
        };

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

                self.systems.client.as_mut().unwrap().input.flush_input(
                    self.thread_pool
                        .get_message_bus_senders_mut()
                        .first_mut()
                        .unwrap(),
                );
                self.distribute_events();
                self.update();
                self.distribute_events();
                self.render();
            }
            event => {
                self.systems
                    .client
                    .as_mut()
                    .unwrap()
                    .input
                    .handle_input(event);
            }
        });
    }

    pub fn run_server(mut self) {
        self.load_level();
        self.last_update = std::time::Instant::now();

        loop {
            self.update();
            self.distribute_events();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        // self.shutdown();
    }

    fn update(&mut self) {
        let time_now = std::time::Instant::now();
        while time_now.duration_since(self.last_update) > UPDATE_INTERVAL {
            self.last_update += UPDATE_INTERVAL;

            let systems = &mut self.systems;

            self.thread_pool.scoped(|thread_pool_scope| {
                systems.core.physics.update(&thread_pool_scope);
            });
        }
    }

    fn render(&mut self) {
        let core_systems = &mut self.systems.core;
        let client_systems = &mut self.systems.client;

        let delta_time = 1.0 / 60.0; // TODO

        self.thread_pool.scoped(|thread_pool_scope| {
            core_systems.physics.render(&thread_pool_scope, delta_time);

            if let Some(client_systems) = client_systems {
                client_systems.camera.render(&thread_pool_scope, delta_time);
                client_systems
                    .static_mesh
                    .render(&thread_pool_scope, delta_time);
            }
        });
    }

    fn distribute_events(&mut self) {
        self.state_manager.distribute(
            self.thread_pool.get_message_bus_senders_mut(),
            &mut self.systems,
        );
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
