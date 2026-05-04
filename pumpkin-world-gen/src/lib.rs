use generator::{GeneratorInit, VanillaGenerator};
use pumpkin_data::dimension::Dimension;
use pumpkin_util::{
    random::xoroshiro128::{Xoroshiro, XoroshiroSplitter},
    world_seed::Seed,
};

mod biome;
mod blender;
mod block_predicate;
mod block_state_provider;
mod carver;
pub mod chunk_system;
mod feature;
pub mod generator;
mod height_limit;
mod height_provider;
pub mod lighting;
pub mod noise;
pub mod positions;
pub mod proto_chunk;
mod proto_chunk_test;
mod rule;
mod structure;
mod surface;

pub type BlockId = u16;
pub type BlockStateId = u16;

pub mod chunk {
    pub use pumpkin_chunk::{
        CHUNK_AREA, CHUNK_WIDTH, BIOME_VOLUME, ChunkData, ChunkEntityData, ChunkHeightmapType,
        ChunkHeightmaps, ChunkLight, ChunkSections, RandomTickSectionCache, SUBCHUNK_VOLUME,
    };

    pub mod format {
        pub use pumpkin_chunk::format::LightContainer;
    }

    pub mod io {
        pub use pumpkin_chunk::io::{Dirtiable, FileIO, LoadedData};
    }
}

pub mod level {
    pub use pumpkin_chunk::level::{
        LevelFolder, LevelTickPriority, RandomTickSample, SyncChunk, SyncEntityChunk, TickData,
    };
}

pub mod world {
    pub use pumpkin_world_core::{BlockAccessor, BlockRegistryExt};
}

pub mod generation {
    pub mod feature {
        pub mod configured_features {
            pub use crate::feature::configured_features::*;
        }

        pub mod features {
            pub use crate::feature::features::*;
        }

        pub mod placed_features {
            pub use crate::feature::placed_features::*;
        }

        pub mod size {
            pub use crate::feature::size::*;
        }
    }

    pub use pumpkin_world_core::biome_coords;
    pub use pumpkin_world_core::section_coords;
    pub use crate::generator;
    pub use crate::get_world_gen;
    pub use crate::noise;
    pub use crate::positions;
    pub use crate::proto_chunk;
    pub use crate::GlobalRandomConfig;
}

pub use pumpkin_world_core::section_coords;

#[must_use]
pub fn get_world_gen(seed: Seed, dimension: Dimension) -> Box<VanillaGenerator> {
    // TODO decide which WorldGenerator to pick based on config.
    Box::new(VanillaGenerator::new(seed, dimension))
}

pub struct GlobalRandomConfig {
    pub seed: u64,
    pub legacy_random_source: bool,
    base_random_deriver: XoroshiroSplitter,
    aquifer_random_deriver: XoroshiroSplitter,
    pub ore_random_deriver: XoroshiroSplitter,
}

impl GlobalRandomConfig {
    #[must_use]
    pub fn new(seed: u64, legacy_random_source: bool) -> Self {
        let random_deriver = Xoroshiro::from_seed(seed).next_splitter();

        let aquifer_deriver = random_deriver
            .split_string("minecraft:aquifer")
            .next_splitter();
        let ore_deriver = random_deriver.split_string("minecraft:ore").next_splitter();
        Self {
            seed,
            legacy_random_source,
            base_random_deriver: random_deriver,
            aquifer_random_deriver: aquifer_deriver,
            ore_random_deriver: ore_deriver,
        }
    }

    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.seed
    }
}

#[derive(PartialEq, Eq)]
pub enum Direction {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}
