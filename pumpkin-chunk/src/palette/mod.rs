mod heterogeneous_palette_data;
mod paletted_container;

// According to the wiki, palette serialization for disk and network is different. Disk
// serialization always uses a palette if greater than one entry. Network serialization packs ids
// directly instead of using a palette above a certain bits-per-entry

// TODO: Do our own testing; do we really need to handle network and disk serialization differently?
mod biome_palette;
mod block_palette;
pub mod network;

pub use biome_palette::{BIOME_NETWORK_MAX_BITS, BiomePalette, SectionBiomes};
pub use block_palette::{BLOCK_NETWORK_MAX_BITS, BlockPalette, SectionBlockStates};

/// 3d array indexed by y,z,x
type AbstractCube<T, const DIM: usize> = [[[T; DIM]; DIM]; DIM];
