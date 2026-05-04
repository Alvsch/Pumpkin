use pumpkin_util::math::{square_f64, vector3::Vector3};
use pumpkin_world_core::biome_coords;
use sha2::{Digest, Sha256};
use std::cell::RefCell;

use enum_dispatch::enum_dispatch;
use pumpkin_data::chunk::{Biome, BiomeTree, NETHER_BIOME_SOURCE, OVERWORLD_BIOME_SOURCE};

use crate::noise::router::multi_noise_sampler::MultiNoiseSampler;

pub mod end;
pub mod multi_noise;
pub mod position_finder;

// This blends biome boundaries, returning which biome to populate the surface on based on the seed
pub fn get_biome_blend(
    bottom_y: i8,
    height: u16,
    seed: i64,
    x: i32,
    y: i32,
    z: i32,
) -> Vector3<i32> {
    // This is the "left" side of the biome boundary
    let offset_x = x - 2;
    let offset_y = y - 2;
    let offset_z = z - 2;
    let biome_x = biome_coords::from_block(offset_x);
    let biome_y = biome_coords::from_block(offset_y);
    let biome_z = biome_coords::from_block(offset_z);
    // &'ing 3 gives values of 0-3, it is also the data we removed when converting to biome coords
    // This is effectively "quarters" into the biome
    // Original was "/ 4.0" but we use "* 0.25" as multiplication can be faster
    let biome_x_quarters = (offset_x & 0b11) as f64 * 0.25;
    let biome_y_quarters = (offset_y & 0b11) as f64 * 0.25;
    let biome_z_quarters = (offset_z & 0b11) as f64 * 0.25;

    let mut best_permutation = 0;
    let mut best_score = f64::INFINITY;
    for permutation in 0..8 {
        let should_maintain_x = (permutation & 0b100) == 0;
        let should_maintain_y = (permutation & 0b010) == 0;
        let should_maintain_z = (permutation & 0b001) == 0;

        // If we are shifting, add 1 to the biome coords
        let shifted_biome_x = if should_maintain_x {
            biome_x
        } else {
            biome_x + 1
        };
        let shifted_biome_y = if should_maintain_y {
            biome_y
        } else {
            biome_y + 1
        };
        let shifted_biome_z = if should_maintain_z {
            biome_z
        } else {
            biome_z + 1
        };

        // And reflect the "quarters" across the shift
        let shifted_biome_x_quarters = if should_maintain_x {
            biome_x_quarters
        } else {
            biome_x_quarters - 1.0
        };
        let shifted_biome_y_quarters = if should_maintain_y {
            biome_y_quarters
        } else {
            biome_y_quarters - 1.0
        };
        let shifted_biome_z_quarters = if should_maintain_z {
            biome_z_quarters
        } else {
            biome_z_quarters - 1.0
        };

        let permutation_score = score_permutation(
            seed,
            shifted_biome_x,
            shifted_biome_y,
            shifted_biome_z,
            shifted_biome_x_quarters,
            shifted_biome_y_quarters,
            shifted_biome_z_quarters,
        );

        if best_score > permutation_score {
            best_score = permutation_score;
            best_permutation = permutation;
        }
    }

    // Now check if we want to use the "left" side or the "right" side
    let biome_x = if (best_permutation & 0b100) == 0 {
        biome_x
    } else {
        biome_x + 1
    };
    let biome_y = if (best_permutation & 0b010) == 0 {
        biome_y
    } else {
        biome_y + 1
    };
    let biome_z = if (best_permutation & 0b001) == 0 {
        biome_z
    } else {
        biome_z + 1
    };

    // Java's `getBiomeForNoiseGen`
    let bottom_y = bottom_y as i32;
    let biome_bottom = biome_coords::from_block(bottom_y);
    let biome_top = biome_bottom + biome_coords::from_block(height as i32) - 1;
    let biome_y = biome_y.clamp(biome_bottom, biome_top);

    Vector3::new(biome_x, biome_y, biome_z)
}

