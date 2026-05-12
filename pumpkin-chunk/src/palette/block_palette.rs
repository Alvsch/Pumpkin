use std::collections::HashMap;

use pumpkin_data::{
    BlockState,
    block_properties::{has_random_ticks, is_air, is_liquid},
};
use pumpkin_nbt::nbt_long_array;
use pumpkin_util::encompassing_bits;
use serde::{Deserialize, Serialize};

use crate::{
    has_random_ticking_fluid,
    palette::{
        network::{BeNetworkSerialization, NetworkPalette, NetworkSerialization},
        paletted_container::PalettedContainer,
    },
};

const BLOCK_DISK_MIN_BITS: u8 = 4;
const BLOCK_NETWORK_MIN_MAP_BITS: u8 = 4;
const BLOCK_NETWORK_MAX_MAP_BITS: u8 = 8;
pub const BLOCK_NETWORK_MAX_BITS: u8 = 15;

pub type BlockPalette = PalettedContainer<u16, 16>;

impl BlockPalette {
    #[must_use]
    pub fn convert_network(&self) -> NetworkSerialization<u16> {
        match self {
            Self::Homogeneous(registry_id) => NetworkSerialization {
                bits_per_entry: 0,
                palette: NetworkPalette::Single(*registry_id),
                packed_data: Box::new([]),
            },
            Self::Heterogeneous(data) => {
                let raw_bits_per_entry = encompassing_bits(data.counts.len());
                if raw_bits_per_entry > BLOCK_NETWORK_MAX_MAP_BITS {
                    let bits_per_entry = BLOCK_NETWORK_MAX_BITS;
                    let values_per_i64 = 64 / bits_per_entry;
                    let mut packed_data = Vec::new();
                    let mut current_idx = 0;
                    while current_idx < Self::VOLUME {
                        let mut acc = 0u64;
                        for i in 0..values_per_i64 as usize {
                            if current_idx + i < Self::VOLUME {
                                let y = (current_idx + i) / (Self::SIZE * Self::SIZE);
                                let z = ((current_idx + i) / Self::SIZE) % Self::SIZE;
                                let x = (current_idx + i) % Self::SIZE;
                                let value = data.get(x, y, z);
                                debug_assert!((1 << bits_per_entry) > value);
                                acc |= (value as u64) << (bits_per_entry as u64 * i as u64);
                            }
                        }
                        packed_data.push(acc as i64);
                        current_idx += values_per_i64 as usize;
                    }

                    NetworkSerialization {
                        bits_per_entry,
                        palette: NetworkPalette::Direct,
                        packed_data: packed_data.into_boxed_slice(),
                    }
                } else {
                    let bits_per_entry = raw_bits_per_entry.max(BLOCK_NETWORK_MIN_MAP_BITS);
                    let (palette, packed) = self.to_palette_and_packed_data(bits_per_entry);

                    NetworkSerialization {
                        bits_per_entry,
                        palette: NetworkPalette::Indirect(palette),
                        packed_data: packed,
                    }
                }
            }
        }
    }

    #[must_use]
    pub fn convert_be_network(&self) -> BeNetworkSerialization<u16> {
        match self {
            Self::Homogeneous(registry_id) => BeNetworkSerialization {
                bits_per_entry: 0,
                palette: NetworkPalette::Single(BlockState::to_be_network_id(*registry_id)),
                packed_data: Box::new([]),
            },
            Self::Heterogeneous(data) => {
                let bits_per_entry = encompassing_bits(data.palette.len());

                let key_to_index_map: HashMap<_, usize> = data
                    .palette
                    .iter()
                    .enumerate()
                    .map(|(index, key)| (*key, index))
                    .collect();

                let blocks_per_word = 32 / bits_per_entry;
                let expected_word_count = Self::VOLUME.div_ceil(blocks_per_word as usize);
                let mut packed_data = Vec::with_capacity(expected_word_count);

                let mut current_word: u32 = 0;
                let mut current_index_in_word = 0;

                for x in 0..16 {
                    for y in 0..16 {
                        for z in 0..16 {
                            // Java has it in y, z, x order, so we need to convert it back to x, y, z
                            // Please test your code on bedrock before merging
                            let key = data.get(x, z, y);
                            let key_index = key_to_index_map.get(&key).unwrap();
                            debug_assert!((1 << bits_per_entry) > *key_index);

                            current_word |= (*key_index as u32)
                                << (bits_per_entry as u32 * current_index_in_word);
                            current_index_in_word += 1;

                            if current_index_in_word == blocks_per_word as u32 {
                                packed_data.push(current_word);
                                current_word = 0;
                                current_index_in_word = 0;
                            }
                        }
                    }
                }

                // Push any remaining bits if the volume isn't a multiple of blocks_per_word
                if current_index_in_word > 0 {
                    packed_data.push(current_word);
                }

                BeNetworkSerialization {
                    bits_per_entry,
                    palette: NetworkPalette::Indirect(
                        data.palette
                            .iter()
                            .map(|&id| BlockState::to_be_network_id(id))
                            .collect(),
                    ),
                    packed_data: packed_data.into_boxed_slice(),
                }
            }
        }
    }

