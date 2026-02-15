use temper_macros::{NetDecode, packet};

#[derive(NetDecode)]
#[packet(packet_id = "set_carried_item", state = "play")]
pub struct SetHeldItem {
    pub slot_index: i16,
}
