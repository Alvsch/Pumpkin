use std::io::Write;

use pumpkin_data::packet::clientbound::PLAY_PLAYER_POSITION;
use pumpkin_macros::packet;
use pumpkin_util::math::vector3::Vector3;

use crate::{
    ClientPacket, PositionFlag, ServerPacket, VarInt,
    ser::{NetworkReadExt, NetworkWriteExt, WritingError},
};

#[packet(PLAY_PLAYER_POSITION)]
pub struct CPlayerPosition {
    pub teleport_id: VarInt,
    pub position: Vector3<f64>,
    pub delta: Vector3<f64>,
    pub yaw: f32,
    pub pitch: f32,
    pub releatives: Vec<PositionFlag>,
}

impl CPlayerPosition {
    pub fn new(
        teleport_id: VarInt,
        position: Vector3<f64>,
        delta: Vector3<f64>,
        yaw: f32,
        pitch: f32,
        releatives: &[PositionFlag],
    ) -> Self {
        Self {
            teleport_id,
            position,
            delta,
            yaw,
            pitch,
            releatives: releatives.to_vec(),
        }
    }
}

// TODO: Do we need a custom impl?
impl ClientPacket for CPlayerPosition {
    fn write_packet_data(&self, write: impl Write) -> Result<(), WritingError> {
        let mut write = write;

        write.write_var_int(&self.teleport_id)?;
        write.write_f64_be(self.position.x)?;
        write.write_f64_be(self.position.y)?;
        write.write_f64_be(self.position.z)?;
        write.write_f64_be(self.delta.x)?;
        write.write_f64_be(self.delta.y)?;
        write.write_f64_be(self.delta.z)?;
        write.write_f32_be(self.yaw)?;
        write.write_f32_be(self.pitch)?;
        // not sure about that
        write.write_i32_be(PositionFlag::get_bitfield(&self.releatives))
    }
}

impl ServerPacket for CPlayerPosition {
    fn read(mut read: impl std::io::Read) -> Result<Self, crate::ser::ReadingError> {
        Ok(Self {
            teleport_id: read.get_var_int()?,
            position: Vector3::new(read.get_f64_be()?, read.get_f64_be()?, read.get_f64_be()?),
            delta: Vector3::new(read.get_f64_be()?, read.get_f64_be()?, read.get_f64_be()?),
            yaw: read.get_f32_be()?,
            pitch: read.get_f32_be()?,
            releatives: {
                // TODO impl from bitfield
                read.get_i32_be()?;
                Vec::new()
            },
        })
    }
}
