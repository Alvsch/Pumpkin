use pumpkin_data::packet::clientbound::LOGIN_HELLO;
use pumpkin_macros::packet;
use serde::Serialize;

use crate::{ser::NetworkReadExt, ServerPacket};

#[derive(Serialize)]
#[packet(LOGIN_HELLO)]
pub struct CEncryptionRequest<'a> {
    pub server_id: &'a str, // 20
    pub public_key: &'a [u8],
    pub verify_token: &'a [u8],
    pub should_authenticate: bool,
}

impl<'a> CEncryptionRequest<'a> {
    pub fn new(
        server_id: &'a str,
        public_key: &'a [u8],
        verify_token: &'a [u8],
        should_authenticate: bool,
    ) -> Self {
        Self {
            server_id,
            public_key,
            verify_token,
            should_authenticate,
        }
    }
}

#[packet(LOGIN_HELLO)]
pub struct CEncryptionRequestRead {
    pub server_id: String, // 20
    pub public_key: Box<[u8]>,
    pub verify_token: Box<[u8]>,
    pub should_authenticate: bool,
}

impl ServerPacket for CEncryptionRequestRead {
    fn read(mut read: impl std::io::Read) -> Result<Self, crate::ser::ReadingError> {
        let server_id = read.get_string_bounded(20)?;
        let len = read.get_var_int()?.0 as usize;
        let public_key = read.read_boxed_slice(len)?;
        let len = read.get_var_int()?.0 as usize;
        let verify_token = read.read_boxed_slice(len)?;

        Ok(Self {
            server_id,
            public_key,
            verify_token,
            should_authenticate: read.get_bool()?,
        })
    }
}
