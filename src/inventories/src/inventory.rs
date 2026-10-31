use crate::errors::InventoryError;
use crate::item::ItemID;
use crate::slot::InventorySlot;
use crate::{INVENTORY_UPDATES_QUEUE, InventoryUpdate};
use bevy_ecs::prelude::{Component, Entity};
use serde::{Deserialize, Serialize};
use type_hash::TypeHash;

/// Represents an inventory, which is a collection of slots that can hold items.
///
/// To avoid TOC/TOU bugs, if you need to fetch an inventory slot and modify it between systems,
/// get the slot's unique_id and use `get_item_by_unique_id` to fetch it again before modifying it.
#[derive(Component, Clone, Debug, Serialize, Deserialize, TypeHash)]
pub struct Inventory {
    pub slots: Box<[Option<InventorySlot>]>,
}

impl Default for Inventory {
    /// Make default inventory, sized for a PLAYER.
    /// 46 = (5 * 9) + 1 =
    /// NOT divisible by 9.
    fn default() -> Self {
        Self::new(Self::DEFAULT_PLAYER_SIZE)
    }
}

impl Inventory {
    pub const DEFAULT_PLAYER_SIZE: usize = 46;

    pub fn new(size: usize) -> Self {
        Self {
            slots: vec![None; size].into_boxed_slice(),
        }
    }

    pub fn clear(&mut self) {
        for slot in &mut self.slots {
            *slot = None;
        }
    }

    pub fn contains_item(&self, item_id: i32) -> bool {
        self.slots.iter().any(|slot| {
            if let Some(slot) = slot {
                if let Some(item) = &slot.item_id {
                    item.0.0 == item_id
                } else {
                    false
                }
            } else {
                false
            }
        })
    }

    pub fn add_item(&mut self, item: InventorySlot) -> Result<(), InventoryError> {
        for slot in self.slots.iter_mut() {
            if slot.is_none() {
                *slot = Some(item);
                return Ok(());
            }
        }
        Err(InventoryError::InventoryFull)
    }

