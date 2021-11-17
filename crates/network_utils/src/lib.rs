use nalgebra_glm::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use system::{Timestamp, STEPS_PER_SECOND};

pub type NetworkId = u16;

pub const NETWORK_SNAPSHOTS_LEN: usize = STEPS_PER_SECOND;

#[derive(Serialize, Deserialize)]
pub enum PacketType {
    Input,
    ServerConnect,
    StaticMesh,
    Velocity,
}

impl From<&[u8]> for PacketType {
    fn from(data: &[u8]) -> Self {
        bincode::deserialize(data).unwrap()
    }
}

#[derive(Serialize, Deserialize)]
pub struct InputPacket {
    packet_type: PacketType,
    pub timestamp: Timestamp,
    pub input: Vec2,
}

impl InputPacket {
    pub fn new(timestamp: Timestamp, input: Vec2) -> Self {
        Self {
            packet_type: PacketType::Input,
            timestamp,
            input,
        }
    }
}

impl From<&[u8]> for InputPacket {
    fn from(data: &[u8]) -> Self {
        bincode::deserialize(data).unwrap()
    }
}

impl Into<Vec<u8>> for InputPacket {
    fn into(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
}

#[derive(Serialize, Deserialize)]
pub struct ServerConnectPacket {
    packet_type: PacketType,
    pub timestamp: Timestamp,
}

impl ServerConnectPacket {
    pub fn new(timestamp: Timestamp) -> Self {
        Self {
            packet_type: PacketType::ServerConnect,
            timestamp,
        }
    }
}

impl From<&[u8]> for ServerConnectPacket {
    fn from(data: &[u8]) -> Self {
        bincode::deserialize(data).unwrap()
    }
}

impl Into<Vec<u8>> for ServerConnectPacket {
    fn into(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
}

#[derive(Serialize, Deserialize)]
pub struct StaticMeshPacket {
    packet_type: PacketType,
    pub timestamp: Timestamp,
    pub network_id: NetworkId,
    pub location: Vec3,
    pub velocity: Vec3,
}

impl StaticMeshPacket {
    pub fn new(
        timestamp: Timestamp,
        network_id: NetworkId,
        location: Vec3,
        velocity: Vec3,
    ) -> Self {
        Self {
            packet_type: PacketType::StaticMesh,
            timestamp,
            network_id,
            location,
            velocity,
        }
    }
}

impl From<&[u8]> for StaticMeshPacket {
    fn from(data: &[u8]) -> Self {
        bincode::deserialize(data).unwrap()
    }
}

impl Into<Vec<u8>> for StaticMeshPacket {
    fn into(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
}

#[derive(Serialize, Deserialize)]
pub struct VelocityPacket {
    packet_type: PacketType,
    pub timestamp: Timestamp,
    pub network_id: NetworkId,
    pub velocity: Vec3,
}

impl VelocityPacket {
    pub fn new(timestamp: Timestamp, network_id: NetworkId, velocity: Vec3) -> Self {
        Self {
            packet_type: PacketType::Velocity,
            timestamp,
            network_id,
            velocity,
        }
    }
}

impl From<&[u8]> for VelocityPacket {
    fn from(data: &[u8]) -> Self {
        bincode::deserialize(data).unwrap()
    }
}

impl Into<Vec<u8>> for VelocityPacket {
    fn into(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
}
