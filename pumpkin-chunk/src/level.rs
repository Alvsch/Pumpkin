use crate::tick::{OrderedTick, TickPriority};
use crate::{ChunkData, ChunkEntityData};
use pumpkin_data::{Block, fluid::Fluid};
use pumpkin_util::math::position::BlockPos;
use pumpkin_world_core::BlockEntity;
use std::path::PathBuf;
use std::sync::Arc;

pub type SyncChunk = Arc<ChunkData>;
pub type SyncEntityChunk = Arc<ChunkEntityData>;

pub struct TickData {
    pub block_ticks: Vec<OrderedTick<&'static Block>>,
    pub fluid_ticks: Vec<OrderedTick<&'static Fluid>>,
    pub random_ticks: Vec<RandomTickSample>,
    pub block_entities: Vec<Arc<dyn BlockEntity>>,
}

#[derive(Clone, Copy)]
pub struct RandomTickSample {
    pub position: BlockPos,
    pub tick_block: bool,
    pub tick_fluid: bool,
}

#[derive(Clone)]
pub struct LevelFolder {
    pub root_folder: PathBuf,
    pub region_folder: PathBuf,
    pub entities_folder: PathBuf,
}

pub type LevelTickPriority = TickPriority;
