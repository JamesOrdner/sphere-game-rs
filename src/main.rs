mod common;
mod engine;
mod message_bus;
mod systems;
mod thread_pool;

use engine::Engine;

use std::vec::Vec;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--server".to_string()) {
        let engine = Engine::create_server();
        engine.run_server();
    } else {
        let event_loop = winit::event_loop::EventLoop::new();
        let engine = Engine::create_client(&event_loop);
        engine.run_client(event_loop);
    }
}
