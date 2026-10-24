use bevy_ecs::prelude::{Commands, Entity, Has, MessageWriter, Query, Res};
use std::collections::HashSet;
use temper_components::last_chunk_pos::LastChunkPos;
use temper_components::player::chunk_receiver::ChunkReceiver;
use temper_components::player::player_marker::PlayerMarker;
use temper_core::pos::ChunkPos;
use temper_entities::MobKind;
use temper_messages::DespawnMob;
use temper_state::GlobalStateResource;
use tracing::{error, trace};

pub fn handle(
    state: Res<GlobalStateResource>,
    query: Query<&ChunkReceiver>,
    mut cmd: Commands,
    entity_query: Query<(Entity, &LastChunkPos, Has<PlayerMarker>, Has<MobKind>)>,
    mut despawn_mobs: MessageWriter<DespawnMob>,
) {
    let mut visible_chunks = HashSet::new();

    // gather chunks players can see OR are waiting to see
    for chunk_receiver in query.iter() {
        visible_chunks.extend(chunk_receiver.loaded.iter().copied());

        // this protects chunks currently being generated/sent
        visible_chunks.extend(chunk_receiver.loading.iter().copied());

        // this protects chunks waiting for block updates
        visible_chunks.extend(chunk_receiver.dirty.iter().copied());

        // this protects chunks dispatched to the pool but not yet harvested
        visible_chunks.extend(chunk_receiver.in_flight.iter().copied());
    }

    // map all chunks currently in the cache
    let mut all_chunks = HashSet::new();
    for entry in state.0.world.get_cache().iter() {
        all_chunks.insert(*entry.key());
    }

    // A chunk with live generation jobs also needs its neighbours kept
    // resident: higher stages read neighbours as snapshots after those
    // neighbours' own jobs have already completed, so `has_pending_jobs`
    // alone doesn't protect them.
    let mut generation_locked = HashSet::new();
    for chunk_pos in all_chunks.iter() {
        if state
            .0
            .world
            .chunk_generators
            .get(&chunk_pos.dimension())
            .expect("should have generator for dimension")
            .has_pending_jobs(chunk_pos.dimension(), **chunk_pos)
        {
            for dx in -1..=1 {
                for dz in -1..=1 {
                    generation_locked.insert(ChunkPos::new(chunk_pos.x() + dx, chunk_pos.z() + dz));
                }
            }
        }
    }

    let mut unloaded_entries = 0;
    let mut generation_skipped = 0;
    let mut chunks_to_write = Vec::new();

    // unload anything not in the visible/pending set
    // if 0 players are online, visible_chunks is empty, so this should gracefully unload the entire server.
    for chunk_pos in all_chunks.difference(&visible_chunks) {
        if generation_locked.contains(chunk_pos) {
            generation_skipped += 1;
            continue;
        }

        if let Some((pos, chunk)) = state.0.world.get_cache().remove(chunk_pos) {
            // drop this chunk's completed job entries
            state
                .0
                .world
                .chunk_generators
                .get(&pos.dimension())
                .expect("should have generator for dimension")
                .forget_chunk(pos.dimension(), *pos);

            // despawn orphaned entities
            for (entity, last_chunk, is_player, is_mob) in entity_query.iter() {
                if is_player || last_chunk.0 != pos {
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
            // queue for saving, writes happen off-thread, below
            if chunk.is_dirty() {
                chunks_to_write.push((pos, chunk));
            }

            unloaded_entries += 1;
        }
    }

    let written_chunks = chunks_to_write.len();
    if written_chunks > 0 {
        let state_clone = state.clone();
        state.0.thread_pool.oneshot(move || {
            for (pos, chunk) in chunks_to_write {
                if let Err(err) = state_clone.0.world.insert_chunk(pos, chunk) {
                    error!("Failed to write chunk {} back to storage: {:?}", pos, err);
                }
            }
        });
    }

    if unloaded_entries > 0 || generation_skipped > 0 {
        trace!(
            "Unloaded {} chunks ({} queued for write, {} kept for in-progress generation). {} chunks remain in cache.",
            unloaded_entries,
            written_chunks,
            generation_skipped,
            state.0.world.get_cache().len()
        );
    }
}
