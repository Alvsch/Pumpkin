use std::{hash::Hash, iter::repeat_n};

use pumpkin_util::encompassing_bits;
use tracing::warn;

use crate::palette::{
    AbstractCube,
    heterogeneous_palette_data::{HeterogeneousPaletteData, PaletteStorage},
};

/// A paletted container is a cube of registry ids. It uses a custom compression scheme based on how
/// may distinct registry ids are in the cube.
#[derive(Clone)]
pub enum PalettedContainer<V: Hash + Eq + Copy + Default, const DIM: usize> {
    Homogeneous(V),
    Heterogeneous(Box<HeterogeneousPaletteData<V, DIM>>),
}

impl<V: Hash + Eq + Copy + Default, const DIM: usize> PalettedContainer<V, DIM> {
    pub const SIZE: usize = DIM;
    pub const VOLUME: usize = DIM * DIM * DIM;

    fn from_cube(cube: Box<AbstractCube<V, DIM>>) -> Self {
        let mut palette: Vec<V> = Vec::new();
        let mut counts: Vec<u16> = Vec::new();

        // Iterate over the flattened cube to populate the palette and counts
        for val in cube.as_flattened().as_flattened() {
            if let Some(index) = palette.iter().position(|v| v == val) {
                // Value already exists, increment its count
                counts[index] += 1;
            } else {
                // New value, add it to the palette and start its count
                palette.push(*val);
                counts.push(1);
            }
        }

        if palette.len() == 1 {
            // Fast path: the cube is homogeneous, so we can store just one value
            Self::Homogeneous(palette[0])
        } else {
            // Heterogeneous cube, store the full data
            if palette.len() <= 256 && std::mem::size_of::<V>() > 1 {
                let mut indices = Box::new([[[0u8; DIM]; DIM]; DIM]);
                for (i, v) in cube.as_flattened().as_flattened().iter().enumerate() {
                    let idx = palette.iter().position(|p| p == v).unwrap();
                    indices.as_flattened_mut().as_flattened_mut()[i] = idx as u8;
                }
                Self::Heterogeneous(Box::new(HeterogeneousPaletteData {
                    storage: PaletteStorage::Indexed(indices),
                    palette,
                    counts,
                }))
            } else {
                Self::Heterogeneous(Box::new(HeterogeneousPaletteData {
                    storage: PaletteStorage::Dense(cube),
                    palette,
                    counts,
                }))
            }
        }
    }

    pub fn bits_per_entry(&self) -> u8 {
        match self {
            Self::Homogeneous(_) => 0,
            Self::Heterogeneous(data) => encompassing_bits(data.counts.len()),
        }
    }

    pub fn to_palette_and_packed_data(&self, bits_per_entry: u8) -> (Box<[V]>, Box<[i64]>) {
        match self {
            Self::Homogeneous(registry_id) => (Box::new([*registry_id]), Box::new([])),
            Self::Heterogeneous(data) => {
                debug_assert!(bits_per_entry >= encompassing_bits(data.counts.len()));
                debug_assert!(bits_per_entry <= 15);

                // Don't use HashMap's here, because its slow
                let blocks_per_i64 = 64 / bits_per_entry;

                let packed_indices: Box<[i64]> = match &data.storage {
                    PaletteStorage::Dense(cube) => cube
                        .as_flattened()
                        .as_flattened()
                        .chunks(blocks_per_i64 as usize)
                        .map(|chunk| {
                            chunk.iter().enumerate().fold(0, |acc, (index, key)| {
                                let key_index =
                                    data.palette.iter().position(|&x| x == *key).unwrap();
                                debug_assert!((1 << bits_per_entry) > key_index);

                                let packed_offset_index =
                                    (key_index as u64) << (bits_per_entry as u64 * index as u64);
                                acc | packed_offset_index as i64
                            })
                        })
                        .collect(),
                    PaletteStorage::Indexed(indices) => indices
                        .as_flattened()
                        .as_flattened()
                        .chunks(blocks_per_i64 as usize)
                        .map(|chunk| {
                            chunk.iter().enumerate().fold(0, |acc, (index, key_index)| {
                                let key_index = *key_index as usize;
                                debug_assert!((1 << bits_per_entry) > key_index);

                                let packed_offset_index =
                                    (key_index as u64) << (bits_per_entry as u64 * index as u64);
                                acc | packed_offset_index as i64
                            })
                        })
                        .collect(),
                };

                (data.palette.clone().into_boxed_slice(), packed_indices)
            }
        }
    }

