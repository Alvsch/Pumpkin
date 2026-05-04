//! The foundational crate for Pumpkin, providing core data structures and types shared across all crates.

/// Current supported Minecraft Java version
pub const CURRENT_MC_VERSION: &str = "26.1";

#[cfg(feature = "bedrock")]
/// Current supported Minecraft Bedrock version string.
pub const CURRENT_BEDROCK_MC_VERSION: &str = "1.26.10";
#[cfg(feature = "bedrock")]
/// Current supported Bedrock protocol version number.
pub const CURRENT_BEDROCK_MC_PROTOCOL: u32 = 944;
