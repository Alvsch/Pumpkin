use pumpkin_data::{Block, BlockState};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::math::position::BlockPos;
use rustc_hash::FxHashMap;

use crate::{BlockStateId, HeightmapType, Heightmaps, Sections};

pub struct ChunkData {
    pub heightmaps: Heightmaps,
    pub sections: Sections,
    pub block_entities: FxHashMap<BlockPos, NbtCompound>,
}

impl ChunkData {
    /// Returns the replaced block state ID
    pub fn set_block_absolute_y(
        &mut self,
        relative_x: usize,
        y: i32,
        relative_z: usize,
        block_state_id: BlockStateId,
    ) -> BlockStateId {
        let min_y = self.sections.min_y;
        let y_rel = y - min_y;
        if y_rel < 0 {
            return Block::AIR.default_state.id;
        }
        let relative_y = y_rel as usize;

        let old = self.sections.set_block_no_heightmap_update(
            relative_x,
            relative_y,
            relative_z,
            block_state_id,
        );
        if old != block_state_id {
            let state = BlockState::from_id(block_state_id);
            self.update_heightmap(relative_x, relative_y, relative_z, state);
        }
        old
    }

    fn update_heightmap(
        &mut self,
        relative_x: usize,
        relative_y: usize,
        relative_z: usize,
        block_state: &BlockState,
    ) {
        let min_y = self.sections.min_y;
        let x = relative_x as i32;
        let y = relative_y as i32 + min_y;
        let z = relative_z as i32;

        for &hm_type in &[
            HeightmapType::WorldSurface,
            HeightmapType::MotionBlocking,
            HeightmapType::MotionBlockingNoLeaves,
        ] {
            self.heightmaps
                .update(hm_type, x, z, y, block_state, min_y, |y_at| {
                    let id = self
                        .sections
                        .get_block_absolute_y(relative_x, y_at, relative_z)
                        .unwrap_or(0);
                    BlockState::from_id(id)
                });
        }
    }

    /// Gets the given block in the chunk
    #[inline]
    #[must_use]
    pub fn get_relative_block(
        &self,
        relative_x: usize,
        relative_y: usize,
        relative_z: usize,
    ) -> Option<BlockStateId> {
        self.sections
            .get_relative_block(relative_x, relative_y, relative_z)
    }

    /// Sets the given block in the chunk
    #[inline]
    pub fn set_relative_block(
        &mut self,
        relative_x: usize,
        relative_y: usize,
        relative_z: usize,
        block_state_id: BlockStateId,
    ) {
        let state = BlockState::from_id(block_state_id);
        self.update_heightmap(relative_x, relative_y, relative_z, state);
        self.sections
            .set_relative_block(relative_x, relative_y, relative_z, block_state_id);
    }

    /// Sets the given block in the chunk, returning the old block
    /// Contrary to `set_block` this does not update the heightmap.
    ///
    /// Only use this if you know you don't need to update the heightmap
    /// or if you manually set the heightmap in `empty_with_heightmap`
    #[inline]
    pub fn set_block_no_heightmap_update(
        &mut self,
        relative_x: usize,
        relative_y: usize,
        relative_z: usize,
        block_state_id: BlockStateId,
    ) {
        self.sections
            .set_relative_block(relative_x, relative_y, relative_z, block_state_id);
    }

    //TODO: Tracking heightmaps update.
    pub fn calculate_heightmap(&self) -> Heightmaps {
        let highest_non_empty_subchunk = self.get_highest_non_empty_subchunk();
        let mut heightmaps = Heightmaps::default();

        for x in 0..16 {
            for z in 0..16 {
                self.populate_heightmaps(&mut heightmaps, highest_non_empty_subchunk, x, z);
            }
        }

        // log::info!("WorldSurface:");
        // heightmaps.log_heightmap(ChunkHeightmapType::WorldSurface, self.sections.min_y);
        // log::info!("MotionBlocking:");
        // heightmaps.log_heightmap(ChunkHeightmapType::MotionBlocking, self.sections.min_y);
        // log::info!("min_y: {}", self.sections.min_y);
        heightmaps
    }

    #[inline]
    fn populate_heightmaps(
        &self,
        heightmaps: &mut Heightmaps,
        start_sub_chunk: usize,
        x: usize,
        z: usize,
    ) {
        let start_height = (start_sub_chunk as i32) * 16 - self.sections.min_y.abs() + 15;
        let mut has_found = [false, false, false];

        for y in (self.sections.min_y..=start_height).rev() {
            let state_id = self.sections.get_block_absolute_y(x, y, z).unwrap();
            let block_state = BlockState::from_id(state_id);

            for hm_type in [
                HeightmapType::WorldSurface,
                HeightmapType::MotionBlocking,
                HeightmapType::MotionBlockingNoLeaves,
            ] {
                let idx = hm_type as usize;
                if !has_found[idx] && hm_type.is_opaque(block_state) {
                    heightmaps.set(hm_type, x as i32, z as i32, y, self.sections.min_y);
                    has_found[idx] = true;
                }
            }

            if has_found.iter().all(|&found| found) {
                return;
            }
        }

        for (idx, is_set) in has_found.iter().enumerate() {
            if !(*is_set) {
                heightmaps.set(
                    idx.try_into().unwrap(),
                    x as i32,
                    z as i32,
                    self.sections.min_y - 1,
                    self.sections.min_y,
                );
            }
        }
    }

    #[must_use]
    pub fn get_highest_non_empty_subchunk(&self) -> usize {
        self.sections
            .block_sections
            .iter()
            .enumerate()
            .rev()
            .find(|(_, sub)| !sub.has_only_air())
            .map_or(0, |(idx, _)| idx)
    }
}
