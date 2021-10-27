use client::Client;
use server::Server;
use winit::event_loop::EventLoop;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--server".to_string()) {
        let server = Server::new();
        server.run();
    } else {
        let event_loop = EventLoop::new();
        let client = Client::new(&event_loop);
        client.run(event_loop);
    }
}
