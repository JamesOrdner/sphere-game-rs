use std::num::Wrapping;

use ::entity::EntityId;
use component::Component;
use event::{EventListener, EventManager};
use gfx::Graphics;
use system::{Timestamp, TIMESTEP};
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
    last_sim_instant: std::time::Instant,
    last_frame_instant: std::time::Instant,
    timestamp: Timestamp,
    entities: Vec<Entity>,
    graphics: Graphics,
    systems: Systems,
}

impl Client {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let window = Window::new(event_loop).unwrap();

        let (task_executor, thread_ids) = Executor::new();
        let event_manager = EventManager::new(&thread_ids);

        Self {
            event_manager,
            task_executor,
            last_sim_instant: std::time::Instant::now(),
            last_frame_instant: std::time::Instant::now(),
            timestamp: Wrapping(0),
            entities: Vec::new(),
            graphics: Graphics::new(window, &thread_ids),
            systems: Systems::new(),
        }
    }

    pub fn run(mut self, event_loop: EventLoop<()>) -> ! {
        self.load_level();

        self.last_sim_instant = std::time::Instant::now();
        self.last_frame_instant = std::time::Instant::now();

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
        self.event_manager.distribute(|entity_id, component| {
            if let Component::Timestamp(timestamp) = component {
                self.timestamp = *timestamp;
            }

            self.systems.receive_event(entity_id, component);
        });
    }

    fn frame(&mut self) {
        self.flush_input();
        self.distribute_events();

        let now = std::time::Instant::now();
        let frame_delta_time = now.duration_since(self.last_frame_instant).as_secs_f32();
        self.last_frame_instant = now;

        let mut frame_task = async {
            let mut simulation = async {
                while now.duration_since(self.last_sim_instant) > TIMESTEP {
                    self.last_sim_instant += TIMESTEP;

                    let mut camera = self.systems.sim_camera.simulate(frame_delta_time);
                    let mut network_client =
                        self.systems.sim_network_client.simulate(self.timestamp);
                    let mut physics = self.systems.sim_physics.simulate(self.timestamp);

                    run_parallel([&mut camera, &mut network_client, &mut physics]).await;

                    self.timestamp += Wrapping(1);
                }
            };

            let mut graphics = async {
                self.graphics.frame_begin().await;

                // run sequentially until we get secondary command buffers up and running
                self.systems.gfx_camera.render().await;
                self.systems.gfx_static_mesh.render().await;

                self.graphics.frame_end().await;
            };

            run_parallel([&mut simulation, &mut graphics]).await;
        };

        // println!("{}", std::mem::size_of_val(&frame_task));

        self.task_executor.execute_blocking(&mut frame_task);
    }

    fn load_level(&mut self) {
        let static_mesh = self.graphics.create_static_mesh("suzanne");
        self.entities
            .push(entity::static_mesh(10, &mut self.systems, static_mesh));
        self.entities.push(entity::camera(20, &mut self.systems));

        self.systems.sim_camera.set_target(10);
    }

    fn shutdown(&mut self) {
        for entity in self.entities.drain(..) {
            entity.destroy(&mut self.systems);
        }
    }
}

impl EventListener for Client {
    fn receive_event(&mut self, entity_id: EntityId, component: &Component) {
        if let Component::Timestamp(timestamp) = component {
            self.timestamp = *timestamp;
        }

        self.systems.receive_event(entity_id, component);
    }
}

pub struct Systems {
    pub input: input::System,
    pub sim_camera: sim_camera::System,
    pub sim_network_client: sim_network_client::System,
    pub sim_physics: sim_physics::System,
    pub gfx_camera: gfx_camera::System,
    pub gfx_static_mesh: gfx_static_mesh::System,
}

impl Systems {
    pub fn new() -> Self {
        Self {
            input: input::System::new(),
            sim_camera: sim_camera::System::new(),
            sim_network_client: sim_network_client::System::new(),
            sim_physics: sim_physics::System::new(),
            gfx_camera: gfx_camera::System::new(),
            gfx_static_mesh: gfx_static_mesh::System::new(),
        }
    }
}

impl EventListener for Systems {
    fn receive_event(&mut self, entity_id: EntityId, component: &Component) {
        self.sim_camera.receive_event(entity_id, component);
        self.sim_network_client.receive_event(entity_id, component);
        self.sim_physics.receive_event(entity_id, component);
        self.gfx_camera.receive_event(entity_id, component);
        self.gfx_static_mesh.receive_event(entity_id, component);
    }
}
