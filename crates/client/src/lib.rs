use std::num::Wrapping;

use ::entity::EntityId;
use component::Component;
use event::{EventListener, EventManager};
use gfx::Graphics;
use system::{Timestamp, TIMESTEP, TIMESTEP_F32};
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

        let event_manager = EventManager::new();
        let (task_executor, thread_ids) = Executor::new(|| unsafe {
            event::add_event_sender();
        });

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
            self.systems.receive_event(entity_id, component);
        });
    }

    fn frame(&mut self) {
        self.flush_input();
        self.distribute_events();

        let now = std::time::Instant::now();
        let delta_time = now.duration_since(self.last_frame_instant).as_secs_f32();
        self.last_frame_instant = now;

        // simulate

        while now.duration_since(self.last_sim_instant) > TIMESTEP {
            self.last_sim_instant += TIMESTEP;

            {
                let mut simulate = self.systems.simulation.simulate(self.timestamp);
                self.task_executor.execute_blocking(&mut simulate);
            }

            self.distribute_events();

            self.timestamp += Wrapping(1);
        }

        // todo: we cannot parallelize simulation and rendering because of event distribution

        // render

        let mut render = async {
            let rem = TIMESTEP - now.duration_since(self.last_sim_instant);
            let frame_interp = 1.0 - rem.as_secs_f32() / TIMESTEP_F32;

            let mut simulation = self.systems.simulation.render(delta_time, frame_interp);

            let mut graphics = async {
                self.graphics.frame_begin().await;
                self.systems.graphics.render().await;
                self.graphics.frame_end().await;
            };

            run_parallel([&mut simulation, &mut graphics]).await;
        };

        self.task_executor.execute_blocking(&mut render);
    }

    fn load_level(&mut self) {
        let static_mesh = self.graphics.create_static_mesh("suzanne");
        self.entities
            .push(entity::static_mesh(10, &mut self.systems, static_mesh));
        self.entities.push(entity::camera(20, &mut self.systems));

        self.systems.simulation.camera.set_target(10);
    }

    fn shutdown(&mut self) {
        for entity in self.entities.drain(..) {
            entity.destroy(&mut self.systems);
        }
    }
}

impl EventListener for Client {
    fn receive_event(&mut self, entity_id: EntityId, component: &Component) {
        self.systems.receive_event(entity_id, component);
    }
}

pub struct Systems {
    pub input: input::System,
    pub simulation: SimulationSystems,
    pub graphics: GraphicsSystems,
}

impl Systems {
    pub fn new() -> Self {
        Self {
            input: input::System::new(),
            simulation: SimulationSystems::new(),
            graphics: GraphicsSystems::new(),
        }
    }
}

impl EventListener for Systems {
    fn receive_event(&mut self, entity_id: EntityId, component: &Component) {
        self.simulation.receive_event(entity_id, component);
        self.graphics.receive_event(entity_id, component);
    }
}

pub struct SimulationSystems {
    pub camera: sim_camera::System,
    pub network_client: sim_network_client::System,
    pub physics: sim_physics::System,
}

impl SimulationSystems {
    pub fn new() -> Self {
        Self {
            camera: sim_camera::System::new(),
            network_client: sim_network_client::System::new(),
            physics: sim_physics::System::new(),
        }
    }

    pub async fn simulate(&mut self, timestamp: Timestamp) {
        let mut network_client = self.network_client.simulate(timestamp);
        let mut physics = self.physics.simulate(timestamp);

        run_parallel([&mut network_client, &mut physics]).await;
    }

    pub async fn render(&mut self, delta_time: f32, frame_interp: f32) {
        let mut camera = self.camera.render(delta_time);
        let mut physics = self.physics.render(frame_interp);

        run_parallel([&mut camera, &mut physics]).await;
    }
}

impl EventListener for SimulationSystems {
    fn receive_event(&mut self, entity_id: EntityId, component: &Component) {
        self.camera.receive_event(entity_id, component);
        self.network_client.receive_event(entity_id, component);
        self.physics.receive_event(entity_id, component);
    }
}

pub struct GraphicsSystems {
    pub camera: gfx_camera::System,
    pub static_mesh: gfx_static_mesh::System,
}

impl GraphicsSystems {
    pub fn new() -> Self {
        Self {
            camera: gfx_camera::System::new(),
            static_mesh: gfx_static_mesh::System::new(),
        }
    }

    pub async fn render(&mut self) {
        // run sequentially until we get secondary command buffers up and running
        self.camera.render().await;
        self.static_mesh.render().await;
    }
}

impl EventListener for GraphicsSystems {
    fn receive_event(&mut self, entity_id: EntityId, component: &Component) {
        self.camera.receive_event(entity_id, component);
        self.static_mesh.receive_event(entity_id, component);
    }
}
