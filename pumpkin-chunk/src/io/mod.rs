use std::{collections::HashMap, path::PathBuf};

use crate::io::format::ChunkFormat;

pub mod format;

pub trait Dirtiable {
    fn is_dirty(&self) -> bool;
    fn mark_dirty(&self, flag: bool);
}

/// Manages loaded chunks in memory and transparently persists them to disk.
/// Chunks are loaded on demand, reference-counted while in use, and flushed
/// back to storage when no longer needed.
pub struct ChunkCache<F: ChunkFormat> {
}
