use std::io::{Read, Write};

use crate::{
    ClientPacket,
    ser::{NetworkReadExt, NetworkWriteExt, WritingError},
};
use pumpkin_data::packet::serverbound::LOGIN_HELLO;
use pumpkin_macros::packet;

use crate::{ServerPacket, ser::ReadingError};

#[packet(LOGIN_HELLO)]
pub struct SLoginStart {
    pub name: String, // 16
    pub uuid: uuid::Uuid,
}

impl ServerPacket for SLoginStart {
    fn read(read: impl Read) -> Result<Self, ReadingError> {
        let mut read = read;

        Ok(Self {
            name: read.get_string_bounded(16)?,
            uuid: read.get_uuid()?,
        })
    }
}

impl ClientPacket for SLoginStart {
    fn write_packet_data(&self, mut write: impl Write) -> Result<(), WritingError> {
        write.write_string_bounded(&self.name, 16)?;
        write.write_uuid(&self.uuid)
    }
}
