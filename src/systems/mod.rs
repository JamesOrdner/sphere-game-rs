use crate::thread_pool::Scope;

pub mod camera;
pub mod graphics;
pub mod input;
pub mod physics;

pub trait Updatable {
    fn update(&mut self, thread_pool_scope: &Scope);
}

pub trait Renderable {
    fn render(&mut self, thread_pool_scope: &Scope, delta_time: f32);
}
