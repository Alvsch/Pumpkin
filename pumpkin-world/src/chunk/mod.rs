use pumpkin_nbt::compound::NbtCompound;
use rustc_hash::FxHashMap;
use std::sync::atomic::AtomicBool;
use thiserror::Error;
use tokio::sync::Mutex;

pub mod format;
pub mod io;

#[derive(Error, Debug)]
pub enum ChunkReadingError {
    #[error("Io error: {0}")]
    IoError(std::io::Error),
    #[error("Invalid header")]
    InvalidHeader,
    #[error("Region is invalid")]
    RegionIsInvalid,
    #[error("Compression error {0}")]
    Compression(CompressionError),
    #[error("Tried to read chunk which does not exist")]
    ChunkNotExist,
    #[error("Failed to parse chunk from bytes: {0}")]
    ParsingError(ChunkParsingError),
}

#[derive(Error, Debug)]
pub enum ChunkWritingError {
    #[error("Io error: {0}")]
    IoError(std::io::Error),
    #[error("Compression error {0}")]
    Compression(CompressionError),
    #[error("Chunk serializing error: {0}")]
    ChunkSerializingError(String),
}

#[derive(Error, Debug)]
pub enum CompressionError {
    #[error("Compression scheme not recognised")]
    UnknownCompression,
    #[error("Error while working with zlib compression: {0}")]
    ZlibError(std::io::Error),
    #[error("Error while working with Gzip compression: {0}")]
    GZipError(std::io::Error),
    #[error("Error while working with LZ4 compression: {0}")]
    LZ4Error(std::io::Error),
    #[error("Error while working with zstd compression: {0}")]
    ZstdError(std::io::Error),
}

// Clone here cause we want to clone a snapshot of the chunk so we don't block writing for too long
// pub struct ChunkData {
//     pub section: ChunkSections,
//     /// See `https://minecraft.wiki/w/Heightmap` for more info
//     pub heightmap: std::sync::Mutex<ChunkHeightmaps>,
//     pub x: i32,
//     pub z: i32,
//     pub block_ticks: ChunkTickScheduler<&'static Block>,
//     pub fluid_ticks: ChunkTickScheduler<&'static Fluid>,
//     pub pending_block_entities: std::sync::Mutex<FxHashMap<BlockPos, NbtCompound>>,
//     pub light_engine: std::sync::Mutex<ChunkLight>,
//     pub light_populated: AtomicBool,
//     pub status: ChunkStatus,
//     pub blending_data: Option<crate::generation::blender::blending_data::BlendingData>,
//     pub dirty: AtomicBool,
// }

pub struct ChunkEntityData {
    /// Chunk X
    pub x: i32,
    /// Chunk Z
    pub z: i32,
    pub data: Mutex<FxHashMap<uuid::Uuid, NbtCompound>>,

    pub dirty: AtomicBool,
}

#[derive(Error, Debug)]
pub enum ChunkParsingError {
    #[error("Failed reading chunk status {0}")]
    FailedReadStatus(pumpkin_nbt::Error),
    #[error("The chunk isn't generated yet")]
    ChunkNotGenerated,
    #[error("Error deserializing chunk: {0}")]
    ErrorDeserializingChunk(String),
}

#[derive(Error, Debug)]
pub enum ChunkSerializingError {
    #[error("Error serializing chunk: {0}")]
    ErrorSerializingChunk(pumpkin_nbt::Error),
}

#[cfg(test)]
mod tests {
    use pumpkin_chunk::{Sections, palette::BlockPalette};
    use pumpkin_data::{Block, block_properties::has_random_ticks};

    #[test]
    fn random_tick_cache_initializes_from_palette_contents() {
        let mut sections = vec![BlockPalette::default(), BlockPalette::default()];
        sections[1].set(0, 0, 0, Block::LAVA.default_state.id);

        let (cache, _mask) = Sections::build_random_tick_sections_cache(&sections);
        let cache = cache.unwrap();
        assert!(!cache[0].is_randomly_ticking());
        assert!(cache[1].random_ticking_fluid_count > 0);
        assert!(cache[1].is_randomly_ticking());
    }

    #[test]
    fn random_tick_cache_updates_on_block_mutation() {
        let min_y = -64;
        let mut sections = Sections::new(1, min_y);

        assert!(
            sections
                .random_tick_sections
                .as_ref()
                .is_none_or(|c| !c[0].is_randomly_ticking()),
            "fresh sections should not be randomly ticking"
        );

        let random_block_state = Block::WHEAT.default_state.id;
        assert!(
            has_random_ticks(random_block_state),
            "test requires a known randomly ticking block state"
        );

        sections.set_block_absolute_y(0, min_y, 0, random_block_state);
        let cache = sections.random_tick_sections.as_ref().unwrap();
        assert_eq!(cache[0].random_ticking_block_count, 1);
        assert_eq!(cache[0].random_ticking_fluid_count, 0);
        assert!(cache[0].is_randomly_ticking());

        sections.set_block_absolute_y(0, min_y, 0, Block::STONE.default_state.id);
        let cache = sections.random_tick_sections.as_ref().unwrap();
        assert_eq!(cache[0].random_ticking_block_count, 0);
        assert_eq!(cache[0].random_ticking_fluid_count, 0);
        assert!(!cache[0].is_randomly_ticking());

        sections.set_block_absolute_y(0, min_y, 0, Block::LAVA.default_state.id);
        let cache = sections.random_tick_sections.as_ref().unwrap();
        assert!(cache[0].random_ticking_fluid_count > 0);
        assert!(cache[0].is_randomly_ticking());
    }
}
