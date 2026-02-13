use temper_inventories::slot::InventorySlot;
use temper_codec::net_types::var_int::VarInt;
use temper_macros::{packet, NetEncode};

#[derive(NetEncode)]
#[packet(packet_id = "set_player_inventory", state = "play")]
/// # This packet is buggy and does not seem to work.
pub struct SetPlayerInventorySlot {
    pub slot_index: VarInt,
    pub slot: InventorySlot,
}
