use temper_codec::net_types::var_int::VarInt;
use temper_macros::{packet, NetDecode};

#[derive(NetDecode)]
#[packet(packet_id = "swing", state = "play")]
pub struct SwingArmPacket {
    pub hand: VarInt,
}
