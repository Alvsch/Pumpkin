use pumpkin_data::BlockState;
use pumpkin_nbt::nbt_long_array;
use serde::{Deserialize, Serialize};

mod heightmap_type;

pub use crate::heightmaps::heightmap_type::HeightmapType;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub struct Heightmaps {
    #[serde(
        serialize_with = "nbt_long_array",
        skip_serializing_if = "Option::is_none"
    )]
    pub world_surface: Option<Box<[i64]>>,
    #[serde(
        serialize_with = "nbt_long_array",
        skip_serializing_if = "Option::is_none"
    )]
    pub motion_blocking: Option<Box<[i64]>>,
    #[serde(
        serialize_with = "nbt_long_array",
        skip_serializing_if = "Option::is_none"
    )]
    pub motion_blocking_no_leaves: Option<Box<[i64]>>,
}

impl Heightmaps {
    pub fn set(&mut self, heightmap: HeightmapType, x: i32, z: i32, height: i32, min_y: i32) {
        let data = match heightmap {
            HeightmapType::WorldSurface => &mut self.world_surface,
            HeightmapType::MotionBlocking => &mut self.motion_blocking,
            HeightmapType::MotionBlockingNoLeaves => &mut self.motion_blocking_no_leaves,
        };

        let data = data.get_or_insert_with(|| vec![0; 37].into_boxed_slice());

        let local_x = (x & 15) as usize;
        let local_z = (z & 15) as usize;
        let column_idx = local_z * 16 + local_x;

        // In Minecraft 1.16+, height is stored as (y - min_y + 1). 0 means below min_y.
        // It uses 9 bits per value, packed such that they do not cross u64 boundaries.
        // 64 / 9 = 7 values per u64.
        let val = (height - min_y + 1).max(0) as u64;

        let array_idx = column_idx / 7;
        let shift = (column_idx % 7) * 9;

        let mask = 0x1FFu64 << shift;

        let mut current = data[array_idx] as u64;
        current = (current & !mask) | ((val & 0x1FF) << shift);
        data[array_idx] = current as i64;
    }

    #[must_use]
    pub fn get(&self, heightmap: HeightmapType, x: i32, z: i32, min_y: i32) -> i32 {
        let data = match heightmap {
            HeightmapType::WorldSurface => &self.world_surface,
            HeightmapType::MotionBlocking => &self.motion_blocking,
            HeightmapType::MotionBlockingNoLeaves => &self.motion_blocking_no_leaves,
        };

        let Some(data) = data else {
            return min_y - 1;
        };

        let local_x = (x & 15) as usize;
        let local_z = (z & 15) as usize;
        let column_idx = local_z * 16 + local_x;

        let array_idx = column_idx / 7;
        let shift = (column_idx % 7) * 9;

        let current = data[array_idx] as u64;
        let val = (current >> shift) & 0x1FF;

        (val as i32) + min_y - 1
    }

    #[expect(clippy::too_many_arguments)]
    pub fn update<F>(
        &mut self,
        heightmap_type: HeightmapType,
        local_x: i32,
        local_y: i32,
        local_z: i32,
        block_state: &BlockState,
        min_y: i32,
        get_block: F,
    ) -> bool
    where
        F: Fn(i32) -> &'static BlockState,
    {
        let first_available = self.get(heightmap_type, local_x, local_z, min_y) + 1;
        if local_y <= first_available - 2 {
            return false;
        }

        if heightmap_type.is_opaque(block_state) {
            if local_y >= first_available {
                self.set(heightmap_type, local_x, local_z, local_y, min_y);
                return true;
            }
        } else if first_available - 1 == local_y {
            for y in (min_y..local_y).rev() {
                let state = get_block(y);
                if heightmap_type.is_opaque(state) {
                    self.set(heightmap_type, local_x, local_z, y, min_y);
                    return true;
                }
            }
            self.set(heightmap_type, local_x, local_z, min_y - 1, min_y);
            return true;
        }

        false
    }
}

/// The Heightmap for a completely empty chunk
impl Default for Heightmaps {
    fn default() -> Self {
        Self {
            motion_blocking: None,
            motion_blocking_no_leaves: None,
            world_surface: None,
        }
    }
}
