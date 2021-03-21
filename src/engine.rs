use crate::entity;
use crate::entity::Entity;
use crate::message_bus::MessageBus;
use crate::systems::{
    camera::CameraSystem, graphics::GraphicsSystem, input::InputSystem, physics::PhysicsSystem, Renderable, Updatable,
};
use crate::thread_pool::ThreadPool;
use winit::window::Window;

pub const UPDATE_INTERVAL: std::time::Duration = std::time::Duration::from_micros(16_666);

pub struct Systems {
    pub physics_system: PhysicsSystem,
}

impl Systems {
    fn new() -> Self {
        Systems {
            physics_system: PhysicsSystem::new(),
        }
    }
}

pub struct ClientSystems {
    pub camera_system: CameraSystem,
    pub graphics_system: GraphicsSystem,
    pub input_system: InputSystem,
}

impl ClientSystems {
    fn new(window: Window) -> Self {
        ClientSystems {
            camera_system: CameraSystem::new(),
            graphics_system: GraphicsSystem::new(window),
            input_system: InputSystem::new(),
        }
    }
}

pub struct Engine {
    message_bus: MessageBus,
    thread_pool: ThreadPool,
    last_update: std::time::Instant,
    entities: Vec<Entity>,
    systems: Systems,
    client_systems: Option<ClientSystems>,
}

impl Engine {
    pub fn create_client(event_loop: &winit::event_loop::EventLoop<()>) -> Self {
        let window = Window::new(&event_loop).unwrap();

        Engine {
            message_bus: MessageBus::new(),
            thread_pool: ThreadPool::new(2),
            last_update: std::time::Instant::now(),
            entities: Vec::new(),
            systems: Systems::new(),
            client_systems: Some(ClientSystems::new(window)),
        }
    }

    pub fn create_server() -> Self {
        Engine {
            message_bus: MessageBus::new(),
            thread_pool: ThreadPool::new(2),
            last_update: std::time::Instant::now(),
            entities: Vec::new(),
            systems: Systems::new(),
            client_systems: None,
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

                self.client_systems
                    .as_mut()
                    .unwrap()
                    .input_system
                    .flush_input(
                        self.thread_pool
                            .get_message_bus_senders_mut()
                            .first_mut()
                            .unwrap(),
                    );
                self.distribute_messages();
                self.update();
                self.distribute_messages();
                self.render();
            }
            event => {
                self.client_systems
                    .as_mut()
                    .unwrap()
                    .input_system
                    .handle_input(event);
            }
        });
    }

    pub fn run_server(mut self) {
        self.load_level();
        self.last_update = std::time::Instant::now();

        loop {
            self.update();
            self.distribute_messages();
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
                systems.physics_system.update(&thread_pool_scope);
            });
        }
    }

    fn render(&mut self) {
        let systems = &mut self.systems;
        let client_systems = &mut self.client_systems;

        let delta_time = 1.0 / 60.0; // TODO

        self.thread_pool.scoped(|thread_pool_scope| {
            systems.physics_system.render(&thread_pool_scope, delta_time);

            if let Some(client_systems) = client_systems {
                client_systems.camera_system.render(&thread_pool_scope, delta_time);
                client_systems.graphics_system.render(&thread_pool_scope, delta_time);
            }
        });
    }

    fn distribute_messages(&mut self) {
        let systems = &mut self.systems;
        let client_systems = &mut self.client_systems;

        let message_bus = &self.message_bus;
        let message_senders = self.thread_pool.get_message_bus_senders();

        self.thread_pool.scoped(|thread_pool_scope| {
            thread_pool_scope.execute(|_| {
                message_bus.distribute(message_senders, &mut systems.physics_system);
            });

            if let Some(client_systems) = client_systems {
                thread_pool_scope.execute(|_| {
                    message_bus.distribute(message_senders, &mut client_systems.camera_system);
                });

                thread_pool_scope.execute(|_| {
                    message_bus.distribute(message_senders, &mut client_systems.graphics_system);
                });
            }
        });

        let message_senders_mut = self.thread_pool.get_message_bus_senders_mut();
        self.message_bus.clear_queue(message_senders_mut);
    }

    fn load_level(&mut self) {
        self.entities.push(entity::create_static_mesh(0, &mut self.systems, self.client_systems.as_mut()));
        self.entities.push(entity::create_camera(1, self.client_systems.as_mut()));
    }

    fn shutdown(&mut self) {
        for entity in self.entities.drain(..) {
            entity.destroy(&mut self.systems, self.client_systems.as_mut());
        }
    }
}
