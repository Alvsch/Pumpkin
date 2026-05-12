use std::sync::atomic::AtomicU32;

use pumpkin_data::{
    Block,
    block_properties::{has_random_ticks, is_air},
};

use crate::{
    BlockStateId, has_random_ticking_fluid,
    palette::{BiomePalette, BlockPalette},
    sections::cache::RandomTickSectionCache,
};

mod cache;

/// Represents pure block data for a chunk.
/// Subchunks are vertical portions of a chunk. They are 16 blocks tall.
/// There are currently 24 subchunks per chunk.
///
/// A chunk can be:
/// - Subchunks: 24 separate subchunks are stored.
pub struct Sections {
    pub count: usize,
    pub block_sections: Box<[BlockPalette]>,
    pub random_tick_sections: Option<Box<[RandomTickSectionCache]>>,
    pub randomly_ticking_mask: AtomicU32,
    pub biome_sections: Box<[BiomePalette]>,
    pub min_y: i32,
}

impl Sections {
    #[must_use]
    pub fn build_random_tick_sections_cache(
        block_sections: &[BlockPalette],
    ) -> (Option<Box<[RandomTickSectionCache]>>, u32) {
        let mut mask = 0;
        let mut has_ticks = false;
        let cache = block_sections
            .iter()
            .enumerate()
            .map(|(i, section)| {
                let (random_ticking_block_count, random_ticking_fluid_count) =
                    section.random_ticking_counts();
                if random_ticking_block_count > 0 || random_ticking_fluid_count > 0 {
                    mask |= 1 << i;
                    has_ticks = true;
                }
                RandomTickSectionCache {
                    random_ticking_block_count,
                    random_ticking_fluid_count,
                }
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();

        if has_ticks {
            (Some(cache), mask)
        } else {
            (None, 0)
        }
    }

    #[must_use]
    pub fn new(num_sections: usize, min_y: i32) -> Self {
        let block_sections = vec![BlockPalette::default(); num_sections].into_boxed_slice();
        let (random_tick_sections, randomly_ticking_mask) =
            Self::build_random_tick_sections_cache(&block_sections);
        let biome_sections = vec![BiomePalette::default(); num_sections].into_boxed_slice();

        Self {
            count: num_sections,
            block_sections,
            random_tick_sections,
            randomly_ticking_mask: AtomicU32::new(randomly_ticking_mask),
            biome_sections,
            min_y,
        }
    }

    #[must_use]
    pub fn get_block_absolute_y(
        &self,
        relative_x: usize,
        y: i32,
        relative_z: usize,
    ) -> Option<BlockStateId> {
        let y = y - self.min_y;
        if y < 0 {
            None
        } else {
            let relative_y = y as usize;
            self.get_relative_block(relative_x, relative_y, relative_z)
        }
    }

    pub fn set_block_absolute_y(
        &mut self,
        relative_x: usize,
        y: i32,
        relative_z: usize,
        block_state_id: BlockStateId,
    ) -> BlockStateId {
        let y = y - self.min_y;
        if y < 0 {
            return Block::AIR.default_state.id;
        }
        let relative_y = y as usize;
        self.set_block_no_heightmap_update(relative_x, relative_y, relative_z, block_state_id)
    }

    #[must_use]
    pub fn get_rough_biome_absolute_y(
        &self,
        relative_x: usize,
        y: i32,
        relative_z: usize,
    ) -> Option<u8> {
        let y = y - self.min_y;
        if y < 0 {
            None
        } else {
            let relative_y = y as usize;
            self.get_noise_biome(
                relative_y / BlockPalette::SIZE,
                relative_x >> 2 & 3,
                relative_y >> 2 & 3,
                relative_z >> 2 & 3,
            )
        }
    }

    /// Gets the given block in the chunk
    pub fn get_relative_block(
        &self,
        relative_x: usize,
        relative_y: usize,
        relative_z: usize,
    ) -> Option<BlockStateId> {
        debug_assert!(relative_x < BlockPalette::SIZE);
        debug_assert!(relative_z < BlockPalette::SIZE);

        let section_index = relative_y / BlockPalette::SIZE;
        let relative_y = relative_y % BlockPalette::SIZE;
        self.block_sections
            .get(section_index)
            .map(|section| section.get(relative_x, relative_y, relative_z))
    }

    /// Sets the given block in the chunk, returning the old block state ID
    #[inline]
    pub fn set_relative_block(
        &mut self,
        relative_x: usize,
        relative_y: usize,
        relative_z: usize,
        block_state_id: BlockStateId,
    ) -> BlockStateId {
        self.set_block_no_heightmap_update(relative_x, relative_y, relative_z, block_state_id)
    }

    /// Sets the given block in the chunk, returning the old block
    /// Contrary to `set_block` this does not update the heightmap.
    ///
    /// Only use this if you know you don't need to update the heightmap
    /// or if you manually set the heightmap in `empty_with_heightmap`
    pub fn set_block_no_heightmap_update(
        &mut self,
        relative_x: usize,
        relative_y: usize,
        relative_z: usize,
        block_state_id: BlockStateId,
    ) -> BlockStateId {
        debug_assert!(relative_x < BlockPalette::SIZE);
        debug_assert!(relative_z < BlockPalette::SIZE);

        let section_index = relative_y / BlockPalette::SIZE;
        let relative_y = relative_y % BlockPalette::SIZE;

        // Keep lock order consistent to avoid deadlocks: block sections first, then random-tick cache.
        if let Some(section) = self.block_sections.get_mut(section_index) {
            let replaced_block_state_id =
                section.set(relative_x, relative_y, relative_z, block_state_id);
            if replaced_block_state_id == block_state_id {
                return replaced_block_state_id;
            }

            if (has_random_ticks(block_state_id) || has_random_ticking_fluid(block_state_id))
                && self.random_tick_sections.is_none()
            {
                let new_cache =
                    vec![RandomTickSectionCache::default(); self.count].into_boxed_slice();
                self.random_tick_sections = Some(new_cache);
            }

            if let Some(random_tick_sections) = self.random_tick_sections.as_mut() {
                let random_tick_cache = &mut random_tick_sections[section_index];
                if has_random_ticks(replaced_block_state_id) {
                    random_tick_cache.random_ticking_block_count = random_tick_cache
                        .random_ticking_block_count
                        .saturating_sub(1);
                }
                if has_random_ticking_fluid(replaced_block_state_id) {
                    random_tick_cache.random_ticking_fluid_count = random_tick_cache
                        .random_ticking_fluid_count
                        .saturating_sub(1);
                }

                if has_random_ticks(block_state_id) {
                    random_tick_cache.random_ticking_block_count = random_tick_cache
                        .random_ticking_block_count
                        .saturating_add(1);
                }
                if has_random_ticking_fluid(block_state_id) {
                    random_tick_cache.random_ticking_fluid_count = random_tick_cache
                        .random_ticking_fluid_count
                        .saturating_add(1);
                }

                // Update the bitmask
                let mut mask = self
                    .randomly_ticking_mask
                    .load(std::sync::atomic::Ordering::Relaxed);
                if random_tick_cache.is_randomly_ticking() {
                    mask |= 1 << section_index;
                } else {
                    mask &= !(1 << section_index);
                }
                self.randomly_ticking_mask
                    .store(mask, std::sync::atomic::Ordering::Relaxed);

                // If no more ticking sections, we could potentially deallocate, but that might be overkill and cause jitter.
            }

            return replaced_block_state_id;
        }
        0
    }

    pub fn set_relative_biome(
        &mut self,
        relative_x: usize,
        relative_y: usize,
        relative_z: usize,
        biome_id: u8,
    ) {
        debug_assert!(relative_x < BiomePalette::SIZE);
        debug_assert!(relative_z < BiomePalette::SIZE);

        let section_index = relative_y / BiomePalette::SIZE;
        let relative_y = relative_y % BiomePalette::SIZE;
        if let Some(section) = self.biome_sections.get_mut(section_index) {
            section.set(relative_x, relative_y, relative_z, biome_id);
        }
    }

    #[must_use]
    pub fn get_noise_biome(
        &self,
        index: usize,
        scale_x: usize,
        scale_y: usize,
        scale_z: usize,
    ) -> Option<u8> {
        debug_assert!(scale_x < BiomePalette::SIZE);
        debug_assert!(scale_z < BiomePalette::SIZE);
        self.biome_sections
            .get(index)
            .map(|section| section.get(scale_x, scale_y, scale_z))
    }

    #[must_use]
    pub fn get_top_y(&self, relative_x: usize, relative_z: usize, first_y: i32) -> Option<i32> {
        debug_assert!(relative_x < BlockPalette::SIZE);
        debug_assert!(relative_z < BlockPalette::SIZE);

        let mut y = first_y;
        while y >= self.min_y {
            if let Some(block_state_id) = self.get_block_absolute_y(relative_x, y, relative_z)
                && !is_air(block_state_id)
            {
                return Some(y);
            }
            y -= 1;
        }
        None
    }
}

#[cfg(test)]
impl Sections {
    #[must_use]
    pub fn dump_blocks(&self) -> Vec<u16> {
        self.block_sections
            .iter()
            .flat_map(|section| section.iter().copied())
            .collect()
    }

    #[must_use]
    pub fn dump_biomes(&self) -> Vec<u8> {
        self.biome_sections
            .iter()
            .flat_map(|section| section.iter().copied())
            .collect()
    }
}
