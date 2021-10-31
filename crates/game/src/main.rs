use client::Client;
use server::Server;
use winit::event_loop::EventLoop;

fn main() {
    if std::env::args().any(|arg| arg == "--server") {
        let server = Server::new();
        server.run();
    } else {
        let event_loop = EventLoop::new();
        let client = Client::new(&event_loop);
        client.run(event_loop);
    }
}
