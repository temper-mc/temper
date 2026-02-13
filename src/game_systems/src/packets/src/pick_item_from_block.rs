use bevy_ecs::prelude::{Entity, Query, Res};
use temper_codec::net_types::var_int::VarInt;
use temper_components::player::abilities::PlayerAbilities;
use temper_components::player::player_identity::PlayerIdentity;
use temper_inventories::item::ItemID;
use temper_inventories::slot::InventorySlot;
use temper_inventories::{hotbar::Hotbar, inventory::Inventory};
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::outgoing::set_held_slot::SetHeldItem;
use temper_state::GlobalStateResource;

use temper_components::player::position::Position;
use temper_core::pos::{ChunkBlockPos, ChunkPos};
use temper_protocol::PickItemFromBlockReceiver;
use tracing::{debug, error, warn};

pub fn handle(
    events: Res<PickItemFromBlockReceiver>, // Packet queue
    state: Res<GlobalStateResource>,
    mut player_inv_query: Query<(
        Entity,
        &PlayerIdentity,
        &PlayerAbilities,
        &mut Inventory,
        &mut Hotbar,
        &StreamWriter,
    )>,
) {
    for (packet, sender_entity) in events.0.try_iter() {
        // 1. Get player's components
        let (entity, identity, abilities, mut inventory, mut hotbar, writer) =
            match player_inv_query.get_mut(sender_entity) {
                Ok(data) => data,
                Err(e) => {
                    panic!(
                        "PickItemFromBlock: Recieved packet from entity {:?} without components {:?}",
                        sender_entity, e
                    );
                }
            };

        debug!(
            "Player {} requested pick block at {:?} (Include Data: {})",
            identity.username, packet.location, packet.include_data,
        );

        // 2. Get block from world
        let pos = Position::from(packet.location);
        let chunk = match state
            .0
            .world
            .get_or_generate_chunk(ChunkPos::from(pos.coords), "overworld")
        {
            Ok(chunk) => chunk,
            Err(e) => {
                warn!(
                    "PickItemFromBlock: Failed to get chunk for position {:?}: {:?}",
                    pos, e
                );
                continue;
            }
        };
        let block_state_id = chunk.get_block(ChunkBlockPos::new(
            pos.coords.x as u8,
            pos.coords.y as i16,
            pos.coords.z as u8,
        ));

        // 3. Convert `BlockStateId` to `ItemId`
        let item_id = match ItemID::from_block_state(block_state_id) {
            Some(id) => id,
            None => {
                debug!(
                    "PickItemFromBlock: No item for block state {:?}",
                    block_state_id
                );
                continue; // No item for this block (e.g., air)
            }
        };

        debug!(
            "PickItemFromBlock: Block corresponds to ItemID: {:?}",
            item_id
        );

        // 4. Search the inventory for `ItemID`
        let found_slot_index = inventory.find_item(item_id);

        // 5a. Search hotbar
        if let Some(hotbar_slot) = hotbar.find_item(&inventory, item_id) {
            // Item is in the hotbar. Check if we're already holding it.
            if hotbar.selected_slot == hotbar_slot {
                continue; // Do nothing
            }

            debug!(
                "Item found in hotbar slot {}. Switching held item.",
                hotbar_slot
            );

            // 1. Update the server's state
            hotbar.selected_slot = hotbar_slot;

            // 2. Send the packet to sync the client
            let packet = SetHeldItem { slot: hotbar_slot };
            if let Err(e) = writer.send_packet_ref(&packet) {
                error!("Failed to send SetHeldItem packet: {:?}", e);
            }
        }
        // 5b. Search rest of inventory
        else if let Some(inventory_slot_index) = found_slot_index {
            debug!(
                "Found item in slot {}. Swapping with hotbar slot {}.",
                inventory_slot_index, hotbar.selected_slot
            );

            // Check if the item is already in the selected hotbar slot.
            if inventory_slot_index == hotbar.get_selected_inventory_index() {
                continue; // Nothing to do
            }

            if let Err(e) =
                hotbar.swap_with_inventory_slot(&mut inventory, inventory_slot_index, entity)
            {
                warn!("Failed to swap slots: {:?}", e);
            }
        }
        // 6. If not found AND in creative mode
        else if abilities.creative_mode {
            // TODO: Possible bug with using creative_mode ability instead of creative Gamemode
            debug!("Item not found. Creating stack for creative player.");

            let new_slot = InventorySlot {
                item_id: Some(item_id),
                count: VarInt::new(1),
                ..Default::default()
            };

            // TODO: Handle NBT data
            if packet.include_data {
                warn!("PickBlock: NBT data request (include_data=true is not implemented yet.");
            }

            if let Some(new_index) = hotbar.get_lowest_open_slot(&inventory) {
                if let Err(e) =
                    hotbar.set_item_with_update(&mut inventory, new_index, new_slot, entity)
                {
                    warn!("Failed to set creative item in hotbar: {:?}", e);
                } else {
                    let packet = SetHeldItem { slot: new_index };
                    if let Err(e) = writer.send_packet_ref(&packet) {
                        error!("Failed to send SetHeldItem packet: {:?}", e);
                    }
                }
            }
        }
        // 7. If not found AND survival...
        else {
            debug!("Item not found in inventory and player is in survival. Doing nothing.")
            // No-op
        }
    }
}
