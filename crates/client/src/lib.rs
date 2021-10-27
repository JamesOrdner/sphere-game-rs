use std::pin::Pin;

use component::Component;
use event::{EventListener, EventManager};
use task::Executor;
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
        println!(
            "{}",
            std::mem::size_of_val(&self.systems.simulate_and_render())
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

                self.flush_input();
                self.distribute_events();
                self.simulate_and_render();
            }
            event => {
                self.systems.input.handle_input(event);
            }
        });
    }

    fn flush_input(&mut self) {
        let mut task = self.systems.input.flush_input();
        let task = unsafe { Pin::new_unchecked(&mut task) };
        self.task_executor.execute_blocking(task);
    }

    fn distribute_events(&mut self) {
        self.event_manager.distribute(&mut self.systems);
    }

    // fn simulate(&mut self) {
    //     const UPDATE_INTERVAL: std::time::Duration = std::time::Duration::from_micros(16_666);

    //     let time_now = std::time::Instant::now();
    //     while time_now.duration_since(self.last_update) > UPDATE_INTERVAL {
    //         self.last_update += UPDATE_INTERVAL;
    //         let mut task = self.systems.simulate();
    //         let task = unsafe { Pin::new_unchecked(&mut task) };
    //         self.task_executor.execute_blocking(task);
    //     }
    // }

    // fn render(&mut self) {
    //     let mut task = self.systems.render();
    //     let task = unsafe { Pin::new_unchecked(&mut task) };
    //     self.task_executor.execute_blocking(task);
    // }

    fn simulate_and_render(&mut self) {
        let mut task = self.systems.simulate_and_render();
        let task = unsafe { Pin::new_unchecked(&mut task) };
        self.task_executor.execute_blocking(task);
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

pub struct Systems {
    input: input::System,
    gfx_static_mesh: gfx_static_mesh::System,
    sim_camera: sim_camera::System,
    sim_physics: sim_physics::System,
}

impl Systems {
    pub fn new(window: Window) -> Self {
        Self {
            input: input::System::new(),
            gfx_static_mesh: gfx_static_mesh::System::new(window),
            sim_camera: sim_camera::System::new(),
            sim_physics: sim_physics::System::new(),
        }
    }

    pub async fn simulate(&mut self) {
        let camera = self.sim_camera.simulate();
        let physics = self.sim_physics.simulate();

        camera.await;
        physics.await;
    }

    pub async fn render(&mut self) {
        let static_mesh = self.gfx_static_mesh.render();

        static_mesh.await;
    }

    pub async fn simulate_and_render(&mut self) {
        let simulate = async {
            let camera = self.sim_camera.simulate();
            let physics = self.sim_physics.simulate();

            camera.await;
            physics.await;
        };

        let render = async {
            let static_mesh = self.gfx_static_mesh.render();

            static_mesh.await;
        };

        // todo: join rather than await sequentially
        simulate.await;
        render.await;
    }
}

impl EventListener for Systems {
    fn receive_event(&mut self, entity_id: ::entity::EntityID, component: &Component) {
        self.gfx_static_mesh.receive_event(entity_id, component);
        self.sim_camera.receive_event(entity_id, component);
        self.sim_physics.receive_event(entity_id, component);
    }
}
