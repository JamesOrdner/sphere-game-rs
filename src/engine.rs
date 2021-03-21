use crate::common::EntityID;
use crate::message_bus::MessageBus;
use crate::systems::{
    graphics::GraphicsSystem, input::InputSystem, physics::PhysicsSystem, Componentable,
    Renderable, Updatable,
};
use crate::thread_pool::ThreadPool;

pub const UPDATE_INTERVAL: std::time::Duration = std::time::Duration::from_micros(16_666);

pub struct Engine {
    message_bus: MessageBus,
    thread_pool: ThreadPool,
    last_update: std::time::Instant,
    entities: Vec<EntityID>,
    input_system: Option<InputSystem>,
    graphics_system: Option<GraphicsSystem>,
    physics_system: PhysicsSystem,
}

impl Engine {
    pub fn create_client(event_loop: &winit::event_loop::EventLoop<()>) -> Self {
        Engine {
            message_bus: MessageBus::new(),
            thread_pool: ThreadPool::new(2),
            last_update: std::time::Instant::now(),
            entities: Vec::new(),
            input_system: Some(InputSystem::new()),
            graphics_system: Some(GraphicsSystem::new(&event_loop)),
            physics_system: PhysicsSystem::new(),
        }
    }

    pub fn create_server() -> Self {
        Engine {
            message_bus: MessageBus::new(),
            thread_pool: ThreadPool::new(2),
            last_update: std::time::Instant::now(),
            entities: Vec::new(),
            input_system: None,
            graphics_system: None,
            physics_system: PhysicsSystem::new(),
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

                self.input_system.as_mut().unwrap().flush_input(
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
                self.input_system.as_mut().unwrap().handle_input(event);
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

            let physics_system = &mut self.physics_system;

            self.thread_pool.scoped(|thread_pool_scope| {
                physics_system.update(&thread_pool_scope);
            });
        }
    }

    fn render(&mut self) {
        let physics_system = &mut self.physics_system;
        let graphics_system = self.graphics_system.as_mut().unwrap();

        self.thread_pool.scoped(|thread_pool_scope| {
            physics_system.render(&thread_pool_scope);
            graphics_system.render(&thread_pool_scope);
        });
    }

    fn distribute_messages(&mut self) {
        let message_bus = &self.message_bus;
        let physics_system = &mut self.physics_system;
        let graphics_system = self.graphics_system.as_mut().unwrap();

        let message_senders = self.thread_pool.get_message_bus_senders();
        self.thread_pool.scoped(|thread_pool_scope| {
            thread_pool_scope.execute(|_| {
                message_bus.distribute(message_senders, graphics_system);
            });

            thread_pool_scope.execute(|_| {
                message_bus.distribute(message_senders, physics_system);
            });
        });

        let message_senders_mut = self.thread_pool.get_message_bus_senders_mut();
        self.message_bus.clear_queue(message_senders_mut);
    }

    fn load_level(&mut self) {
        let entity_id: EntityID = 0;
        self.physics_system.create_component(entity_id);
        self.graphics_system
            .as_mut()
            .unwrap()
            .create_static_mesh_component(entity_id, "suzanne");
        self.entities.push(entity_id);
    }

    fn shutdown(&mut self) {
        for entity_id in self.entities.drain(..) {
            self.physics_system.destroy_component(entity_id);
            self.graphics_system
                .as_mut()
                .unwrap()
                .destroy_static_mesh_component(entity_id);
        }
    }
}
