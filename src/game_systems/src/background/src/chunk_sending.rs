use anyhow::Context;
use bevy_ecs::prelude::{Entity, MessageWriter, Query, Res};
use std::cmp::max;
use std::collections::HashSet;
use std::sync::atomic::Ordering;
use temper_codec::encode::NetEncodeOpts;
use temper_components::player::chunk_receiver::{ChunkReceiver, PreparedChunk, ReadyChunk};
use temper_components::player::entity_tracker::EntityTracker;
use temper_components::player::position::Position;
use temper_core::pos::{ChunkPos, FullChunkPos};
use temper_net_runtime::compression::compress_packet;
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::outgoing::chunk_and_light_data::ChunkAndLightData;
use temper_protocol::outgoing::chunk_batch_finish::ChunkBatchFinish;
use temper_protocol::outgoing::chunk_batch_start::ChunkBatchStart;
use temper_protocol::outgoing::set_center_chunk::SetCenterChunk;
use temper_state::GlobalStateResource;
use tracing::error;

/// How many times a chunk may fail to prepare before we stop retrying it.
const MAX_CHUNK_RETRIES: u8 = 3;

pub fn handle(
    mut query: Query<(
        Entity,
        &StreamWriter,
        &mut ChunkReceiver,
        &Position,
        &EntityTracker,
    )>,
    state: Res<GlobalStateResource>,
    mut mob_load_writer: MessageWriter<temper_messages::load_chunk_entities::LoadChunkEntities>,
) {
    // Cap on how much chunk work one player can have queued on the pool at once.
    // Without it, fast flight outruns generation and `in_flight` grows without
    // bound — measured climbing past 600 on a 24-core machine, with harvest
    // batches large enough to push ticks to 350ms. The multiplier was fitted on
    // that machine: 6x cores kept in-flight work bounded and the worst tick
    // spikes under 100ms. It does not make generation faster; sustained flight
    // past pool capacity still backs up, it just backs up in `loading` where it
    // costs memory instead of pool depth.
    let max_in_flight = state.0.thread_pool.core_count() * 6;

    for (eid, conn, mut chunk_receiver, pos, entity_tracker) in query.iter_mut() {
        if !state.0.players.is_connected(eid) {
            continue;
        }

        let chunk_receiver = &mut *chunk_receiver;

        // ==========================================
        // PHASE 1: HARVEST & SEND COMPLETED CHUNKS
        // ==========================================
        let mut harvested = Vec::with_capacity(chunk_receiver.ready_rx.len());

        // Pull everything the pool has finished with — successes and failures
        while let Ok(prepared) = chunk_receiver.ready_rx.try_recv() {
            harvested.push(prepared);
        }

        // Anything harvested is no longer in flight, whatever its outcome.
        // This is what stops a failure from stranding the coordinate forever.
        for prepared in &harvested {
            chunk_receiver
                .chunks
                .finish_preparation(prepared.chunk_pos());
        }

        let mut ready_to_send: Vec<ReadyChunk> = Vec::new();
        for prepared in harvested {
            match prepared {
                PreparedChunk::Ready(ready) => {
                    // Success wipes the failure history for this chunk, so a
                    // chunk that fails twice then succeeds starts clean.
                    chunk_receiver.retry_counts.remove(&ready.chunk_pos);

                    // The player may have flown past while this was generating.
                    // The chunk calculator owns the current view, so stale work
                    // is discarded here and can be queued again if they return.
                    if chunk_receiver.chunks.is_needed(ready.chunk_pos) {
                        ready_to_send.push(ready);
                    }
                }
                PreparedChunk::Failed { chunk_pos } => {
                    if !chunk_receiver.chunks.is_needed(chunk_pos) {
                        chunk_receiver.retry_counts.remove(&chunk_pos);
                        continue;
                    }

                    let attempts = chunk_receiver.retry_counts.entry(chunk_pos).or_insert(0);
                    *attempts += 1;

                    if *attempts >= MAX_CHUNK_RETRIES {
                        error!(
                            "Chunk {} failed to prepare {} times; giving up for this session",
                            chunk_pos, attempts
                        );
                        chunk_receiver.retry_counts.remove(&chunk_pos);
                    } else {
                        chunk_receiver.chunks.queue_loading(chunk_pos);
                    }
                }
            }
        }

        if !ready_to_send.is_empty() {
            conn.send_packet(ChunkBatchStart {})
                .expect("Failed to send ChunkBatchStart");

            let center_chunk = ChunkPos::from(pos.coords);
            conn.send_packet(SetCenterChunk {
                x: center_chunk.x().into(),
                z: center_chunk.z().into(),
            })
            .expect("Failed to send SetCenterChunk");

            let packets_len = ready_to_send.len();

            for chunk in ready_to_send {
                let marked_loaded = chunk_receiver.chunks.mark_loaded(chunk.chunk_pos);
                debug_assert!(marked_loaded, "only needed chunks should be sent");

                if chunk.is_new_load {
                    mob_load_writer.write(temper_messages::load_chunk_entities::LoadChunkEntities(
                        chunk.chunk_pos,
                    ));
                }

                if let Err(err) = conn.send_raw_packet(chunk.packet_data) {
                    error!("Failed to send chunk packet: {:?}", err);
                }

                for entity_tuple in chunk.entities {
                    entity_tracker.to_track.push(entity_tuple);
                }
            }

            if let Err(err) = conn.send_packet(ChunkBatchFinish {
                batch_size: packets_len.into(),
            }) {
                error!("Failed to send ChunkBatchFinish packet: {:?}", err);
            }
        }

        // tell the client to unload chunks that are no longer needed
        while let Some(chunk_pos) = chunk_receiver.chunks.pop_unloading() {
            let packet = temper_protocol::outgoing::unload_chunk::UnloadChunk {
                x: chunk_pos.x(),
                z: chunk_pos.z(),
            };
            if let Err(err) = conn.send_packet(packet) {
                error!("Failed to send UnloadChunk packet: {:?}", err);
            }
        }

        // ==========================================
        // PHASE 2: DISPATCH NEW CHUNKS (NON-BLOCKING)
        // ==========================================
        let chunk_per_tick = match state.0.config.performance.chunks_per_tick {
            0 => max(
                chunk_receiver.chunks.loading_len() / 3,
                state.0.config.performance.chunks_per_tick_min as usize,
            ),
            -1 => usize::MAX,
            hard_limit => hard_limit as usize,
        };

        let dispatch_budget = max_in_flight.saturating_sub(chunk_receiver.chunks.in_flight_len());
        if dispatch_budget == 0 {
            continue;
        }
        let chunk_per_tick = chunk_per_tick.min(dispatch_budget);

        if !chunk_receiver.chunks.has_pending_work() {
            continue;
        }

        let mut dirty_chunks = Vec::new();
        let mut sent_chunks = 0;

        while let Some(coords) = chunk_receiver.chunks.pop_dirty() {
            dirty_chunks.push(coords);
            sent_chunks += 1;
            if sent_chunks >= chunk_per_tick {
                break;
            }
        }

        let mut needed_chunks: Vec<FullChunkPos> = Vec::new();

        if sent_chunks < chunk_receiver.chunks_per_tick as usize {
            while let Some(coords) = chunk_receiver.chunks.pop_loading() {
                needed_chunks.push(coords);
                sent_chunks += 1;
                if sent_chunks >= chunk_per_tick {
                    break;
                };
            }
        }

        let loading_chunks: HashSet<_> = needed_chunks.iter().copied().collect();
        needed_chunks.extend(dirty_chunks);

        if needed_chunks.is_empty() {
            continue;
        }

        // dispatch to the thread pool
        for coordinates in needed_chunks {
            if !chunk_receiver.chunks.begin_preparation(coordinates) {
                continue;
            }
            let is_new_load = loading_chunks.contains(&coordinates);
            let is_compressed = conn.compress.load(Ordering::Relaxed);

            let state_clone = state.clone();
            let tx = chunk_receiver.ready_tx.clone();

            state.0.thread_pool.oneshot(move || {
                // Inner closure so we can use `?` on the fallible steps instead
                // of unwinding the whole pool thread on any of them.
                let prepare = || -> anyhow::Result<PreparedChunk> {
                    let chunk_data = {
                        let chunk_ref = state_clone
                            .0
                            .world
                            .get_or_generate_chunk(coordinates)
                            .context("load or generate failed")?;

                        (*chunk_ref).clone_without_transient_noise()
                    };

                    let mut entities = Vec::with_capacity(chunk_data.entities.len());
                    for kv in chunk_data.entities.iter() {
                        entities.push((*kv.key(), kv.value().0.to_entity_type().id));
                    }

                    let packet = ChunkAndLightData::from_chunk(*coordinates, &chunk_data)
                        .context("building ChunkAndLightData failed")?;

                    let packet_data = compress_packet(
                        &packet,
                        is_compressed,
                        &NetEncodeOpts::WithLength,
                        state_clone.0.config.network_compression_threshold as usize,
                    )
                    .context("compressing chunk packet failed")?;

                    Ok(PreparedChunk::Ready(ReadyChunk {
                        chunk_pos: coordinates,
                        packet_data,
                        entities,
                        is_new_load,
                    }))
                };

                let message = prepare().unwrap_or_else(|err| {
                    error!("Chunk {} failed to prepare: {err:#}", coordinates);
                    PreparedChunk::Failed {
                        chunk_pos: coordinates,
                    }
                });

                let _ = tx.send(message);
            });
        }
    }
}
