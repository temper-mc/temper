use temper_codec::net_types::var_int::VarInt;
use temper_macros::{NetEncode, packet};

#[derive(NetEncode)]
#[packet(packet_id = "block_changed_ack", state = "play")]
pub struct BlockChangeAck {
    pub sequence: VarInt,
}
