use crate::WritingError;
use crate::codec::bit_set::BitSet;
use crate::{ClientPacket, VarInt, ser::NetworkWriteExt};
use pumpkin_chunk::{LightContainer, LightData};
use pumpkin_data::packet::clientbound::PLAY_LIGHT_UPDATE;
use pumpkin_macros::java_packet;
use pumpkin_util::version::MinecraftVersion;
use std::io::Write;

/// Sent by the server to update light levels (block light and sky light) for a chunk.
///
/// This packet updates lighting data for a specific chunk without sending the full chunk data.
/// It's used when block placement or removal changes the lighting in a chunk.
#[java_packet(PLAY_LIGHT_UPDATE)]
pub struct CLightUpdate {
    pub x: i32,
    pub z: i32,
    pub light_data: LightData,
}

impl ClientPacket for CLightUpdate {
    fn write_packet_data(
        &self,
        mut write: impl Write,
        _version: &MinecraftVersion,
    ) -> Result<(), WritingError> {
        // Chunk X
        write.write_var_int(&VarInt(self.x))?;
        // Chunk Z
        write.write_var_int(&VarInt(self.z))?;

        serialize_light(&mut write, &self.light_data)?;
        Ok(())
    }
}

pub fn serialize_light(
    write: &mut impl Write,
    light_engine: &LightData,
) -> Result<(), WritingError> {
    let num_sections = light_engine.sky_light.len();

    // Light masks include sections from -1 (below world) to num_sections (above world)
    // This means we need to account for 2 extra sections in the bitset
    let mut sky_light_empty_mask = 0u64;
    let mut block_light_empty_mask = 0u64;
    let mut sky_light_mask = 0u64;
    let mut block_light_mask = 0u64;

    // Bit 0 represents the section below the world (always empty)
    sky_light_empty_mask |= 1 << 0;
    block_light_empty_mask |= 1 << 0;

    // Bits 1..=num_sections represent the actual world sections
    for section_index in 0..num_sections {
        let bit_index = section_index + 1; // Offset by 1 for the below-world section

        if let LightContainer::Full(_) = &light_engine.sky_light[section_index] {
            sky_light_mask |= 1 << bit_index;
        } else {
            sky_light_empty_mask |= 1 << bit_index;
        }

        if let LightContainer::Full(_) = &light_engine.block_light[section_index] {
            block_light_mask |= 1 << bit_index;
        } else {
            block_light_empty_mask |= 1 << bit_index;
        }
    }

    // Bit num_sections+1 represents the section above the world (always empty)
    sky_light_empty_mask |= 1 << (num_sections + 1);
    block_light_empty_mask |= 1 << (num_sections + 1);

    // Write Sky Light Mask
    write.write_bitset(&BitSet(Box::new([sky_light_mask as i64])))?;
    // Write Block Light Mask
    write.write_bitset(&BitSet(Box::new([block_light_mask as i64])))?;
    // Write Empty Sky Light Mask
    write.write_bitset(&BitSet(Box::new([sky_light_empty_mask as i64])))?;
    // Write Empty Block Light Mask
    write.write_bitset(&BitSet(Box::new([block_light_empty_mask as i64])))?;

    let light_data_size: VarInt = VarInt(LightContainer::ARRAY_SIZE as i32);

    // Write Sky Light arrays
    write.write_var_int(&VarInt(sky_light_mask.count_ones() as i32))?;
    for section_index in 0..num_sections {
        if let LightContainer::Full(data) = &light_engine.sky_light[section_index] {
            write.write_var_int(&light_data_size)?;
            write.write_slice(data.as_ref())?;
        }
    }

    // Write Block Light arrays
    write.write_var_int(&VarInt(block_light_mask.count_ones() as i32))?;
    for section_index in 0..num_sections {
        if let LightContainer::Full(data) = &light_engine.block_light[section_index] {
            write.write_var_int(&light_data_size)?;
            write.write_slice(data.as_ref())?;
        }
    }
    Ok(())
}
