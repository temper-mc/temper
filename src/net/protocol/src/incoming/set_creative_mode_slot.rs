use temper_inventories::slot::InventorySlot;
use temper_macros::{NetDecode, packet};

#[derive(NetDecode)]
#[packet(packet_id = "set_creative_mode_slot", state = "play")]
pub struct SetCreativeModeSlot {
    pub slot_index: i16,
    pub slot: InventorySlot,
}
