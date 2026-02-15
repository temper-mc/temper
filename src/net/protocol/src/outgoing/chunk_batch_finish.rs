use temper_codec::net_types::var_int::VarInt;
use temper_macros::{NetEncode, packet};

#[derive(NetEncode)]
#[packet(packet_id = "chunk_batch_finished", state = "play")]
pub struct ChunkBatchFinish {
    pub batch_size: VarInt,
}