    pub fn add_item_with_update(
        &mut self,
        item: InventorySlot,
        entity: Entity,
    ) -> Result<(), InventoryError> {
        for (index, slot) in self.slots.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(item.clone());
                INVENTORY_UPDATES_QUEUE.push(InventoryUpdate {
                    slot_index: index as u8,
                    slot: item,
                    entity,
                });
                return Ok(());
            }
        }
        Err(InventoryError::InventoryFull)
    }

    pub fn set_item_by_index(
        &mut self,
        index: usize,
        item: InventorySlot,
    ) -> Result<(), InventoryError> {
        if index >= self.slots.len() {
            return Err(InventoryError::InvalidSlotIndex(index));
        }
        self.slots[index] = Some(item);
        Ok(())
    }

    pub fn set_item_by_index_with_update(
        &mut self,
        index: usize,
        item: InventorySlot,
        entity: Entity,
    ) -> Result<(), InventoryError> {
        if index >= self.slots.len() {
            return Err(InventoryError::InvalidSlotIndex(index));
        }
        self.slots[index] = Some(item.clone());
        INVENTORY_UPDATES_QUEUE.push(InventoryUpdate {
            slot_index: index as u8,
            slot: item,
            entity,
        });
        Ok(())
    }

    pub fn get_item_by_index(
        &self,
        index: usize,
    ) -> Result<Option<&InventorySlot>, InventoryError> {
        if index >= self.slots.len() {
            return Err(InventoryError::InvalidSlotIndex(index));
        }
        Ok(self.slots[index].as_ref())
    }

    pub fn remove_item_by_index(&mut self, index: usize) -> Result<(), InventoryError> {
        if index >= self.slots.len() {
            return Err(InventoryError::InvalidSlotIndex(index));
        }
        if self.slots[index].is_none() {
            return Err(InventoryError::ItemNotFound);
        }
        self.slots[index] = None;
        Ok(())
    }

    pub fn remove_item_by_index_with_update(
        &mut self,
        index: usize,
        entity: Entity,
    ) -> Result<(), InventoryError> {
        if index >= self.slots.len() {
            return Err(InventoryError::InvalidSlotIndex(index));
        }
        if self.slots[index].is_none() {
            return Err(InventoryError::ItemNotFound);
        }
        self.slots[index] = None;
        INVENTORY_UPDATES_QUEUE.push(InventoryUpdate {
            slot_index: index as u8,
            slot: InventorySlot::default(),
            entity,
        });
        Ok(())
    }

    /// Clears an inventory slot, regardless of its current state, and sends an update.
    /// This is idempotent and will not error if the slot is already empty.
    pub fn clear_slot_with_update(
        &mut self,
        index: usize,
        entity: Entity,
    ) -> Result<(), InventoryError> {
        if index >= self.slots.len() {
            return Err(InventoryError::InvalidSlotIndex(index));
        }

        // If the slot is already empty, we don't need to do anything
        // except send the update (which is good practice).
        if self.slots[index].is_none() {
            // Fall through to send the update
        }

        // Set the server's state to empty
        self.slots[index] = None;

        // Queue the update to tell the client the slot is now empty
        INVENTORY_UPDATES_QUEUE.push(InventoryUpdate {
            slot_index: index as u8,
            slot: InventorySlot::default(), // An empty slot (count: 0)
            entity,
        });
        Ok(())
    }

    /// Searches the inventory for the first slot containing the given ItemID.
    ///
    /// Returns `Some(index)` if found, `None` otherwise.
    pub fn find_item(&self, item_id: ItemID) -> Option<usize> {
        self.slots.iter().position(|slot| match slot {
            Some(inventory_slot) => inventory_slot.item_id == Some(item_id),
            None => false,
        })
    }

    /// Swaps the contents of two slots and sends updates to the client.
    pub fn swap_slots_with_update(
        &mut self,
        index_a: usize,
        index_b: usize,
        entity: Entity,
    ) -> Result<(), InventoryError> {
        if index_a >= self.slots.len() {
            return Err(InventoryError::InvalidSlotIndex(index_a));
        }
        if index_b >= self.slots.len() {
            return Err(InventoryError::InvalidSlotIndex(index_b));
        }
        if index_a == index_b {
            return Ok(()); // Nothing to do
        }

        // Swap the slots in the server's memory
        self.slots.swap(index_a, index_b);

        // Send an update for the first slot
        INVENTORY_UPDATES_QUEUE.push(InventoryUpdate {
            slot_index: index_a as u8,
            // Clone the data that is now in slot A
            slot: self.slots[index_a].clone().unwrap_or_default(),
            entity,
        });

        // Send an update for the second slot
        INVENTORY_UPDATES_QUEUE.push(InventoryUpdate {
            slot_index: index_b as u8,
            // Clone the data that is now in slot B
            slot: self.slots[index_b].clone().unwrap_or_default(),
            entity,
        });

        Ok(())
    }

    /// Sets an item's data by its unique_id, if it exists in the inventory.
    /// The replacing item's unique ID is discarded and the original unique ID is preserved.
    pub fn set_item_by_unique_id(
        &mut self,
        unique_id: u64,
        item: InventorySlot,
    ) -> Result<(), InventoryError> {
        let Some(index) = self.get_index_by_unique_id(unique_id) else {
            return Err(InventoryError::UniqueIdNotFound(unique_id));
        };
        let Some(slot) = self.slots.get_mut(index) else {
            return Err(InventoryError::InvalidSlotIndex(index));
        };
        let Some(slot) = slot else {
            return Err(InventoryError::ItemNotFound);
        };
        // This isn't great I know, but it keeps the unique_id an opaque value that can't be changed
        // by the caller, while still allowing them to change the rest of the slot's data.
        slot.count = item.count;
        slot.item_id = item.item_id;
        slot.components_to_remove = item.components_to_remove;
        slot.components_to_add = item.components_to_add;
        Ok(())
    }

    /// Sets an item's data by its unique_id, if it exists in the inventory, and sends an update to the client.
    /// The replacing item's unique ID is discarded and the original unique ID is preserved.
    pub fn set_item_by_unique_id_with_update(
        &mut self,
        unique_id: u64,
        item: InventorySlot,
        entity: Entity,
    ) -> Result<(), InventoryError> {
        let Some(index) = self.get_index_by_unique_id(unique_id) else {
            return Err(InventoryError::UniqueIdNotFound(unique_id));
        };
        if self.slots.get(index).is_none() {
            return Err(InventoryError::InvalidSlotIndex(index));
        }
        let Some(slot) = self.slots.get_mut(index) else {
            return Err(InventoryError::InvalidSlotIndex(index));
        };
        let Some(slot) = slot else {
            return Err(InventoryError::ItemNotFound);
        };
        slot.count = item.count;
        slot.item_id = item.item_id;
        slot.components_to_remove = item.components_to_remove;
        slot.components_to_add = item.components_to_add;
        INVENTORY_UPDATES_QUEUE.push(InventoryUpdate {
            slot_index: index as u8,
            slot: slot.clone(),
            entity,
        });
        Ok(())
    }

    pub fn get_item_by_unique_id(&self, unique_id: u64) -> Option<&InventorySlot> {
        self.slots.iter().find_map(|slot| {
            slot.as_ref()
                .filter(|slot| slot.get_unique_id() == unique_id)
        })
    }

    pub fn remove_item_by_unique_id(&mut self, unique_id: u64) -> Result<(), InventoryError> {
        let Some(index) = self.get_index_by_unique_id(unique_id) else {
            return Err(InventoryError::UniqueIdNotFound(unique_id));
        };
        if self.slots.get(index).is_none() {
            return Err(InventoryError::InvalidSlotIndex(index));
        }
        self.slots[index] = None;
        Ok(())
    }

    pub fn remove_item_by_unique_id_with_update(
        &mut self,
        unique_id: u64,
        entity: Entity,
    ) -> Result<(), InventoryError> {
        let Some(index) = self.get_index_by_unique_id(unique_id) else {
            return Err(InventoryError::UniqueIdNotFound(unique_id));
        };
        if self.slots.get(index).is_none() {
            return Err(InventoryError::InvalidSlotIndex(index));
        }
        self.slots[index] = None;
        INVENTORY_UPDATES_QUEUE.push(InventoryUpdate {
            slot_index: index as u8,
            slot: InventorySlot::default(),
            entity,
        });
        Ok(())
    }

    fn get_index_by_unique_id(&self, unique_id: u64) -> Option<usize> {
        self.slots.iter().position(|slot| {
            if let Some(slot) = slot {
                slot.get_unique_id() == unique_id
            } else {
                false
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::ItemID;
    use temper_codec::net_types::var_int::VarInt;

    fn make_slot_with_id(id: i32) -> InventorySlot {
        InventorySlot::builder()
            .count(VarInt::from(1))
            .item_id(ItemID::new(id))
            .build()
    }

    #[test]
    fn test_new_inventory() {
        let inv = Inventory::new(5);
        assert_eq!(inv.slots.len(), 5);
        assert!(inv.slots.iter().all(|s| s.is_none()));
    }

    #[test]
    fn test_add_and_get_item() {
        let mut inv = Inventory::new(2);
        let slot = make_slot_with_id(1);
        assert!(inv.add_item(slot.clone()).is_ok());
        assert!(inv.get_item_by_index(0).unwrap().is_some());
        assert!(inv.get_item_by_index(1).unwrap().is_none());
    }

    #[test]
    fn test_add_item_full() {
        let mut inv = Inventory::new(1);
        let slot = make_slot_with_id(1);
        inv.add_item(slot).unwrap();
        let slot2 = make_slot_with_id(2);
        assert!(matches!(
            inv.add_item(slot2),
            Err(InventoryError::InventoryFull)
        ));
    }

    #[test]
    fn test_set_and_remove_item() {
        let mut inv = Inventory::new(1);
        let slot = make_slot_with_id(1);
        inv.set_item_by_index(0, slot).unwrap();
        assert!(inv.get_item_by_index(0).unwrap().is_some());
        inv.remove_item_by_index(0).unwrap();
        assert!(inv.get_item_by_index(0).unwrap().is_none());
    }

    #[test]
    fn test_contains_item() {
        let mut inv = Inventory::new(2);
        let slot = make_slot_with_id(42);
        inv.add_item(slot).unwrap();
        assert!(inv.contains_item(42));
        assert!(!inv.contains_item(99));
    }

    #[test]
    fn test_clear() {
        let mut inv = Inventory::new(2);
        inv.set_item_by_index(0, make_slot_with_id(1)).unwrap();
        inv.set_item_by_index(1, make_slot_with_id(2)).unwrap();
        inv.clear();
        assert!(inv.slots.iter().all(|s| s.is_none()));
    }

    #[test]
    fn test_invalid_index() {
        let mut inv = Inventory::new(1);
        assert!(matches!(
            inv.get_item_by_index(2),
            Err(InventoryError::InvalidSlotIndex(2))
        ));
        assert!(matches!(
            inv.set_item_by_index(2, make_slot_with_id(1)),
            Err(InventoryError::InvalidSlotIndex(2))
        ));
        assert!(matches!(
            inv.remove_item_by_index(2),
            Err(InventoryError::InvalidSlotIndex(2))
        ));
    }

    #[test]
    fn test_get_item_by_unique_id() {
        let mut inv = Inventory::new(1);
        assert!(inv.get_item_by_unique_id(123).is_none());

        let slot = make_slot_with_id(1);
        let unique_id = slot.get_unique_id();
        inv.add_item(slot).unwrap();

        assert_eq!(
            inv.get_item_by_unique_id(unique_id)
                .and_then(|slot| slot.item_id),
            Some(ItemID::new(1))
        );
        assert!(inv.get_item_by_unique_id(123).is_none());
    }

    #[test]
    fn test_set_item_by_unique_id() {
        let mut inv = Inventory::new(1);
        let original = make_slot_with_id(1);
        let unique_id = original.get_unique_id();
        inv.add_item(original).unwrap();

        inv.set_item_by_unique_id(unique_id, make_slot_with_id(2))
            .unwrap();
        assert_eq!(
            inv.get_item_by_unique_id(unique_id)
                .and_then(|slot| slot.item_id),
            Some(ItemID::new(2))
        );
        assert!(matches!(
            inv.set_item_by_unique_id(123, make_slot_with_id(3)),
            Err(InventoryError::UniqueIdNotFound(123))
        ));
    }

    #[test]
    fn test_set_item_by_unique_id_with_update() {
        let mut inv = Inventory::new(1);
        let original = make_slot_with_id(1);
        let unique_id = original.get_unique_id();
        inv.add_item(original).unwrap();

        inv.set_item_by_unique_id_with_update(unique_id, make_slot_with_id(2), Entity::PLACEHOLDER)
            .unwrap();
        assert_eq!(
            inv.get_item_by_unique_id(unique_id)
                .and_then(|slot| slot.item_id),
            Some(ItemID::new(2))
        );
    }

    #[test]
    fn test_remove_item_by_unique_id() {
        let mut inv = Inventory::new(1);
        let slot = make_slot_with_id(1);
        let unique_id = slot.get_unique_id();
        inv.add_item(slot).unwrap();

        inv.remove_item_by_unique_id(unique_id).unwrap();
        assert!(inv.get_item_by_unique_id(unique_id).is_none());
        assert!(matches!(
            inv.remove_item_by_unique_id(unique_id),
            Err(InventoryError::UniqueIdNotFound(id)) if id == unique_id
        ));
    }

    #[test]
    fn test_remove_item_by_unique_id_with_update() {
        let mut inv = Inventory::new(1);
        let slot = make_slot_with_id(1);
        let unique_id = slot.get_unique_id();
        inv.add_item(slot).unwrap();

        inv.remove_item_by_unique_id_with_update(unique_id, Entity::PLACEHOLDER)
            .unwrap();
        assert!(inv.get_item_by_unique_id(unique_id).is_none());
    }
}
