use crate::{
    components::{Component, InputAcceleration},
    state_manager::{Event, Sender},
    systems::SubsystemType,
};

use nalgebra_glm as glm;
use winit::event::{ElementState, Event as InputEvent, ScanCode, WindowEvent};

pub struct InputSystem {
    input_acceleration: InputAcceleration,
    w_held: bool,
    a_held: bool,
    s_held: bool,
    d_held: bool,
}

impl InputSystem {
    pub fn new() -> Self {
        InputSystem {
            input_acceleration: glm::Vec2::zeros(),
            w_held: false,
            a_held: false,
            s_held: false,
            d_held: false,
        }
    }

    pub fn handle_input(&mut self, event: InputEvent<()>) {
        match event {
            InputEvent::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input,
                        is_synthetic: false,
                        ..
                    },
                ..
            } => {
                self.handle_keypress(input.scancode, input.state);
            }
            _ => {}
        };
    }

    pub fn flush_input(&self, message_sender: &mut Sender) {
        message_sender.push(Event {
            entity_id: 0,
            component: Component::InputAcceleration(self.input_acceleration),
            system_type: SubsystemType::Input,
        });
    }

    fn handle_keypress(&mut self, scancode: ScanCode, state: ElementState) {
        match scancode {
            13 | 17 => {
                if state == ElementState::Pressed {
                    self.input_acceleration.y = 1.0;
                    self.w_held = true;
                } else {
                    if self.s_held {
                        self.input_acceleration.y = -1.0;
                    } else {
                        self.input_acceleration.y = 0.0;
                    }
                    self.w_held = false;
                }
            }
            0 | 30 => {
                if state == ElementState::Pressed {
                    self.input_acceleration.x = -1.0;
                    self.a_held = true;
                } else {
                    if self.d_held {
                        self.input_acceleration.x = 1.0;
                    } else {
                        self.input_acceleration.x = 0.0;
                    }
                    self.a_held = false;
                }
            }
            1 | 31 => {
                if state == ElementState::Pressed {
                    self.input_acceleration.y = -1.0;
                    self.s_held = true;
                } else {
                    if self.w_held {
                        self.input_acceleration.y = 1.0;
                    } else {
                        self.input_acceleration.y = 0.0;
                    }
                    self.s_held = false;
                }
            }
            2 | 32 => {
                if state == ElementState::Pressed {
                    self.input_acceleration.x = 1.0;
                    self.d_held = true;
                } else {
                    if self.a_held {
                        self.input_acceleration.x = -1.0;
                    } else {
                        self.input_acceleration.x = 0.0;
                    }
                    self.d_held = false;
                }
            }
            _ => {}
        };
    }
}
