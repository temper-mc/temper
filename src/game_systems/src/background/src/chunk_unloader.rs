use bevy_ecs::prelude::{Commands, Entity, Has, MessageWriter, Query, Res};
use std::collections::{HashMap, HashSet};
use temper_components::last_chunk_pos::LastChunkPos;
use temper_components::player::chunk_receiver::ChunkReceiver;
use temper_components::player::player_marker::PlayerMarker;
use temper_entities::MobKind;
use temper_messages::DespawnMob;
use temper_state::GlobalStateResource;
use tracing::{error, trace};

pub fn handle(
    state: Res<GlobalStateResource>,
    query: Query<(Entity, &ChunkReceiver)>,
    mut cmd: Commands,
    entity_query: Query<(Entity, &LastChunkPos, Has<PlayerMarker>, Has<MobKind>)>,
    mut despawn_mobs: MessageWriter<DespawnMob>,
) {
    // Publish the latest client interests to the lifecycle owner. That same
    // owner also accounts for active generation and pending storage writes.
    let mut owners = HashMap::new();
    for (entity, chunk_receiver) in query.iter() {
        let chunks = chunk_receiver
            .chunks
            .protected_chunks()
            .collect::<HashSet<_>>();
        if !chunks.is_empty() {
            owners.insert(entity.to_bits(), chunks);
        }
    }
    state.0.world.chunk_lifecycle().replace_owners(owners);

    // The lifecycle owner atomically decides whether each cache entry can be
    // detached, including client interests and generation leases.
    let all_chunks = state.0.world.cached_chunk_keys();
    let mut unloaded_entries = 0;
    let mut lifecycle_skipped = 0;
    let mut chunks_to_write = Vec::new();

    for chunk_pos in all_chunks {
        if let Some(eviction) = state.0.world.evict_chunk(chunk_pos) {
            // Despawn entities whose last known location belonged to the chunk.
            for (entity, last_chunk, is_player, is_mob) in entity_query.iter() {
                if is_player || last_chunk.0 != chunk_pos {
                    continue;
                }

                if is_mob {
                    despawn_mobs.write(DespawnMob {
                        entity,
                        remove_from_chunk: false,
                    });
                } else {
                    cmd.entity(entity).despawn();
                }
            }

            if eviction.needs_save() {
                chunks_to_write.push(eviction);
            } else {
                eviction
                    .finish_clean()
                    .expect("clean chunk eviction should finish without a save");
            }

            unloaded_entries += 1;
        } else {
            lifecycle_skipped += 1;
        }
    }

    let written_chunks = chunks_to_write.len();
    if written_chunks > 0 {
        state.0.thread_pool.oneshot(move || {
            for eviction in chunks_to_write {
                let pos = eviction.pos();
                if let Err(err) = eviction.save() {
                    error!("Failed to write chunk {} back to storage: {:?}", pos, err);
                }
            }
        });
    }

    if unloaded_entries > 0 || lifecycle_skipped > 0 {
        trace!(
            "Unloaded {} chunks ({} queued for write, {} retained by the lifecycle owner). {} chunks remain in cache.",
            unloaded_entries,
            written_chunks,
            lifecycle_skipped,
            state.0.world.cached_chunk_count()
        );
    }
}
