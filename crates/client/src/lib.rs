use ::entity::EntityId;
use component::Component;
use event::{EventListener, EventManager};
use task::{run_parallel, Executor};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use crate::entity::Entity;

mod entity;
mod input;

pub struct Client {
    event_manager: EventManager,
    task_executor: Executor,
    last_update: std::time::Instant,
    entities: Vec<Entity>,
    systems: Systems,
}

impl Client {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let window = Window::new(&event_loop).unwrap();

        let (task_executor, thread_ids) = Executor::new();
        let event_manager = EventManager::new(thread_ids);

        Self {
            event_manager,
            task_executor,
            last_update: std::time::Instant::now(),
            entities: Vec::new(),
            systems: Systems::new(window),
        }
    }

    pub fn run(mut self, event_loop: EventLoop<()>) -> ! {
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

                self.frame();
            }
            event => {
                self.systems.input.handle_input(event);
            }
        });
    }

    fn flush_input(&mut self) {
        let mut task = self.systems.input.flush_input();
        self.task_executor.execute_blocking(&mut task);
    }

    fn distribute_events(&mut self) {
        self.event_manager.distribute(&mut self.systems);
    }

    fn frame(&mut self) {
        self.flush_input();
        self.distribute_events();

        let time_now = std::time::Instant::now();
        let delta_time = time_now.duration_since(self.last_update).as_secs_f32();
        self.last_update = time_now;

        let mut frame_task = async {
            let mut simulation = async {
                let mut camera = self.systems.sim_camera.simulate(delta_time);
                let mut physics = self.systems.sim_physics.simulate(delta_time);

                run_parallel([&mut camera, &mut physics]).await;
            };

            let mut graphics = async {
                let mut static_mesh = self.systems.gfx_static_mesh.render();

                run_parallel([&mut static_mesh]).await;
            };

            run_parallel([&mut simulation, &mut graphics]).await;
        };

        self.task_executor.execute_blocking(&mut frame_task);
    }

    fn load_level(&mut self) {
        self.entities
            .push(entity::static_mesh(10, &mut self.systems));
        self.entities.push(entity::camera(20, &mut self.systems));
    }

    fn shutdown(&mut self) {
        for entity in self.entities.drain(..) {
            entity.destroy(&mut self.systems);
        }
    }
}

pub struct Systems {
    pub input: input::System,
    pub sim_camera: sim_camera::System,
    pub sim_physics: sim_physics::System,
    pub gfx_static_mesh: gfx_static_mesh::System,
}

impl Systems {
    pub fn new(window: Window) -> Self {
        Self {
            input: input::System::new(),
            sim_camera: sim_camera::System::new(),
            sim_physics: sim_physics::System::new(),
            gfx_static_mesh: gfx_static_mesh::System::new(window),
        }
    }
}

impl EventListener for Systems {
    fn receive_event(&mut self, entity_id: EntityId, component: &Component) {
        self.sim_camera.receive_event(entity_id, component);
        self.sim_physics.receive_event(entity_id, component);
        self.gfx_static_mesh.receive_event(entity_id, component);
    }
}
