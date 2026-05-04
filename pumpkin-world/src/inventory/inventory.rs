use pumpkin_data::item::Item;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_nbt::{compound::NbtCompound, tag::NbtTag};
use std::any::Any;
use std::pin::Pin;
use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};
use tokio::sync::{Mutex, OwnedMutexGuard};

pub struct ComparableInventory(pub Arc<dyn Inventory>);

impl PartialEq for ComparableInventory {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for ComparableInventory {}

impl Hash for ComparableInventory {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let ptr = Arc::as_ptr(&self.0);
        ptr.hash(state);
    }
}
