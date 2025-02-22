use dashmap::{
    DashMap,
    mapref::one::{Ref, RefMut},
};
use pumpkin_data::chunk::ChunkStatus;
use pumpkin_protocol::codec::chunk::{ChunkData, ChunkParsingError};
use pumpkin_util::{WORLD_HEIGHT, math::vector2::Vector2};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
};
use thiserror::Error;

use crate::level::LevelFolder;

pub mod anvil;
pub mod linear;

pub const CHUNK_AREA: usize = 16 * 16;
pub const SUBCHUNK_VOLUME: usize = CHUNK_AREA * 16;
pub const SUBCHUNKS_COUNT: usize = WORLD_HEIGHT / 16;
pub const CHUNK_VOLUME: usize = CHUNK_AREA * WORLD_HEIGHT;

/// File locks manager to prevent multiple threads from writing to the same file at the same time
/// but allowing multiple threads to read from the same file at the same time.
static FILE_LOCK_MANAGER: LazyLock<Arc<FileLocksManager>> = LazyLock::new(Arc::default);

pub trait ChunkReader: Sync + Send {
    fn read_chunk(
        &self,
        save_file: &LevelFolder,
        at: &Vector2<i32>,
    ) -> Result<ChunkData, ChunkReadingError>;
}

pub trait ChunkWriter: Send + Sync {
    fn write_chunk(
        &self,
        chunk: &ChunkData,
        level_folder: &LevelFolder,
        at: &Vector2<i32>,
    ) -> Result<(), ChunkWritingError>;
}

#[derive(Error, Debug)]
pub enum ChunkReadingError {
    #[error("Io error: {0}")]
    IoError(std::io::ErrorKind),
    #[error("Invalid header")]
    InvalidHeader,
    #[error("Region is invalid")]
    RegionIsInvalid,
    #[error("Compression error {0}")]
    Compression(CompressionError),
    #[error("Tried to read chunk which does not exist")]
    ChunkNotExist,
    #[error("Failed to parse Chunk from bytes: {0}")]
    ParsingError(ChunkParsingError),
}

#[derive(Error, Debug)]
pub enum ChunkWritingError {
    #[error("Io error: {0}")]
    IoError(std::io::ErrorKind),
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

/// A guard that allows reading from a file while preventing writing to it
/// This is used to prevent writes while a read is in progress.
/// (dont suffer for "write starvation" problem)
///
/// When the guard is dropped, the file is unlocked.
pub struct FileReadGuard<'a> {
    _guard: Ref<'a, PathBuf, ()>,
}

/// A guard that allows writing to a file while preventing reading from it
/// This is used to prevent multiple threads from writing to the same file at the same time.
/// (dont suffer for "write starvation" problem)
///
/// When the guard is dropped, the file is unlocked.
pub struct FileWriteGuard<'a> {
    _guard: RefMut<'a, PathBuf, ()>,
}

/// Central File Lock Manager for chunk files
/// This is used to prevent multiple threads from writing to the same file at the same time
#[derive(Clone, Default)]
pub struct FileLocksManager {
    locks: DashMap<PathBuf, ()>,
}

impl FileLocksManager {
    pub fn get_read_guard(&self, path: &Path) -> FileReadGuard {
        if let Some(lock) = self.locks.get(path) {
            FileReadGuard { _guard: lock }
        } else {
            FileReadGuard {
                _guard: self
                    .locks
                    .entry(path.to_path_buf())
                    .or_insert(())
                    .downgrade(),
            }
        }
    }

    pub fn get_write_guard(&self, path: &Path) -> FileWriteGuard {
        FileWriteGuard {
            _guard: self.locks.entry(path.to_path_buf()).or_insert(()),
        }
    }

    pub fn remove_file_lock(path: &Path) {
        FILE_LOCK_MANAGER.locks.remove(path);
    }
}

#[derive(Error, Debug)]
pub enum ChunkSerializingError {
    #[error("Error serializing chunk: {0}")]
    ErrorSerializingChunk(pumpkin_nbt::Error),
}
