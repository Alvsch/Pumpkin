use std::sync::atomic::AtomicBool;

use parking_lot::Mutex;
use pumpkin_nbt::compound::NbtCompound;
use rustc_hash::FxHashMap;
use uuid::Uuid;

pub struct EntityData {
    /// Chunk X
    pub x: i32,
    /// Chunk Z
    pub z: i32,
    pub data: Mutex<FxHashMap<Uuid, NbtCompound>>,

    pub dirty: AtomicBool,
}
