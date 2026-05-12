use pumpkin_data::fluid::Fluid;

mod chunk_data;
mod heightmaps;
mod light_data;
pub mod palette;
mod sections;

use crate::palette::{BiomePalette, BlockPalette};
pub use crate::{
    chunk_data::ChunkData,
    heightmaps::{HeightmapType, Heightmaps},
    light_data::{LightContainer, LightData},
    sections::Sections,
};

// TODO
pub const CHUNK_WIDTH: usize = BlockPalette::SIZE;
pub const CHUNK_AREA: usize = CHUNK_WIDTH * CHUNK_WIDTH;
pub const BIOME_VOLUME: usize = BiomePalette::VOLUME;
pub const SUBCHUNK_VOLUME: usize = CHUNK_AREA * CHUNK_WIDTH;

#[must_use]
pub fn has_random_ticking_fluid(state_id: u16) -> bool {
    Fluid::from_state_id(state_id)
        .is_some_and(|fluid| Fluid::same_fluid_type(fluid.id, Fluid::LAVA.id))
}

pub type BlockId = u16;
pub type BlockStateId = u16;

pub struct Chunk {
    pub chunk_data: ChunkData,
    pub light_data: LightData,
}
