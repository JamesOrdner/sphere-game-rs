use nalgebra_glm::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use system::{Timestamp, STEPS_PER_SECOND};

pub type NetworkId = u16;

pub const NETWORK_SNAPSHOTS_LEN: usize = STEPS_PER_SECOND;

pub trait TimestampOffset {
    fn add_client_offset(&mut self, offset: Timestamp);
}

#[derive(Serialize, Deserialize)]
pub enum Packet {
    EstablishConnection,
    Input(InputPacket),
    Ping(PingPacket),
    StaticMesh(StaticMeshPacket),
    Velocity(VelocityPacket),
}

impl From<&[u8]> for Packet {
    fn from(data: &[u8]) -> Self {
        bincode::deserialize(data).unwrap()
    }
}

impl Into<Vec<u8>> for Packet {
    fn into(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct InputPacket {
    pub input: Vec2,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct PingPacket {
    pub timestamp: Timestamp,
}

impl Into<Packet> for PingPacket {
    fn into(self) -> Packet {
        Packet::Ping(self)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct StaticMeshPacket {
    pub timestamp: Timestamp,
    pub network_id: NetworkId,
    pub location: Vec3,
    pub velocity: Vec3,
}

impl Into<Packet> for StaticMeshPacket {
    fn into(self) -> Packet {
        Packet::StaticMesh(self)
    }
}

impl TimestampOffset for StaticMeshPacket {
    fn add_client_offset(&mut self, offset: Timestamp) {
        self.timestamp += offset;
    }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct VelocityPacket {
    pub timestamp: Timestamp,
    pub network_id: NetworkId,
    pub velocity: Vec3,
}

impl TimestampOffset for VelocityPacket {
    fn add_client_offset(&mut self, offset: Timestamp) {
        self.timestamp += offset;
    }
}

impl Into<Packet> for VelocityPacket {
    fn into(self) -> Packet {
        Packet::Velocity(self)
    }
}
