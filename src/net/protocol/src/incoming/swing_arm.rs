use temper_codec::net_types::var_int::VarInt;
use temper_macros::{NetDecode, packet};

#[derive(NetDecode)]
#[packet(packet_id = "swing", state = "play")]
pub struct SwingArmPacket {
    pub hand: VarInt,
}
