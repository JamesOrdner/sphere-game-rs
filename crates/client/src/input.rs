use component::{Component, InputAcceleration};
use event::push_event;
use nalgebra_glm as glm;
use system::SubsystemType;
use winit::event::{ElementState, Event as InputEvent, ScanCode, WindowEvent};

pub struct System {
    input_acceleration: InputAcceleration,
    w_held: bool,
    a_held: bool,
    s_held: bool,
    d_held: bool,
}

impl System {
    pub fn new() -> Self {
        Self {
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

    pub async fn flush_input(&self) {
        push_event(
            0,
            Component::InputAcceleration(self.input_acceleration),
            SubsystemType::Input,
        );
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
