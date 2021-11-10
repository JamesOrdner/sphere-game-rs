use std::{
    collections::HashMap,
    net::SocketAddr,
    thread::{self, JoinHandle},
};

use component::Component;
use crossbeam_channel::{Receiver, Sender};
use entity::EntityId;
use event::{push_event, EventListener};
use laminar::{Packet, Socket, SocketEvent};
use nalgebra_glm::Vec3;
use network_utils::{InputPacket, NetworkId, PacketType, ServerConnectPacket, StaticMeshPacket};
use system::Timestamp;

const SERVER: &str = "127.0.0.1:12351";

pub struct System {
    _socket_thread_join: JoinHandle<()>,
    sender: Sender<Packet>,
    receiver: Receiver<SocketEvent>,
    clients: Vec<SocketAddr>,
    static_mesh_components: HashMap<NetworkId, StaticMeshComponent>,
}

struct StaticMeshComponent {
    entity_id: EntityId,
    location: Vec3,
}

impl System {
    pub fn new() -> Self {
        let mut socket = Socket::bind(SERVER).unwrap();

        let sender = socket.get_packet_sender();
        let receiver = socket.get_event_receiver();

        let _socket_thread_join = thread::spawn(move || {
            socket.start_polling();
        });

        Self {
            _socket_thread_join,
            sender,
            receiver,
            clients: Vec::new(),
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
        // recv

        while let Ok(msg) = self.receiver.try_recv() {
            match msg {
                SocketEvent::Packet(packet) => self.handle_packet(&packet, timestamp),
                SocketEvent::Connect(addr) => self.clients.push(addr),
                SocketEvent::Timeout(_) => println!("timeout"),
                SocketEvent::Disconnect(addr) => self.clients.retain(|a| *a != addr),
            }
        }

        // send

        if timestamp.0 % 6 > 0 {
            return;
        }

        for (network_id, static_mesh) in &mut self.static_mesh_components {
            let packet = StaticMeshPacket::new(timestamp, *network_id, static_mesh.location);
            send_to_clients(&mut self.sender, &self.clients, packet.into());
        }
    }

    fn handle_packet(&mut self, packet: &Packet, timestamp: Timestamp) {
        let payload = packet.payload();
        match PacketType::from(payload) {
            PacketType::Input => self.handle_input_packet(payload),
            PacketType::ServerConnect => panic!(),
            PacketType::StaticMesh => {}
        };

        if !self.clients.contains(&packet.addr()) {
            self.sender
                .send(Packet::reliable_ordered(
                    packet.addr(),
                    ServerConnectPacket::new(timestamp).into(),
                    None,
                ))
                .unwrap();
        }
    }

    fn handle_input_packet(&mut self, packet: &[u8]) {
        let packet = InputPacket::from(packet);
        push_event(0, Component::InputAcceleration(packet.input));
    }
}

fn send_to_clients(sender: &mut Sender<Packet>, clients: &[SocketAddr], packet: Vec<u8>) {
    for client in clients {
        sender
            .send(Packet::reliable_sequenced(*client, packet.clone(), None))
            .unwrap();
    }
}

impl EventListener for System {
    fn receive_event(&mut self, entity_id: EntityId, component: &Component) {
        match component {
            Component::Location(location) => {
                self.static_mesh_components
                    .iter_mut()
                    .find(|(_, static_mesh)| static_mesh.entity_id == entity_id)
                    .unwrap()
                    .1
                    .location = *location;
            }
            _ => {}
        }
    }
}