    #[must_use]
    pub fn from_palette_and_packed_data(
        palette: &[V],
        packed_data: &[i64],
        minimum_bits_per_entry: u8,
    ) -> Self {
        if palette.is_empty() {
            warn!("No palette data! Defaulting...");
            return Self::Homogeneous(V::default());
        }

        if palette.len() == 1 {
            return Self::Homogeneous(palette[0]);
        }

        let bits_per_key = encompassing_bits(palette.len()).max(minimum_bits_per_entry);
        let index_mask = (1 << bits_per_key) - 1;
        let keys_per_i64 = 64 / bits_per_key;

        // Optimized path for indexed storage if palette is small enough
        if palette.len() <= 256 && std::mem::size_of::<V>() > 1 {
            let mut indices = Box::new([[[0u8; DIM]; DIM]; DIM]);
            let mut counts = vec![0u16; palette.len()];
            let indices_flat = indices.as_flattened_mut().as_flattened_mut();

            let mut packed_data_iter = packed_data.iter();
            let mut current_packed_word = *packed_data_iter.next().unwrap_or(&0);

            for (i, index_out) in indices_flat.iter_mut().enumerate().take(Self::VOLUME) {
                let bit_index_in_word = i % keys_per_i64 as usize;
                if bit_index_in_word == 0 && i > 0 {
                    current_packed_word = *packed_data_iter.next().unwrap_or(&0);
                }

                let lookup_index = ((current_packed_word as u64)
                    >> (bit_index_in_word as u64 * bits_per_key as u64))
                    & index_mask;

                let idx = lookup_index as usize;
                if idx < palette.len() {
                    *index_out = idx as u8;
                    counts[idx] += 1;
                } else {
                    warn!("Lookup index out of bounds! Defaulting...");
                    // value is already 0, and counts[0] will be updated correctly if we track it
                }
            }
            // fix counts[0] if it was skipped in out-of-bounds cases (rare)
            // But actually we should just ensure it's correct.

            return Self::Heterogeneous(Box::new(HeterogeneousPaletteData {
                storage: PaletteStorage::Indexed(indices),
                palette: palette.to_vec(),
                counts,
            }));
        }

        let mut decompressed_values = Vec::with_capacity(Self::VOLUME);

        let mut packed_data_iter = packed_data.iter();
        let mut current_packed_word = *packed_data_iter.next().unwrap_or(&0);

        for i in 0..Self::VOLUME {
            let bit_index_in_word = i % keys_per_i64 as usize;

            if bit_index_in_word == 0 && i > 0 {
                current_packed_word = *packed_data_iter.next().unwrap_or(&0);
            }

            let lookup_index = (current_packed_word as u64
                >> (bit_index_in_word as u64 * bits_per_key as u64))
                & index_mask;

            let value = palette
                .get(lookup_index as usize)
                .copied()
                .unwrap_or_else(|| {
                    warn!("Lookup index out of bounds! Defaulting...");
                    V::default()
                });

            decompressed_values.push(value);
        }

        // Now, with all decompressed values, build the counts.
        let mut counts = vec![0; palette.len()];

        for &value in &decompressed_values {
            // This is the key optimization: find the index in the palette Vec
            // and increment the corresponding count.
            if let Some(index) = palette.iter().position(|v| v == &value) {
                counts[index] += 1;
            } else {
                // This case should ideally not happen if the palette is complete.
                warn!("Decompressed value not found in palette!");
            }
        }

        let mut cube = Box::new([[[V::default(); DIM]; DIM]; DIM]);
        cube.as_flattened_mut()
            .as_flattened_mut()
            .copy_from_slice(&decompressed_values);

        Self::Heterogeneous(Box::new(HeterogeneousPaletteData {
            storage: PaletteStorage::Dense(cube),
            palette: palette.to_vec(),
            counts,
        }))
    }

    pub fn get(&self, x: usize, y: usize, z: usize) -> V {
        match self {
            Self::Homogeneous(value) => *value,
            Self::Heterogeneous(data) => data.get(x, y, z),
        }
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, value: V) -> V {
        debug_assert!(x < Self::SIZE);
        debug_assert!(y < Self::SIZE);
        debug_assert!(z < Self::SIZE);

        match self {
            Self::Homogeneous(original) => {
                let original = *original;
                if value != original {
                    let mut cube = Box::new([[[original; DIM]; DIM]; DIM]);
                    cube[y][z][x] = value;
                    *self = Self::from_cube(cube);
                }
                original
            }
            Self::Heterogeneous(data) => {
                let original = data.set(x, y, z, value);
                if data.counts.len() == 1 {
                    *self = Self::Homogeneous(data.palette[0]);
                }
                original
            }
        }
    }

    pub fn iter(&self) -> Box<dyn Iterator<Item = &V> + '_> {
        match self {
            Self::Homogeneous(registry_id) => Box::new(repeat_n(registry_id, Self::VOLUME)),
            Self::Heterogeneous(data) => match &data.storage {
                PaletteStorage::Dense(cube) => Box::new(cube.as_flattened().as_flattened().iter()),
                PaletteStorage::Indexed(indices) => Box::new(
                    indices
                        .as_flattened()
                        .as_flattened()
                        .iter()
                        .map(|&idx| &data.palette[idx as usize]),
                ),
            },
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Homogeneous(value) => *value == V::default(),
            Self::Heterogeneous(_) => false,
        }
    }
}

impl<V: Default + Hash + Eq + Copy, const DIM: usize> Default for PalettedContainer<V, DIM> {
    fn default() -> Self {
        Self::Homogeneous(V::default())
    }
}
