use temper_codec::net_types::var_int::VarInt;
use temper_macros::{NetDecode, packet};

#[derive(NetDecode)]
#[packet(packet_id = "container_close", state = "play")]
pub struct CloseContainer {
    pub window_id: VarInt,
}
