use crate::message_bus::{Message, Sender};
use winit::event::{ElementState, Event, ScanCode, WindowEvent};

pub struct InputSystem {
    input_acceleration_x: f32,
    input_acceleration_y: f32,
    w_held: bool,
    a_held: bool,
    s_held: bool,
    d_held: bool,
}

impl InputSystem {
    pub fn new() -> Self {
        InputSystem {
            input_acceleration_x: 0.0,
            input_acceleration_y: 0.0,
            w_held: false,
            a_held: false,
            s_held: false,
            d_held: false,
        }
    }

    pub fn handle_input(&mut self, event: Event<()>) {
        match event {
            Event::WindowEvent {
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
        message_sender.push(Message::InputAcceleration {
            x: self.input_acceleration_x,
            y: self.input_acceleration_y,
        });
    }

    fn handle_keypress(&mut self, scancode: ScanCode, state: ElementState) {
        match scancode {
            13 | 17 => {
                if state == ElementState::Pressed {
                    self.input_acceleration_y = 1.0;
                    self.w_held = true;
                } else {
                    if self.s_held {
                        self.input_acceleration_y = -1.0;
                    } else {
                        self.input_acceleration_y = 0.0;
                    }
                    self.w_held = false;
                }
            }
            0 | 30 => {
                if state == ElementState::Pressed {
                    self.input_acceleration_x = -1.0;
                    self.a_held = true;
                } else {
                    if self.d_held {
                        self.input_acceleration_x = 1.0;
                    } else {
                        self.input_acceleration_x = 0.0;
                    }
                    self.a_held = false;
                }
            }
            1 | 31 => {
                if state == ElementState::Pressed {
                    self.input_acceleration_y = -1.0;
                    self.s_held = true;
                } else {
                    if self.w_held {
                        self.input_acceleration_y = 1.0;
                    } else {
                        self.input_acceleration_y = 0.0;
                    }
                    self.s_held = false;
                }
            }
            2 | 32 => {
                if state == ElementState::Pressed {
                    self.input_acceleration_x = 1.0;
                    self.d_held = true;
                } else {
                    if self.a_held {
                        self.input_acceleration_x = -1.0;
                    } else {
                        self.input_acceleration_x = 0.0;
                    }
                    self.d_held = false;
                }
            }
            _ => {}
        };
    }
}
