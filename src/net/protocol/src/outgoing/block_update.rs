use temper_codec::net_types::network_position::NetworkPosition;
use temper_codec::net_types::var_int::VarInt;
use temper_macros::{NetEncode, packet};

#[derive(NetEncode)]
#[packet(packet_id = "block_update", state = "play")]
pub struct BlockUpdate {
    pub location: NetworkPosition,
    pub block_state_id: VarInt,
}
