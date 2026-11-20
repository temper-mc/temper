use bevy_ecs::prelude::{MessageWriter, Res};
use temper_messages::save_chunk_entities::SaveChunkEntities;
use temper_state::GlobalStateResource;

pub fn send_save_message(
    mut writer: MessageWriter<SaveChunkEntities>,
    state: Res<GlobalStateResource>,
) {
    for pos in state.0.world.cached_chunk_keys() {
        writer.write(SaveChunkEntities(pos));
    }
}
