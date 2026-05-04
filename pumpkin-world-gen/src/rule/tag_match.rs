use pumpkin_data::tag;
use pumpkin_world_core::RawBlockState;

pub struct TagMatchRuleTest {
    pub tag: tag::Tag,
}

impl TagMatchRuleTest {
    #[must_use]
    pub fn test(&self, state: RawBlockState) -> bool {
        self.tag.1.contains(&state.to_block_id())
    }
}
