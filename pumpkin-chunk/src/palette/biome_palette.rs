use pumpkin_nbt::nbt_long_array;
use pumpkin_util::encompassing_bits;
use serde::{Deserialize, Serialize};

use crate::palette::{
    network::{NetworkPalette, NetworkSerialization},
    paletted_container::PalettedContainer,
};

const BIOME_DISK_MIN_BITS: u8 = 0;
const BIOME_NETWORK_MIN_MAP_BITS: u8 = 1;
const BIOME_NETWORK_MAX_MAP_BITS: u8 = 3;
pub const BIOME_NETWORK_MAX_BITS: u8 = 7;

pub type BiomePalette = PalettedContainer<u8, 4>;

impl BiomePalette {
    #[must_use]
    pub fn convert_network(&self) -> NetworkSerialization<u8> {
        match self {
            Self::Homogeneous(registry_id) => NetworkSerialization {
                bits_per_entry: 0,
                palette: NetworkPalette::Single(*registry_id),
                packed_data: Box::new([]),
            },
            Self::Heterogeneous(data) => {
                let raw_bits_per_entry = encompassing_bits(data.counts.len());
                if raw_bits_per_entry > BIOME_NETWORK_MAX_MAP_BITS {
                    let bits_per_entry = BIOME_NETWORK_MAX_BITS;
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
                    let bits_per_entry = raw_bits_per_entry.max(BIOME_NETWORK_MIN_MAP_BITS);
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
    pub fn from_disk_nbt(nbt: SectionBiomes) -> Self {
        let palette = nbt.palette;

        Self::from_palette_and_packed_data(
            &palette,
            nbt.data.as_ref().unwrap_or(&Box::default()),
            BIOME_DISK_MIN_BITS,
        )
    }

    #[must_use]
    pub fn to_disk_nbt(&self) -> SectionBiomes {
        #[expect(clippy::unnecessary_min_or_max)]
        let bits_per_entry = self.bits_per_entry().max(BIOME_DISK_MIN_BITS);
        let (palette, packed_data) = self.to_palette_and_packed_data(bits_per_entry);
        SectionBiomes {
            data: if packed_data.is_empty() {
                None
            } else {
                Some(packed_data)
            },
            palette,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SectionBiomes {
    #[serde(
        serialize_with = "nbt_long_array",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) data: Option<Box<[i64]>>,
    pub(crate) palette: Box<[u8]>,
}
