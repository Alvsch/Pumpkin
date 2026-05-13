use std::io;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use pumpkin_util::math::vector2::Vector2;

use crate::io::format::{
    ChunkFormat, ChunkParsingError, ChunkReadingError, anvil::chunk::{AnvilChunk, AnvilChunkData}
};

mod chunk;
mod compression;

pub use compression::{Compression, CompressionAlgorithm, Gzip, Lz4, NoCompression, Zlib};

/// The side size of a region in chunks (one region is 32x32 chunks)
const REGION_SIZE: usize = 32;
/// The number of chunks in a region
const CHUNK_COUNT: usize = REGION_SIZE * REGION_SIZE;

/// The number of bits that identify two chunks in the same region
const SUBREGION_BITS: u8 = pumpkin_util::math::ceil_log2(REGION_SIZE as u32);
const SUBREGION_AND: i32 = 2i32.pow(SUBREGION_BITS as u32) - 1;

/// The number of bytes in a sector (4 KiB)
const SECTOR_BYTES: usize = 4096;

// 26.1.2
const WORLD_DATA_VERSION: i32 = 4790;

pub struct AnvilFormat {
    chunks: [Option<AnvilChunk>; CHUNK_COUNT],
    // end_sector: u32,
}

impl AnvilFormat {
    #[must_use]
    const fn get_region_coordinates(at: &Vector2<i32>) -> (i32, i32) {
        // Divide by 32 for the region coordinates
        (at.x >> SUBREGION_BITS, at.y >> SUBREGION_BITS)
    }

    #[must_use]
    const fn get_chunk_index(x: i32, z: i32) -> usize {
        let local_x = x & SUBREGION_AND;
        let local_z = z & SUBREGION_AND;
        let index = (local_z << SUBREGION_BITS) + local_x;
        index as usize
    }
}

impl ChunkFormat for AnvilFormat {
    fn get_chunk_key(chunk: &Vector2<i32>) -> String {
        let (region_x, region_z) = Self::get_region_coordinates(chunk);
        format!("./r.{region_x}.{region_z}.mca")
    }

    fn serialize<B: BufMut>(&self, buf: &mut B) -> Result<(), io::Error> {
        // Build the 8 KiB header in memory
        let mut header = BytesMut::with_capacity(SECTOR_BYTES * 2);
        let mut current_sector: u32 = 2;

        // Location Table
        for metadata in &self.chunks {
            if let Some(chunk) = metadata {
                let sector_count = chunk.data.sector_count();
                header.put_u32((current_sector << 8) | sector_count);
                current_sector += sector_count;
            } else {
                header.put_u32(0);
            }
        }

        // Timestamp Table
        for metadata in &self.chunks {
            if let Some(chunk) = metadata {
                header.put_u32(chunk.timestamp);
            } else {
                header.put_u32(0);
            }
        }

        // Write all 8 KiB in a single async call
        buf.put(header);

        // Write chunk data
        for chunk in self.chunks.iter().flatten() {
            buf.put_slice(&chunk.data.compressed_data);
        }
        Ok(())
    }

    fn deserialize(mut data: Bytes) -> Result<Self, ChunkReadingError> {
        if data.len() < SECTOR_BYTES * 2 {
            return Err(ChunkReadingError::InvalidHeader);
        }

        let headers = data.split_to(SECTOR_BYTES * 2);
        let (mut location_bytes, mut timestamp_bytes) = headers.split_at(SECTOR_BYTES);

        let mut chunks = [const { None }; CHUNK_COUNT];

        let mut last_offset = 2;
        for i in 0..CHUNK_COUNT {
            let timestamp = timestamp_bytes.get_u32();
            let location = location_bytes.get_u32();

            let sector_count = (location & 0xFF) as usize;
            let sector_offset = (location >> 8) as usize;
            let end_offset = sector_offset + sector_count;

            // If the sector offset or count is 0, the chunk is not present (we should not parse empty chunks)
            if sector_offset == 0 || sector_count == 0 {
                continue;
            }

            if end_offset > last_offset {
                last_offset = end_offset;
            }

            // We always subtract 2 for the first two sectors for the timestamp and location tables
            // that we walked earlier
            let bytes_offset = (sector_offset - 2) * SECTOR_BYTES;
            let bytes_count = sector_count * SECTOR_BYTES;

            if bytes_offset + bytes_count > data.len() {
                return Err(ChunkReadingError::ParsingError(
                    ChunkParsingError::ErrorDeserializingChunk(format!(
                        "Not enough bytes available for the chunk {} ({} vs {})",
                        i,
                        bytes_count,
                        data.len().saturating_sub(bytes_offset)
                    )),
                ));
            }

            let serialized_data = AnvilChunkData::deserialize(
                data.slice(bytes_offset..bytes_offset + bytes_count),
            )?;

            chunks[i] = Some(AnvilChunk {
                data: serialized_data,
                timestamp,
                file_sector_offset: sector_offset as u32,
            });
        }

        Ok(AnvilFormat {
            chunks,
            // end_sector: last_offset as u32,
        })
    }
}