// This is effectively getting a random offset (+/- 0.0-0.8ish) to our biome position quarters and
// returning a hypotenuse squared of the parts + the offset
const fn score_permutation(
    seed: i64,
    x: i32,
    y: i32,
    z: i32,
    x_part: f64,
    y_part: f64,
    z_part: f64,
) -> f64 {
    let mix = salt_mix(seed, x as i64);
    let mix = salt_mix(mix, y as i64);
    let mix = salt_mix(mix, z as i64);
    let mix = salt_mix(mix, x as i64);
    let mix = salt_mix(mix, y as i64);
    let mix = salt_mix(mix, z as i64);
    let offset_x = scale_mix(mix);
    let mix = salt_mix(mix, seed);
    let offset_y = scale_mix(mix);
    let mix = salt_mix(mix, seed);
    let offset_z = scale_mix(mix);

    square_f64(z_part + offset_z) + square_f64(y_part + offset_y) + square_f64(x_part + offset_x)
}

#[inline]
pub const fn scale_mix(l: i64) -> f64 {
    // Shifting and then masking with 1023 (1024 - 1)
    // This is mathematically identical to floor_mod(l >> 24, 1024)
    // but executes in a single CPU cycle.
    let d = ((l >> 24) & 1023) as f64 / 1024.0;

    (d - 0.5) * 0.9
}

#[inline]
const fn salt_mix(seed: i64, salt: i64) -> i64 {
    let mixed_seed = seed.wrapping_mul(
        seed.wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407),
    );
    mixed_seed.wrapping_add(salt)
}

thread_local! {
    /// A shortcut; check if last used biome is what we should use
    static LAST_RESULT_NODE: RefCell<Option<&'static BiomeTree>> = const {RefCell::new(None) };
}

#[enum_dispatch]
pub trait BiomeSupplier {
    fn biome(&self, x: i32, y: i32, z: i32, noise: &mut MultiNoiseSampler<'_>) -> &'static Biome;
}

#[derive(Clone, Copy)]
pub struct MultiNoiseBiomeSupplier {
    source: &'static BiomeTree,
}

impl MultiNoiseBiomeSupplier {
    pub const OVERWORLD: Self = Self::new(&OVERWORLD_BIOME_SOURCE);
    pub const NETHER: Self = Self::new(&NETHER_BIOME_SOURCE);

    const fn new(source: &'static BiomeTree) -> Self {
        Self { source }
    }
}

impl BiomeSupplier for MultiNoiseBiomeSupplier {
    fn biome(&self, x: i32, y: i32, z: i32, noise: &mut MultiNoiseSampler<'_>) -> &'static Biome {
        let point = noise.sample(x, y, z);
        let point_list = point.convert_to_list();
        LAST_RESULT_NODE.with_borrow_mut(|last_result| self.source.get(&point_list, last_result))
    }
}

#[must_use]
pub fn hash_seed(seed: u64) -> i64 {
    let mut hasher = Sha256::new();
    hasher.update(seed.to_le_bytes());
    let result = hasher.finalize();
    i64::from_le_bytes(result[..8].try_into().unwrap())
}

#[cfg(test)]
mod test {
    use pumpkin_data::{chunk::Biome, dimension::Dimension};
    use pumpkin_util::{math::vector3::Vector3, read_data_from_file, world_seed::Seed};
    use pumpkin_world_core::biome_coords;
    use serde::Deserialize;

    use crate::{
        biome::{
            BiomeSupplier, MultiNoiseBiomeSupplier, get_biome_blend, hash_seed, scale_mix,
            score_permutation,
        },
        generator::{GeneratorInit, VanillaGenerator},
        noise::router::multi_noise_sampler::{MultiNoiseSampler, MultiNoiseSamplerBuilderOptions},
        positions::chunk_pos,
        proto_chunk::ProtoChunk,
    };

    use super::salt_mix;

    #[test]
    fn mix_seed() {
        let seed = salt_mix(12345678, 12345678);
        assert_eq!(seed, 2937271135939595220);
    }

