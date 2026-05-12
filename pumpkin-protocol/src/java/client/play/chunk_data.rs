use crate::WritingError;
use crate::java::client::play::serialize_light;
use crate::{ClientPacket, VarInt, ser::NetworkWriteExt};
use pumpkin_chunk::palette::network::NetworkPalette;
use pumpkin_chunk::{ChunkData, Heightmaps, LightData, Sections};
use pumpkin_data::block_state_remap::remap_block_state_for_version;
use pumpkin_data::packet::CURRENT_MC_VERSION;
use pumpkin_data::packet::clientbound::PLAY_LEVEL_CHUNK_WITH_LIGHT;
use pumpkin_macros::java_packet;
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::math::position::{BlockPos, get_local_cord};
use pumpkin_util::version::MinecraftVersion;
use rustc_hash::FxHashMap;
use std::io::Write;

/// Sent by the server to provide the client with the full data for a chunk.
///
/// This includes heightmaps, the actual block and biome data (organized into sections),
/// block entities (like signs or chests), and the light level information for both
/// sky and block light.
#[java_packet(PLAY_LEVEL_CHUNK_WITH_LIGHT)]
pub struct CChunkData {
    pub x: i32,
    pub z: i32,
    pub chunk_data: ChunkData,
    pub light_data: LightData,
}

impl ClientPacket for CChunkData {
    fn write_packet_data(
        &self,
        mut write: impl Write,
        version: &MinecraftVersion,
    ) -> Result<(), WritingError> {
        // Chunk X
        write.write_i32_be(self.x)?;
        // Chunk Z
        write.write_i32_be(self.z)?;

        let data = &self.chunk_data;
        serialize_heightmap(&mut write, &data.heightmaps, *version)?;
        serialize_data(&mut write, &data.sections, *version)?;
        serialize_block_entities(&mut write, &data.block_entities)?;
        serialize_light(&mut write, &self.light_data)?;

        Ok(())
    }
}

fn serialize_heightmap(
    write: &mut impl Write,
    heightmaps: &Heightmaps,
    version: MinecraftVersion,
) -> Result<(), WritingError> {
    if version <= MinecraftVersion::V_1_21_4 {
        pumpkin_nbt::serializer::to_bytes_unnamed(heightmaps, write)
            .map_err(|err| WritingError::Serde(err.to_string()))?;
    } else {
        write.write_var_int(&VarInt(3))?; // Map size

        let mut write_heightmap = |index: i32, data: &[i64]| -> Result<(), WritingError> {
            write.write_var_int(&VarInt(index))?;
            write.write_var_int(&VarInt(data.len() as i32))?;
            for val in data {
                write.write_i64_be(*val)?;
            }
            Ok(())
        };

        write_heightmap(1, heightmaps.world_surface.as_deref().unwrap_or(&[0; 37]))?;
        write_heightmap(4, heightmaps.motion_blocking.as_deref().unwrap_or(&[0; 37]))?;
        write_heightmap(
            5,
            heightmaps
                .motion_blocking_no_leaves
                .as_deref()
                .unwrap_or(&[0; 37]),
        )?;
    }

    Ok(())
}

