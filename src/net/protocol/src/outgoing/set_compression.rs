use temper_codec::net_types::var_int::VarInt;
use temper_macros::{NetEncode, packet};

#[derive(NetEncode)]
#[packet(packet_id = "login_compression", state = "login")]
pub struct SetCompressionPacket {
    pub threshold: VarInt,
}
