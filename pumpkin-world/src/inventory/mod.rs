use std::sync::Arc;

use tokio::sync::Mutex;

use pumpkin_data::item_stack::ItemStack;

#[expect(clippy::module_inception)]
mod inventory;
mod simple_inventory;

pub use inventory::*;
pub use simple_inventory::*;
