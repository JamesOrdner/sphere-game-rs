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
use nalgebra_glm::{Vec2, Vec3};
use network_utils::{
    InputPacket, NetworkId, PacketType, ServerConnectPacket, StaticMeshPacket, VelocityPacket,
};
use system::Timestamp;

const SERVER: &str = "127.0.0.1:12351";

const TIMESTEPS_PER_CLIENT_UPDATE: usize = 6;

pub struct System {
    _socket_thread_join: JoinHandle<()>,
    sender: Sender<Packet>,
    receiver: Receiver<SocketEvent>,
    clients: Vec<Client>,
    static_mesh_components: HashMap<NetworkId, StaticMeshComponent>,
}

struct Client {
    addr: SocketAddr,
    input: Vec2,
}

impl Client {
    fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            input: Vec2::zeros(),
        }
    }
}

struct StaticMeshComponent {
    entity_id: EntityId,
    location: Vec3,
    velocity: Vec3,
    velocity_updated: bool,
}

impl StaticMeshComponent {
    fn update_velocity(&mut self, velocity: &Vec3) {
        if self.velocity != *velocity {
            self.velocity = *velocity;
            self.velocity_updated = true;
        }
    }

    fn updated_velocity(&mut self) -> Option<&Vec3> {
        if self.velocity_updated {
            self.velocity_updated = false;
            Some(&self.velocity)
        } else {
            None
        }
    }
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
                velocity: Vec3::zeros(),
                velocity_updated: false,
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
                SocketEvent::Connect(addr) => self.clients.push(Client::new(addr)),
                SocketEvent::Timeout(_) => println!("timeout"),
                SocketEvent::Disconnect(addr) => self.clients.retain(|a| a.addr != addr),
            }
        }

        // send

        if timestamp.0 % TIMESTEPS_PER_CLIENT_UPDATE as u32 == 0 {
            // full update
            for (network_id, static_mesh) in &mut self.static_mesh_components {
                let packet = StaticMeshPacket::new(
                    timestamp,
                    *network_id,
                    static_mesh.location,
                    static_mesh.velocity,
                );
                static_mesh.velocity_updated = false;
                send_to_clients(&self.sender, &self.clients, packet.into());
            }
        } else {
            // send only velocities
            self.static_mesh_components
                .iter_mut()
                .filter_map(|(network_id, static_mesh)| {
                    static_mesh.updated_velocity().map(|vel| (network_id, vel))
                })
                .for_each(|(network_id, velocity)| {
                    let packet = VelocityPacket::new(timestamp, *network_id, *velocity);
                    send_to_clients(&self.sender, &self.clients, packet.into());
                });
        }
    }

    fn handle_packet(&mut self, packet: &Packet, timestamp: Timestamp) {
        let payload = packet.payload();
        match PacketType::from(payload) {
            PacketType::Input => self.handle_input_packet(packet),
            PacketType::ServerConnect => panic!(),
            PacketType::StaticMesh => panic!(),
            PacketType::Velocity => panic!(),
        };

        if !self.clients.iter().any(|a| a.addr == packet.addr()) {
            self.sender
                .send(Packet::reliable_ordered(
                    packet.addr(),
                    ServerConnectPacket::new(timestamp).into(),
                    None,
                ))
                .unwrap();
        }
    }

    fn handle_input_packet(&mut self, packet: &Packet) {
        // hack: only allow first client to send input
        let controller_client_addr = match self.clients.first() {
            Some(Client { addr, input: _ }) => addr,
            _ => return,
        };

        if *controller_client_addr == packet.addr() {
            let packet = InputPacket::from(packet.payload());
            push_event(0, Component::InputAcceleration(packet.input));

            // todo: send immediate velocity update to all clients
        }
    }
}

fn send_to_clients(sender: &Sender<Packet>, clients: &[Client], packet: Vec<u8>) {
    for client in clients {
        sender
            .send(Packet::reliable_sequenced(
                client.addr,
                packet.clone(),
                None,
            ))
            .unwrap();
    }
}

impl EventListener for System {
    fn receive_event(&mut self, entity_id: EntityId, component: &Component) {
        match component {
            Component::Location(location) => {
                self.static_mesh_components
                    .values_mut()
                    .find(|static_mesh| static_mesh.entity_id == entity_id)
                    .unwrap()
                    .location = *location;
            }
            Component::Velocity(velocity) => {
                self.static_mesh_components
                    .values_mut()
                    .find(|static_mesh| static_mesh.entity_id == entity_id)
                    .unwrap()
                    .update_velocity(velocity);
            }
            _ => {}
        }
    }
}
