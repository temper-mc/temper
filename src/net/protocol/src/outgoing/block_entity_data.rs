use temper_codec::net_types::network_position::NetworkPosition;
use temper_codec::net_types::var_int::VarInt;
use temper_macros::{NetEncode, packet};

#[derive(NetEncode)]
#[packet(packet_id = "block_entity_data", state = "play")]
pub struct BlockEntityDataPacket {
    pub location: NetworkPosition,
    pub entity_type: VarInt,
    pub nbt: Vec<u8>,
}