    /// Check if the entire chunk is filled with only air
    #[must_use]
    pub fn has_only_air(&self) -> bool {
        match self {
            Self::Homogeneous(id) => is_air(*id),
            Self::Heterogeneous(data) => data.palette.iter().all(|&id| is_air(id)),
        }
    }

    #[must_use]
    pub fn random_ticking_counts(&self) -> (u16, u16) {
        match self {
            Self::Homogeneous(registry_id) => {
                let block_count = if has_random_ticks(*registry_id) {
                    Self::VOLUME as u16
                } else {
                    0
                };
                let fluid_count = if has_random_ticking_fluid(*registry_id) {
                    Self::VOLUME as u16
                } else {
                    0
                };
                (block_count, fluid_count)
            }
            Self::Heterogeneous(data) => data.palette.iter().zip(data.counts.iter()).fold(
                (0, 0),
                |(block_count, fluid_count), (registry_id, count)| {
                    let block_count = if has_random_ticks(*registry_id) {
                        block_count.saturating_add(*count)
                    } else {
                        block_count
                    };
                    let fluid_count = if has_random_ticking_fluid(*registry_id) {
                        fluid_count.saturating_add(*count)
                    } else {
                        fluid_count
                    };
                    (block_count, fluid_count)
                },
            ),
        }
    }

    #[must_use]
    pub fn non_air_block_count(&self) -> u16 {
        match self {
            Self::Homogeneous(registry_id) => {
                if is_air(*registry_id) {
                    0
                } else {
                    Self::VOLUME as u16
                }
            }
            Self::Heterogeneous(data) => data
                .palette
                .iter()
                .zip(data.counts.iter())
                .filter_map(|(registry_id, count)| (!is_air(*registry_id)).then_some(*count))
                .sum(),
        }
    }

    #[must_use]
    pub fn liquid_block_count(&self) -> u16 {
        match self {
            Self::Homogeneous(registry_id) => {
                if is_liquid(*registry_id) {
                    0
                } else {
                    Self::VOLUME as u16
                }
            }
            Self::Heterogeneous(data) => data
                .palette
                .iter()
                .zip(data.counts.iter())
                .filter_map(|(registry_id, count)| (!is_liquid(*registry_id)).then_some(*count))
                .sum(),
        }
    }

    #[must_use]
    pub fn from_disk_nbt(nbt: SectionBlockStates) -> Self {
        let palette = nbt.palette;

        Self::from_palette_and_packed_data(
            &palette,
            nbt.data.as_ref().unwrap_or(&Box::default()),
            BLOCK_DISK_MIN_BITS,
        )
    }

    #[must_use]
    pub fn to_disk_nbt(&self) -> SectionBlockStates {
        let bits_per_entry = self.bits_per_entry().max(BLOCK_DISK_MIN_BITS);
        let (palette, packed_data) = self.to_palette_and_packed_data(bits_per_entry);
        SectionBlockStates {
            data: if packed_data.is_empty() {
                None
            } else {
                Some(packed_data)
            },
            palette,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SectionBlockStates {
    #[serde(
        serialize_with = "nbt_long_array",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) data: Option<Box<[i64]>>,
    pub(crate) palette: Box<[u16]>,
}
