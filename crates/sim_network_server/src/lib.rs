use std::{
    collections::HashMap,
    net::SocketAddr,
    num::Wrapping,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use component::Component;
use crossbeam_channel::{Receiver, Sender};
use entity::EntityId;
use event::{push_event, EventListener};
use laminar::{Packet as LaminarPacket, Socket, SocketEvent};
use nalgebra_glm::Vec3;
use network_utils::{
    InputPacket, NetworkId, Packet, PingPacket, StaticMeshPacket, TimestampOffset, VelocityPacket,
};
use system::{Timestamp, TIMESTEP_F32};

const SERVER: &str = "127.0.0.1:12351";

const TIMESTEPS_PER_CLIENT_UPDATE: usize = 6;

pub struct System {
    _socket_thread_join: JoinHandle<()>,
    sender: Sender<LaminarPacket>,
    receiver: Receiver<SocketEvent>,
    clients: Vec<Client>,
    static_mesh_components: HashMap<NetworkId, StaticMeshComponent>,
}

struct Client {
    addr: SocketAddr,
    timestamp_offset: Timestamp,
    last_recv_instant: Instant,
    ping: Duration,
    ping_timestamps: Timestamp,
}

impl Client {
    fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            timestamp_offset: Wrapping(0),
            last_recv_instant: Instant::now(),
            ping: Duration::ZERO,
            ping_timestamps: Wrapping(0),
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
                SocketEvent::Connect(addr) => self.handle_connect(addr),
                SocketEvent::Timeout(_) => println!("timeout"),
                SocketEvent::Disconnect(addr) => self.clients.retain(|a| a.addr != addr),
            }
        }

        // send

        if timestamp.0 % TIMESTEPS_PER_CLIENT_UPDATE as u32 == 0 {
            // full update
            for (network_id, static_mesh) in &mut self.static_mesh_components {
                let packet = StaticMeshPacket {
                    timestamp,
                    network_id: *network_id,
                    location: static_mesh.location,
                    velocity: static_mesh.velocity,
                };
                static_mesh.velocity_updated = false;
                send_to_clients(&self.sender, &self.clients, packet);
            }
        } else {
            // send only velocities
            self.static_mesh_components
                .iter_mut()
                .filter_map(|(network_id, static_mesh)| {
                    static_mesh.updated_velocity().map(|vel| (network_id, vel))
                })
                .for_each(|(network_id, velocity)| {
                    let packet = VelocityPacket {
                        timestamp,
                        network_id: *network_id,
                        velocity: *velocity,
                    };
                    send_to_clients(&self.sender, &self.clients, packet);
                });
        }
    }

    fn handle_connect(&mut self, addr: SocketAddr) {
        println!("connection established");

        let mut client = Client::new(addr);
        client.last_recv_instant = Instant::now();
        self.clients.push(client);

        let packet = Packet::Ping(PingPacket {
            timestamp: Wrapping(0),
        });
        let msg = LaminarPacket::reliable_unordered(addr, packet.into());
        self.sender.send(msg).unwrap();
    }

    fn handle_packet(&mut self, packet: &LaminarPacket, timestamp: Timestamp) {
        let payload = packet.payload();
        match Packet::from(payload) {
            Packet::EstablishConnection => self.handle_establish_connection(packet.addr()),
            Packet::Input(data) => self.handle_input_packet(data, packet.addr(), timestamp),
            Packet::Ping(data) => self.handle_ping(data, packet.addr(), timestamp),
            Packet::StaticMesh(_) => panic!(),
            Packet::Velocity(_) => panic!(),
        };
    }

    fn handle_establish_connection(&mut self, addr: SocketAddr) {
        println!("initial client connection request received");

        self.sender
            .send(LaminarPacket::reliable_unordered(
                addr,
                Packet::EstablishConnection.into(),
            ))
            .unwrap();
    }

    fn handle_ping(&mut self, packet: PingPacket, addr: SocketAddr, timestamp: Timestamp) {
        let client = self.clients.iter_mut().find(|c| c.addr == addr).unwrap();

        let now = Instant::now();
        client.ping = now.duration_since(client.last_recv_instant);
        client.ping_timestamps =
            Wrapping((client.ping.as_secs_f32() / TIMESTEP_F32).round() as u32 / 2);

        client.timestamp_offset = packet.timestamp - timestamp - client.ping_timestamps;
    }

    fn handle_input_packet(&mut self, packet: InputPacket, addr: SocketAddr, timestamp: Timestamp) {
        // hack: only allow first client to send input
        let client = match self.clients.first() {
            Some(client) => client,
            _ => return,
        };

        if client.addr == addr {
            push_event(
                0,
                Component::NetInputAcceleration {
                    timestamp: timestamp - client.ping_timestamps,
                    acceleration: packet.input,
                },
            );

            // todo: send immediate input update to all clients
        }
    }
}

fn send_to_clients<P>(sender: &Sender<LaminarPacket>, clients: &[Client], data: P)
where
    P: Copy + TimestampOffset + Into<Packet>,
{
    for client in clients {
        let mut data = data;
        data.add_client_offset(client.timestamp_offset);

        sender
            .send(LaminarPacket::reliable_sequenced(
                client.addr,
                data.into().into(),
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
