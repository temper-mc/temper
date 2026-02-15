use temper_codec::net_types::length_prefixed_vec::LengthPrefixedVec;
use temper_codec::net_types::var_int::VarInt;
use temper_inventories::slot::InventorySlot;
use temper_macros::{NetEncode, packet};

#[derive(NetEncode)]
#[packet(packet_id = "container_set_content", state = "play")]
pub struct SetContainerContent {
    pub window_id: VarInt,
    pub state_id: VarInt,
    pub slots: LengthPrefixedVec<InventorySlot>,
    pub carried_item: InventorySlot,
}
