use crate::common::{ComponentArray, EntityID};
use crate::message_bus::Message;
use crate::thread_pool::Scope;
use winit::window::Window;

struct ComponentData {
    location_x: f32,
    location_y: f32,
}

pub struct GraphicsSystem {
    component_array: ComponentArray<ComponentData>,
    _window: Window,
}

impl GraphicsSystem {
    pub fn new(event_loop: &winit::event_loop::EventLoop<()>) -> Self {
        GraphicsSystem {
            component_array: ComponentArray::new(),
            _window: Window::new(&event_loop).unwrap(),
        }
    }
}

impl super::Componentable for GraphicsSystem {
    fn create_component(&mut self, entity_id: EntityID) {
        self.component_array.push(
            entity_id,
            ComponentData {
                location_x: 0.0,
                location_y: 0.0,
            },
        );
    }

    fn destroy_component(&mut self, entity_id: EntityID) {
        self.component_array.remove(entity_id);
    }
}

impl super::Renderable for GraphicsSystem {
    fn render(&mut self, thread_pool_scope: &Scope) {
        for component in &self.component_array {
            thread_pool_scope.execute(|_| {
                println!(
                    "Object {}: {} {}",
                    component.entity_id, component.data.location_x, component.data.location_y
                );
            });
        }
    }
}

impl crate::message_bus::Receiver for GraphicsSystem {
    fn receive(&mut self, messages: &[Message]) {
        for message in messages {
            match message {
                Message::Location { entity_id, x, y } => {
                    let component_data = &mut self.component_array[*entity_id].data;
                    component_data.location_x = *x;
                    component_data.location_y = *y;
                }
                _ => {}
            }
        }
    }
}
