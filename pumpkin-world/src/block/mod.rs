pub mod entities;
pub mod state;

use std::collections::HashMap;

use pumpkin_data::{Block, BlockState};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
pub use state::RawBlockState;

use crate::BlockStateId;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct BlockStateCodec {
    /// Block name
    #[serde(
        deserialize_with = "parse_block_name",
        serialize_with = "block_to_string"
    )]
    pub name: &'static Block,
    /// Key-value pairs of properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, String>>,
}

fn parse_block_name<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<&'static Block, D::Error> {
    let s = String::deserialize(deserializer)?;
    let block =
        Block::from_name(s.as_str()).ok_or(serde::de::Error::custom("Invalid block name"))?;
    Ok(block)
}

fn block_to_string<S: Serializer>(block: &'static Block, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(block.name)
}

impl BlockStateCodec {
    pub fn get_state(&self) -> &'static BlockState {
        let state_id = self.get_state_id();
        BlockState::from_id(state_id)
    }

    /// Prefer this over `get_state` when the only the state ID is needed
    pub fn get_state_id(&self) -> BlockStateId {
        let block = self.name;

        let mut state_id = block.default_state.id;

        if let Some(properties) = &self.properties {
            let props: Vec<(&str, &str)> = properties
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            let block_properties = block.from_properties(&props);
            state_id = block_properties.to_state_id(block);
        }
        state_id
    }
}

#[cfg(test)]
mod test {
    use pumpkin_data::Block;

    use crate::chunk::palette::BLOCK_NETWORK_MAX_BITS;

    #[test]
    fn test_proper_network_bits_per_entry() {
        let id_to_test = 1 << BLOCK_NETWORK_MAX_BITS;
        if Block::from_state_id(id_to_test) != &Block::AIR {
            panic!("We need to update our constants!");
        }
    }
}
