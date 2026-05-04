use pumpkin_util::{
    math::position::BlockPos,
    random::{RandomGenerator, RandomImpl},
};
use pumpkin_world_core::BlockRegistryExt;

use crate::feature::placed_features::PlacedFeature;
use crate::proto_chunk::GenerationCache;

pub struct SimpleRandomFeature {
    pub features: Vec<PlacedFeature>,
}

impl SimpleRandomFeature {
    #[expect(clippy::too_many_arguments)]
    pub fn generate<T: GenerationCache>(
        &self,
        chunk: &mut T,
        block_registry: &dyn BlockRegistryExt,
        min_y: i8,
        height: u16,
        feature_name: &str, // This placed feature
        random: &mut RandomGenerator,
        pos: BlockPos,
    ) -> bool {
        let i = random.next_bounded_i32(self.features.len() as i32);
        let feature = &self.features[i as usize];
        feature.generate(
            chunk,
            block_registry,
            min_y,
            height,
            feature_name,
            random,
            pos,
        )
    }
}
