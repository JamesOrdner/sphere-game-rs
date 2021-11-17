use std::collections::HashMap;

use component::Component;
use entity::EntityId;
use event::{push_event, EventListener};
use laminar::{Packet, Socket, SocketEvent};
use nalgebra_glm::{Vec2, Vec3};
use network_utils::{
    InputPacket, NetworkId, PacketType, ServerConnectPacket, StaticMeshPacket, VelocityPacket,
};
use system::Timestamp;

const SERVER: &str = "127.0.0.1:12351";

pub struct System {
    socket: Socket,
    input: Vec2,
    static_mesh_components: HashMap<NetworkId, StaticMeshComponent>,
}

struct StaticMeshComponent {
    entity_id: EntityId,
    location: Vec3,
}

impl System {
    pub fn new() -> Self {
        let socket = Socket::bind("127.0.0.1:0").unwrap();

        Self {
            socket,
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

    pub async fn simulate(&mut self, _timestamp: Timestamp) {
        let input = InputPacket::new(self.input);

        self.socket
            .send(Packet::reliable_sequenced(
                SERVER.parse().unwrap(),
                input.into(),
                None,
            ))
            .unwrap();

        self.socket.manual_poll(std::time::Instant::now());

        while let Some(message) = self.socket.recv() {
            match message {
                SocketEvent::Packet(packet) => self.handle_packet(packet.payload()),
                SocketEvent::Connect(_) => println!("client connect"),
                SocketEvent::Timeout(_) => println!("client timeout"),
                SocketEvent::Disconnect(_) => println!("client disconnect"),
            }
        }
    }

    fn handle_packet(&mut self, packet: &[u8]) {
        match PacketType::from(packet) {
            PacketType::Input => {}
            PacketType::ServerConnect => self.handle_server_connect(packet),
            PacketType::StaticMesh => self.handle_static_mesh_packet(packet),
            PacketType::Velocity => self.handle_velocity_packet(packet),
        };
    }

    fn handle_server_connect(&mut self, packet: &[u8]) {
        let packet = ServerConnectPacket::from(packet);

        push_event(0, Component::Timestamp(packet.timestamp));
    }

    fn handle_static_mesh_packet(&mut self, packet: &[u8]) {
        let packet = StaticMeshPacket::from(packet);
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

    fn handle_velocity_packet(&mut self, packet: &[u8]) {
        let packet = VelocityPacket::from(packet);
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
