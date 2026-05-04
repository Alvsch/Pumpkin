use std::sync::Arc;

use barrel::BarrelBlockEntity;
use bed::BedBlockEntity;
use brewing_stand::BrewingStandBlockEntity;
use chest::ChestBlockEntity;
use comparator::ComparatorBlockEntity;
use daylight_detector::DaylightDetectorBlockEntity;
use end_portal::EndPortalBlockEntity;
use furnace::FurnaceBlockEntity;
use piston::PistonBlockEntity;
use pumpkin_data::{Block, block_properties::BLOCK_ENTITY_TYPES};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::math::position::BlockPos;
use pumpkin_world_core::{BlockEntity, MobSpawnerBlockEntity};
use sign::SignBlockEntity;
use trapped_chest::TrappedChestBlockEntity;

use crate::entities::bell::BellBlockEntity;
use crate::entities::blasting_furnace::BlastingFurnaceBlockEntity;
use crate::entities::chiseled_bookshelf::ChiseledBookshelfBlockEntity;
use crate::entities::command_block::CommandBlockEntity;
use crate::entities::dropper::DropperBlockEntity;
use crate::entities::ender_chest::EnderChestBlockEntity;
use crate::entities::hopper::HopperBlockEntity;
use crate::entities::jukebox::JukeboxBlockEntity;
use crate::entities::lectern::LecternBlockEntity;
use crate::entities::shulker_box::ShulkerBoxBlockEntity;
use crate::entities::smoker::SmokerBlockEntity;

pub mod barrel;
pub mod bed;
pub mod bell;
pub mod blasting_furnace;
pub mod brewing_stand;
pub mod chest;
pub mod chest_like_block_entity;
pub mod chiseled_bookshelf;
pub mod command_block;
pub mod comparator;
pub mod daylight_detector;
pub mod dropper;
pub mod end_portal;
pub mod ender_chest;
pub mod furnace;
pub mod furnace_like_block_entity;
pub mod hopper;
pub mod jukebox;
pub mod lectern;
pub mod piston;
pub mod shulker_box;
pub mod sign;
pub mod smoker;
pub mod trapped_chest;

#[must_use]
pub fn block_entity_from_generic<T: BlockEntity>(nbt: &NbtCompound) -> T {
    let x = nbt.get_int("x").unwrap();
    let y = nbt.get_int("y").unwrap();
    let z = nbt.get_int("z").unwrap();
    T::from_nbt(nbt, BlockPos::new(x, y, z))
}

#[must_use]
pub fn block_entity_from_nbt(nbt: &NbtCompound) -> Option<Arc<dyn BlockEntity>> {
    Some(match nbt.get_string("id").unwrap() {
        ChestBlockEntity::ID => Arc::new(block_entity_from_generic::<ChestBlockEntity>(nbt)),
        TrappedChestBlockEntity::ID => {
            Arc::new(block_entity_from_generic::<TrappedChestBlockEntity>(nbt))
        }
        EnderChestBlockEntity::ID => {
            Arc::new(block_entity_from_generic::<EnderChestBlockEntity>(nbt))
        }
        JukeboxBlockEntity::ID => Arc::new(block_entity_from_generic::<JukeboxBlockEntity>(nbt)),
        SignBlockEntity::ID => Arc::new(block_entity_from_generic::<SignBlockEntity>(nbt)),
        BedBlockEntity::ID => Arc::new(block_entity_from_generic::<BedBlockEntity>(nbt)),
        ComparatorBlockEntity::ID => {
            Arc::new(block_entity_from_generic::<ComparatorBlockEntity>(nbt))
        }
        BarrelBlockEntity::ID => Arc::new(block_entity_from_generic::<BarrelBlockEntity>(nbt)),
        HopperBlockEntity::ID => Arc::new(block_entity_from_generic::<HopperBlockEntity>(nbt)),
        MobSpawnerBlockEntity::ID => {
            Arc::new(block_entity_from_generic::<MobSpawnerBlockEntity>(nbt))
        }
        DropperBlockEntity::ID => Arc::new(block_entity_from_generic::<DropperBlockEntity>(nbt)),
        ShulkerBoxBlockEntity::ID => {
            Arc::new(block_entity_from_generic::<ShulkerBoxBlockEntity>(nbt))
        }
        PistonBlockEntity::ID => Arc::new(block_entity_from_generic::<PistonBlockEntity>(nbt)),
        EndPortalBlockEntity::ID => {
            Arc::new(block_entity_from_generic::<EndPortalBlockEntity>(nbt))
        }
        ChiseledBookshelfBlockEntity::ID => Arc::new(block_entity_from_generic::<
            ChiseledBookshelfBlockEntity,
        >(nbt)),
        FurnaceBlockEntity::ID => Arc::new(block_entity_from_generic::<FurnaceBlockEntity>(nbt)),
        CommandBlockEntity::ID => Arc::new(block_entity_from_generic::<CommandBlockEntity>(nbt)),
        BlastingFurnaceBlockEntity::ID => {
            Arc::new(block_entity_from_generic::<BlastingFurnaceBlockEntity>(nbt))
        }
        SmokerBlockEntity::ID => Arc::new(block_entity_from_generic::<SmokerBlockEntity>(nbt)),
        DaylightDetectorBlockEntity::ID => Arc::new(block_entity_from_generic::<
            DaylightDetectorBlockEntity,
        >(nbt)),
        BrewingStandBlockEntity::ID => {
            Arc::new(block_entity_from_generic::<BrewingStandBlockEntity>(nbt))
        }
        BellBlockEntity::ID => Arc::new(block_entity_from_generic::<BellBlockEntity>(nbt)),
        LecternBlockEntity::ID => Arc::new(block_entity_from_generic::<LecternBlockEntity>(nbt)),
        _ => return None,
    })
}

#[must_use]
pub fn has_block_block_entity(block: &Block) -> bool {
    BLOCK_ENTITY_TYPES.contains(&block.name)
}