#[expect(clippy::too_many_lines)]
fn serialize_data(
    write: &mut impl Write,
    sections: &Sections,
    version: MinecraftVersion,
) -> Result<(), WritingError> {
    let mut blocks_and_biomes_buf = Vec::new();
    for (block_palette, biome_palette) in sections
        .block_sections
        .iter()
        .zip(sections.biome_sections.iter())
    {
        let non_empty_block_count = block_palette.non_air_block_count() as i16;
        blocks_and_biomes_buf.write_i16_be(non_empty_block_count)?;
        if version >= MinecraftVersion::V_26_1 {
            // New in 26.1, fluid count
            let liquid_count = block_palette.liquid_block_count() as i16;
            blocks_and_biomes_buf.write_i16_be(liquid_count)?;
        }

        let mut block_network = block_palette.convert_network();
        if version < CURRENT_MC_VERSION {
            match &mut block_network.palette {
                NetworkPalette::Single(registry_id) => {
                    *registry_id = remap_block_state_for_version(*registry_id, version);
                }
                NetworkPalette::Indirect(palette) => {
                    for registry_id in palette.iter_mut() {
                        *registry_id = remap_block_state_for_version(*registry_id, version);
                    }
                }
                NetworkPalette::Direct => {
                    let bits_per_entry = usize::from(block_network.bits_per_entry);
                    let values_per_i64 = 64 / bits_per_entry;
                    let id_mask = (1u64 << bits_per_entry) - 1;

                    for packed_word in &mut block_network.packed_data {
                        let mut remapped_word = 0u64;
                        let packed_word_u64 = *packed_word as u64;
                        for index in 0..values_per_i64 {
                            let shift = index * bits_per_entry;
                            let state_id = ((packed_word_u64 >> shift) & id_mask) as u16;
                            let remapped_id = remap_block_state_for_version(state_id, version);
                            remapped_word |= u64::from(remapped_id) << shift;
                        }
                        *packed_word = remapped_word as i64;
                    }
                }
            }
        }
        blocks_and_biomes_buf.write_u8(block_network.bits_per_entry)?;

        match block_network.palette {
            NetworkPalette::Single(registry_id) => {
                blocks_and_biomes_buf.write_var_int(&registry_id.into())?;
            }
            NetworkPalette::Indirect(palette) => {
                blocks_and_biomes_buf.write_var_int(&palette.len().try_into().map_err(
                    |_| {
                        WritingError::Message(format!(
                            "{} is not representable as a VarInt!",
                            palette.len()
                        ))
                    },
                )?)?;
                for registry_id in palette {
                    blocks_and_biomes_buf.write_var_int(&registry_id.into())?;
                }
            }
            NetworkPalette::Direct => {}
        }

        if version <= MinecraftVersion::V_1_21_4 {
            blocks_and_biomes_buf.write_list(&block_network.packed_data, |buf, &packed| {
                buf.write_i64_be(packed)
            })?;
        } else {
            for packed in block_network.packed_data {
                blocks_and_biomes_buf.write_i64_be(packed)?;
            }
        }

        let biome_network = biome_palette.convert_network();
        blocks_and_biomes_buf.write_u8(biome_network.bits_per_entry)?;

        match biome_network.palette {
            NetworkPalette::Single(registry_id) => {
                blocks_and_biomes_buf.write_var_int(&registry_id.into())?;
            }
            NetworkPalette::Indirect(palette) => {
                blocks_and_biomes_buf.write_var_int(&palette.len().try_into().map_err(
                    |_| {
                        WritingError::Message(format!(
                            "{} is not representable as a VarInt!",
                            palette.len()
                        ))
                    },
                )?)?;
                for registry_id in palette {
                    blocks_and_biomes_buf.write_var_int(&registry_id.into())?;
                }
            }
            NetworkPalette::Direct => (),
        }

        if version <= MinecraftVersion::V_1_21_4 {
            blocks_and_biomes_buf.write_list(&biome_network.packed_data, |buf, &packed| {
                buf.write_i64_be(packed)
            })?;
        } else {
            for packed in biome_network.packed_data {
                blocks_and_biomes_buf.write_i64_be(packed)?;
            }
        }
    }

    write.write_var_int(&blocks_and_biomes_buf.len().try_into().map_err(|_| {
        WritingError::Message(format!(
            "{} is not representable as a VarInt!",
            blocks_and_biomes_buf.len()
        ))
    })?)?;
    write.write_slice(&blocks_and_biomes_buf)?;

    Ok(())
}

fn serialize_block_entities(
    write: &mut impl Write,
    block_entities: &FxHashMap<BlockPos, NbtCompound>,
) -> Result<(), WritingError> {
    write.write_var_int(&VarInt(block_entities.len() as i32))?;
    for (pos, nbt) in block_entities {
        let local_xz = ((get_local_cord(pos.0.x) & 0xF) << 4) | (get_local_cord(pos.0.z) & 0xF);

        write.write_u8(local_xz as u8)?;
        write.write_i16_be(pos.0.y as i16)?;

        let id = nbt.get_string("id").map_or(0, |id_str| {
            let name = id_str.split(':').next_back().unwrap_or(id_str);
            pumpkin_data::block_properties::BLOCK_ENTITY_TYPES
                .iter()
                .position(|&n| n == name)
                .unwrap_or(0)
        });

        write.write_var_int(&VarInt(id as i32))?;
        write.write_nbt(nbt.clone().into())?;
    }
    Ok(())
}
