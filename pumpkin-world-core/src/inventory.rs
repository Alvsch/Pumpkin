use std::{any::Any, pin::Pin, sync::Arc};

use pumpkin_data::{item::Item, item_stack::ItemStack};
use pumpkin_nbt::{compound::NbtCompound, tag::NbtTag};
use tokio::sync::{Mutex, OwnedMutexGuard};

pub type InventoryFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

// NOTE: we have pumpkin-inventory, why is this defined here??
pub trait Inventory: Send + Sync + Clearable {
    fn size(&self) -> usize;

    fn is_empty(&self) -> InventoryFuture<'_, bool>;

    fn get_stack(&self, slot: usize) -> InventoryFuture<'_, Arc<Mutex<ItemStack>>>;

    fn remove_stack(&self, slot: usize) -> InventoryFuture<'_, ItemStack>;

    fn remove_stack_specific(&self, slot: usize, amount: u8) -> InventoryFuture<'_, ItemStack>;

    fn set_stack(&self, slot: usize, stack: ItemStack) -> InventoryFuture<'_, ()>;

    fn on_open(&self) -> InventoryFuture<'_, ()> {
        Box::pin(async {})
    }
    fn on_close(&self) -> InventoryFuture<'_, ()> {
        Box::pin(async {})
    }

    fn count<'a>(&'a self, item: &'a Item) -> InventoryFuture<'a, u8> {
        Box::pin(async move {
            let mut count = 0;

            for i in 0..self.size() {
                let slot = self.get_stack(i).await;
                let stack = slot.lock().await;
                if stack.get_item().id == item.id {
                    count += stack.item_count;
                }
            }

            count
        })
    }

    fn contains_any_predicate<'a>(
        &'a self,
        predicate: &'a (dyn Fn(OwnedMutexGuard<ItemStack>) -> bool + Sync),
    ) -> InventoryFuture<'a, bool> {
        Box::pin(async move {
            for i in 0..self.size() {
                let slot = self.get_stack(i).await;
                let stack = slot.lock_owned().await;
                if predicate(stack) {
                    return true;
                }
            }

            false
        })
    }

    fn contains_any<'a>(&'a self, items: &'a [Item]) -> InventoryFuture<'a, bool> {
        Box::pin(async move {
            self.contains_any_predicate(&|stack| {
                !stack.is_empty() && items.contains(stack.get_item())
            })
            .await
        })
    }

    fn write_inventory_nbt<'a>(
        &'a self,
        nbt: &'a mut NbtCompound,
        include_empty: bool,
    ) -> InventoryFuture<'a, ()> {
        Box::pin(async move {
            let mut slots = Vec::new();
            let size = self.size();

            for i in 0..size {
                let stack_lock = self.get_stack(i).await;
                let stack = stack_lock.lock().await;

                if !stack.is_empty() {
                    let mut item_compound = NbtCompound::new();
                    item_compound.put_byte("Slot", i as i8);
                    stack.write_item_stack(&mut item_compound);
                    slots.push(NbtTag::Compound(item_compound));
                }
            }

            if include_empty || !slots.is_empty() {
                nbt.put("Items", NbtTag::List(slots));
            }
        })
    }

    fn get_max_count_per_stack(&self) -> u8 {
        99
    }

    fn mark_dirty(&self) {}

    fn read_data(&self, nbt: &NbtCompound, stacks: &[Arc<Mutex<ItemStack>>]) {
        if let Some(inventory_list) = nbt.get_list("Items") {
            for tag in inventory_list {
                if let Some(item_compound) = tag.extract_compound()
                    && let Some(slot_byte) = item_compound.get_byte("Slot")
                {
                    let slot = slot_byte as usize;
                    if slot < stacks.len()
                        && let Some(item_stack) = ItemStack::read_item_stack(item_compound)
                    {
                        *stacks[slot].try_lock().unwrap() = item_stack;
                    }
                }
            }
        }
    }

    fn is_valid_slot_for(&self, _slot: usize, _stack: &ItemStack) -> bool {
        true
    }

    fn can_transfer_to(
        &self,
        _hopper_inventory: &dyn Inventory,
        _slot: usize,
        _stack: &ItemStack,
    ) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any;
}

pub trait Clearable {
    fn clear(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}

// These are some utility functions found in Inventories.java
pub async fn split_stack(stacks: &[Arc<Mutex<ItemStack>>], slot: usize, amount: u8) -> ItemStack {
    let mut stack = stacks[slot].lock().await;
    if slot < stacks.len() && !stack.is_empty() && amount > 0 {
        stack.split(amount)
    } else {
        ItemStack::EMPTY.clone()
    }
}
