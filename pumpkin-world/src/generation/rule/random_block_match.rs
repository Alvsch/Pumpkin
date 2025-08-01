use pumpkin_data::{Block, BlockState};
use pumpkin_util::random::{RandomGenerator, RandomImpl};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct RandomBlockMatchRuleTest {
    // This should be a Block codec, so this is wrong
    block: String,
    probability: f32,
}

impl RandomBlockMatchRuleTest {
    pub fn test(&self, state: &BlockState, random: &mut RandomGenerator) -> bool {
        Block::from_state_id(state.id).name
            == self.block.strip_prefix("minecraft:").unwrap_or(&self.block)
            && random.next_f32() < self.probability
    }
}
