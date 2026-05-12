use pumpkin_data::{
    Block, BlockState, block_properties::blocks_movement, tag::Block::MINECRAFT_LEAVES,
};

#[derive(Debug, Clone, Copy)]
pub enum HeightmapType {
    WorldSurface = 0,
    MotionBlocking = 1,
    MotionBlockingNoLeaves = 2,
}
impl TryFrom<usize> for HeightmapType {
    type Error = &'static str;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::WorldSurface),
            1 => Ok(Self::MotionBlocking),
            2 => Ok(Self::MotionBlockingNoLeaves),
            _ => Err("Invalid usize value for HeightmapType. The value should be 0~2."),
        }
    }
}

impl HeightmapType {
    #[must_use]
    pub fn is_opaque(&self, block_state: &BlockState) -> bool {
        let block = Block::get_raw_id_from_state_id(block_state.id);
        match self {
            Self::WorldSurface => !block_state.is_air(),
            Self::MotionBlocking => blocks_movement(block_state, block) || block_state.is_liquid(),
            Self::MotionBlockingNoLeaves => {
                (blocks_movement(block_state, block) || block_state.is_liquid())
                    && !MINECRAFT_LEAVES.1.contains(&block)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use pumpkin_data::Block;

    use crate::HeightmapType;

    #[test]
    fn test_heightmap_is_opaque() {
        let air = Block::AIR.default_state;
        let stone = Block::STONE.default_state;
        let leaves = Block::OAK_LEAVES.default_state;
        let water = Block::WATER.default_state;

        // WORLD_SURFACE: Everything except air
        assert!(!HeightmapType::WorldSurface.is_opaque(air));
        assert!(HeightmapType::WorldSurface.is_opaque(stone));
        assert!(HeightmapType::WorldSurface.is_opaque(leaves));
        assert!(HeightmapType::WorldSurface.is_opaque(water));

        // MOTION_BLOCKING: Blocks movement OR is liquid
        assert!(!HeightmapType::MotionBlocking.is_opaque(air));
        assert!(HeightmapType::MotionBlocking.is_opaque(stone));
        assert!(HeightmapType::MotionBlocking.is_opaque(leaves)); // Leaves block movement
        assert!(HeightmapType::MotionBlocking.is_opaque(water)); // Water is liquid

        // MOTION_BLOCKING_NO_LEAVES: Blocks movement OR is liquid, but NOT leaves
        assert!(!HeightmapType::MotionBlockingNoLeaves.is_opaque(air));
        assert!(HeightmapType::MotionBlockingNoLeaves.is_opaque(stone));
        assert!(!HeightmapType::MotionBlockingNoLeaves.is_opaque(leaves)); // Excludes leaves
        assert!(HeightmapType::MotionBlockingNoLeaves.is_opaque(water)); // Water is liquid
    }
}
