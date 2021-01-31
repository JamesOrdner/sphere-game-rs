use crate::common::EntityID;
use crate::thread_pool::Scope;

pub mod graphics;
pub mod input;
pub mod physics;

pub trait Componentable {
    fn create_component(&mut self, entity_id: EntityID);
    fn destroy_component(&mut self, entity_id: EntityID);
}

pub trait Updatable {
    fn update(&mut self, thread_pool_scope: &Scope);
}

pub trait Renderable {
    fn render(&mut self, thread_pool_scope: &Scope);
}
