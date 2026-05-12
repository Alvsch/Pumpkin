/// Represents the different types of data encoding used in Minecraft's bit-packed chunk sections.
///
/// Minecraft uses a "Palette" system to compress chunk data. Instead of sending a full
/// 15-bit `BlockState` ID for every block, it sends a smaller index (e.g., 4 bits) that
/// points to a value in these palettes.
pub enum NetworkPalette<V> {
    /// **Single Value Palette (Bits per entry: 0)**
    ///
    /// Used when an entire chunk section (16x16x16) consists of only one type of block or biome.
    /// No data array follows this palette in the network buffer.
    Single(V),
    /// **Indirect Palette (Bits per entry: 1-8 for blocks, 1-3 for biomes)**
    ///
    /// A list of unique values present in the section. The data array contains indices
    /// pointing into this list.
    Indirect(Box<[V]>),
    /// **Direct Palette (Bits per entry: 15+ for blocks, 6+ for biomes)**
    ///
    /// Used when the section is too complex for a small palette. The data array
    /// contains global Registry IDs directly. No palette list is sent.
    Direct,
}

pub struct NetworkSerialization<V> {
    pub bits_per_entry: u8,
    pub palette: NetworkPalette<V>,
    pub packed_data: Box<[i64]>,
}

pub struct BeNetworkSerialization<V> {
    pub bits_per_entry: u8,
    pub palette: NetworkPalette<V>,
    pub packed_data: Box<[u32]>,
}