    #[test]
    fn permutation() {
        let seed = hash_seed(0);
        let score = score_permutation(seed, 123, 456, 456, 0.25, 0.5, 0.75);
        assert_eq!(score, 1.276986312866211);
    }

    #[test]
    fn biome_blend() {
        let biome_pos = get_biome_blend(-64, 384, 1234567890, 123, 123, 123);
        assert_eq!(biome_pos, Vector3::new(31, 30, 30));
    }

    #[test]
    fn scale() {
        let seed = scale_mix(12345678);
        assert_eq!(seed, -0.45);
    }

    #[test]
    fn chunk_wide_blend() {
        let data: Vec<(i32, i32, i32, i32, i32, i32)> =
            read_data_from_file!("../../assets/biome_mixer.json");

        let seed = hash_seed((-777i64) as u64);
        for (i, (x, y, z, result_x, result_y, result_z)) in data.into_iter().enumerate() {
            let result = get_biome_blend(i8::MIN, u16::MAX, seed, x, y, z);
            let expected = Vector3::new(result_x, result_y, result_z);
            assert_eq!(
                result, expected,
                "Expected: {expected:?}, was: {result:?} ({i})"
            );
        }
    }

    #[test]
    fn biome_desert() {
        let seed = 13579;

        let generator = VanillaGenerator::new(Seed(seed as u64), Dimension::OVERWORLD);
        let multi_noise_config = MultiNoiseSamplerBuilderOptions::new(1, 1, 1);
        let mut sampler =
            MultiNoiseSampler::generate(&generator.base_router.multi_noise, &multi_noise_config);

        let biome = MultiNoiseBiomeSupplier::OVERWORLD.biome(-24, 1, 8, &mut sampler);
        assert_eq!(biome, &Biome::DESERT);
    }

    #[test]
    fn wide_area_surface() {
        #[derive(Deserialize)]
        struct BiomeData {
            x: i32,
            z: i32,
            data: Vec<(i32, i32, i32, u8)>,
        }

        let expected_data: Vec<BiomeData> =
            read_data_from_file!("../../assets/biome_no_blend_no_beard_0.json");

        let seed = 0;
        use crate::generator::{GeneratorInit, VanillaGenerator};
        use pumpkin_util::world_seed::Seed;
        let generator = VanillaGenerator::new(Seed(seed as u64), Dimension::OVERWORLD);

        for data in expected_data {
            let chunk_x = data.x;
            let chunk_z = data.z;

            let mut chunk = ProtoChunk::new(chunk_x, chunk_z, &generator);

            // Create MultiNoiseSampler for populate_biomes

            let start_x = chunk_pos::start_block_x(chunk_x);
            let start_z = chunk_pos::start_block_z(chunk_z);

            let horizontal_biome_end = biome_coords::from_block(16);
            let multi_noise_config = MultiNoiseSamplerBuilderOptions::new(
                biome_coords::from_block(start_x),
                biome_coords::from_block(start_z),
                horizontal_biome_end as usize,
            );
            let mut multi_noise_sampler = MultiNoiseSampler::generate(
                &generator.base_router.multi_noise,
                &multi_noise_config,
            );

            chunk.populate_biomes(&generator, &mut multi_noise_sampler);

            for (biome_x, biome_y, biome_z, biome_id) in data.data {
                let calculated_biome = chunk.get_biome(biome_x, biome_y, biome_z);

                assert_eq!(
                    biome_id,
                    calculated_biome.id,
                    "Expected {:?} was {:?} at {},{},{} ({},{})",
                    Biome::from_id(biome_id),
                    calculated_biome,
                    biome_x,
                    biome_y,
                    biome_z,
                    data.x,
                    data.z
                );
            }
        }
    }

    #[test]
    fn hash_seed_test() {
        let hashed_seed = hash_seed(0);
        assert_eq!(8794265229978523055, hashed_seed);

        let hashed_seed = hash_seed((-777i64) as u64);
        assert_eq!(-1087248400229165450, hashed_seed);
    }
}
