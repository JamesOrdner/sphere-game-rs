use std::{collections::HashMap, net::SocketAddr};

use component::Component;
use entity::EntityId;
use event::{push_event, EventListener};
use laminar::{Packet as LaminarPacket, Socket, SocketEvent};
use nalgebra_glm::{Vec2, Vec3};
use network_utils::{InputPacket, NetworkId, Packet, PingPacket, StaticMeshPacket, VelocityPacket};
use system::Timestamp;

const SERVER_IP: &str = "127.0.0.1:12351";

pub struct System {
    socket: Socket,
    server_addr: SocketAddr,
    connected: bool,
    input: Vec2,
    static_mesh_components: HashMap<NetworkId, StaticMeshComponent>,
}

struct StaticMeshComponent {
    entity_id: EntityId,
    location: Vec3,
}

impl System {
    pub fn new() -> Self {
        let mut socket = Socket::bind_any().unwrap();
        let server_addr = SERVER_IP.parse().unwrap();

        // send initial connect packet to server
        socket
            .send(LaminarPacket::reliable_unordered(
                server_addr,
                Packet::EstablishConnection.into(),
            ))
            .unwrap();
        socket.manual_poll(std::time::Instant::now());

        Self {
            socket,
            server_addr,
            connected: false,
            input: Vec2::zeros(),
            static_mesh_components: HashMap::new(),
        }
    }

    pub fn create_static_mesh_component(&mut self, entity_id: EntityId) {
        self.static_mesh_components.insert(
            0,
            StaticMeshComponent {
                entity_id,
                location: Vec3::zeros(),
            },
        );
    }

    pub fn destroy_static_mesh_component(&mut self, entity_id: EntityId) {
        self.static_mesh_components
            .retain(|_, static_mesh| static_mesh.entity_id != entity_id);
    }

    pub async fn simulate(&mut self, timestamp: Timestamp) {
        if self.connected {
            let input = Packet::Input(InputPacket { input: self.input });
            self.socket
                .send(LaminarPacket::reliable_sequenced(
                    self.server_addr,
                    input.into(),
                    None,
                ))
                .unwrap();
        }

        self.socket.manual_poll(std::time::Instant::now());

        while let Some(message) = self.socket.recv() {
            match message {
                SocketEvent::Packet(packet) => self.handle_packet(packet.payload(), timestamp),
                SocketEvent::Connect(_) => self.handle_connect(),
                SocketEvent::Timeout(_) => println!("client timeout"),
                SocketEvent::Disconnect(_) => println!("client disconnect"),
            }
        }
    }

    fn handle_connect(&mut self) {
        println!("connection established");
        self.connected = true;
    }

    fn handle_packet(&mut self, packet: &[u8], timestamp: Timestamp) {
        match Packet::from(packet) {
            Packet::EstablishConnection => {}
            Packet::Ping(_) => self.handle_ping(timestamp),
            Packet::Input(_) => panic!(),
            Packet::StaticMesh(data) => self.handle_static_mesh_packet(data),
            Packet::Velocity(data) => self.handle_velocity_packet(data),
        };
    }

    fn handle_ping(&mut self, timestamp: Timestamp) {
        let packet = Packet::Ping(PingPacket { timestamp });
        let msg = LaminarPacket::reliable_unordered(self.server_addr, packet.into());
        self.socket.send(msg).unwrap();
    }

    fn handle_static_mesh_packet(&mut self, packet: StaticMeshPacket) {
        let static_mesh = self
            .static_mesh_components
            .get_mut(&packet.network_id)
            .unwrap();
        static_mesh.location = packet.location;

        push_event(
            static_mesh.entity_id,
            Component::NetStaticMeshLocation {
                timestamp: packet.timestamp,
                location: packet.location,
            },
        );

        push_event(
            static_mesh.entity_id,
            Component::NetStaticMeshVelocity {
                timestamp: packet.timestamp,
                velocity: packet.velocity,
            },
        );
    }

    fn handle_velocity_packet(&mut self, packet: VelocityPacket) {
        let static_mesh = self
            .static_mesh_components
            .get_mut(&packet.network_id)
            .unwrap();

        push_event(
            static_mesh.entity_id,
            Component::NetStaticMeshVelocity {
                timestamp: packet.timestamp,
                velocity: packet.velocity,
            },
        );
    }
}

impl EventListener for System {
    fn receive_event(&mut self, _: EntityId, component: &Component) {
        match component {
            Component::InputAcceleration(acceleration) => {
                self.input = *acceleration;
            }
            _ => {}
        }
    }
}
