use temper_macros::{NetEncode, packet};

#[derive(NetEncode)]
#[packet(packet_id = "chunk_batch_start", state = "play")]
pub struct ChunkBatchStart {}
