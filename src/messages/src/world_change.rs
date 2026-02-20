use bevy_ecs::prelude::Message;
use temper_core::pos::ChunkPos;

#[derive(Message)]
pub struct WorldChange {
    pub chunk: Option<ChunkPos>,
}
