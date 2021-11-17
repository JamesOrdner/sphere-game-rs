use std::{num::Wrapping, time::Duration};

pub const STEPS_PER_SECOND: usize = 60;

pub const TIMESTEP: Duration = Duration::from_micros(16_667);

pub const TIMESTEP_F32: f32 = 1.0 / STEPS_PER_SECOND as f32;

pub type Timestamp = Wrapping<u32>;
