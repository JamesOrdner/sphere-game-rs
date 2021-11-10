use std::num::Wrapping;

pub const TIMESTEP: std::time::Duration = std::time::Duration::from_micros(16_667);

pub const TIMESTEP_F32: f32 = 1.0 / 60.0;

pub type Timestamp = Wrapping<u32>;
