use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::io::format::{ChunkParsingError, ChunkReadingError, CompressionError, anvil::{SECTOR_BYTES, compression::Compression}};

pub struct AnvilChunk {
    pub data: AnvilChunkData,
    pub timestamp: u32,
    // NOTE: This is only valid if our WriteAction is `Parts`
    pub file_sector_offset: u32,
}

pub struct AnvilChunkData {
    pub compression: Compression,
    pub compressed_data: Bytes,
}

impl AnvilChunkData {
    /// Raw size of serialized chunk
    pub const fn raw_write_size(&self) -> usize {
        // 4 bytes for the *length* and 1 byte for the *compression* method
        self.compressed_data.len() + 4 + 1
    }

    /// Size of serialized chunk with padding
    pub const fn padded_size(&self) -> usize {
        let sector_count = self.sector_count() as usize;
        sector_count * SECTOR_BYTES
    }

    pub const fn sector_count(&self) -> u32 {
        let total_size = self.raw_write_size();
        total_size.div_ceil(SECTOR_BYTES) as u32
    }

    pub fn serialize(&self) -> Bytes {
        let mut buf = BytesMut::new();
        buf.put_u32((self.compressed_data.remaining() + 1) as u32);
        buf.put_u8(self.compression.as_u8());
        buf.put_slice(&self.compressed_data);

        let padding_len = self.padded_size() - self.raw_write_size();
        buf.put_bytes(0, padding_len);

        buf.freeze()
    }

    pub fn deserialize(mut data: Bytes) -> Result<Self, ChunkReadingError> {
        // Minus one for the compression byte
        let length = data.get_u32() as usize - 1;

        if length > data.len() {
            return Err(ChunkReadingError::ParsingError(
                ChunkParsingError::ErrorDeserializingChunk(format!(
                    "Chunk length is greater than available bytes ({} vs {})",
                    length,
                    data.len()
                )),
            ));
        }

        let compression_method = data.get_u8();
        let compression = Compression::from_u8(compression_method)
            .map_err(|()| ChunkReadingError::Compression(CompressionError::UnknownCompression))?;

        Ok(Self {
            compression,
            // If this has padding, we need to trim it
            compressed_data: data.slice(..length),
        })
    }
}
