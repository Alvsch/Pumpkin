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
mod feature;
mod generator;
mod height_limit;
mod height_provider;
mod noise;
mod positions;
mod proto_chunk;
mod proto_chunk_test;
mod rule;
mod structure;
mod surface;

pub type BlockId = u16;
pub type BlockStateId = u16;

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
