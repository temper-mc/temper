//! Block Interaction System
//!
//! This module handles direct block interactions in the world (chunks).
//! For simple toggleable blocks (doors, levers, trapdoors, buttons),
//! the system modifies block states directly without creating ECS entities.
//!
//! ## How it works
//!
//! 1. Player right-clicks on a block (PlaceBlock packet)
//! 2. The `interact()` method is called on the blockstate
//! 3. `interact()` will change the state to the new state and also return any other blocks to update
//! 4. The packet handler updates the chunk and broadcasts to players

use bevy_ecs::change_detection::Res;
use bevy_ecs::entity::Entity;
use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::{Local, Query};
use fastcache::Cache;
use std::time::{Duration, Instant};
use temper_blocks::BlockDispatch;
use temper_codec::net_types::network_position::NetworkPosition;
use temper_codec::net_types::var_int::VarInt;
use temper_components::InteractionCooldown;
use temper_components::player::client_information::ClientInformationComponent;
use temper_components::player::position::Position;
use temper_core::dimension::Dimension;
use temper_core::pos::BlockPos;
use temper_messages::BlockInteractMessage;
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::outgoing::block_change_ack::BlockChangeAck;
use temper_protocol::outgoing::block_update::BlockUpdate;
use temper_state::GlobalStateResource;
use tracing::{debug, error};

type InteractQuery<'a> = (
    Entity,
    &'a StreamWriter,
    &'a Position,
    &'a ClientInformationComponent,
);

/// Tells the client its predicted interaction was received. The client holds
/// the prediction until it sees this sequence, so every path out of the
/// handler needs to send one.
fn send_ack(query: &Query<InteractQuery>, player: Entity, sequence: VarInt, context: &str) {
    if let Ok((_, conn, _, _)) = query.get(player)
        && let Err(e) = conn.send_packet_ref(&BlockChangeAck { sequence })
    {
        error!("Failed to send BlockChangeAck ({context}): {:?}", e);
    }
}

pub fn handle_block_interact(
    mut events: MessageReader<BlockInteractMessage>,
    state: Res<GlobalStateResource>,
    query: Query<InteractQuery>,
    mut cooldowns: Local<Option<Cache<BlockPos, Instant>>>,
) {
    let cooldown_duration = Duration::from_millis(InteractionCooldown::default().cooldown_ms);

    for event in events.read() {
        let pos = event.position;

        // Ignore rapid repeated interactions on the same block
        if let Some(cooldowns_map) = cooldowns.as_ref()
            && cooldowns_map
                .get(&pos)
                .is_some_and(|t| t.elapsed() < cooldown_duration)
        {
            send_ack(&query, event.player, event.sequence, "cooldown");
            continue;
        }
        cooldowns
            .get_or_insert(Cache::new(512, Duration::from_secs(30)))
            .insert(pos, Instant::now());

        // Load the chunk and get current block state
        let mut block_state = state
            .0
            .world
            .get_chunk(pos.chunk(), Dimension::Overworld)
            .map(|chunk| chunk.get_block(pos.chunk_block_pos()))
            .unwrap();
        let original = block_state;

        let updates = {
            let updates = block_state.interact(&state.0.world, pos);

            if updates.blocks.is_empty() && block_state == original {
                // Nothing changed, another system handles this interaction,
                // but the client is still waiting on the sequence.
                send_ack(&query, event.player, event.sequence, "unhandled");
                continue;
            }

            let mut updates = updates.blocks;
            updates.insert(pos, block_state);

            debug!(
                "Block interact: toggled ({}, {}, {}) from {} to {}",
                pos.pos.x,
                pos.pos.y,
                pos.pos.z,
                original.raw(),
                block_state.raw()
            );

            for (pos, block_state) in &updates {
                if let Err(error) = state
                    .0
                    .world
                    .get_chunk_mut(pos.chunk(), Dimension::Overworld)
                    .map(|mut chunk| chunk.set_block(pos.chunk_block_pos(), *block_state))
                {
                    error!(
                        "Attempted to update block at {} to {} but failed: {}",
                        pos,
                        block_state.raw(),
                        error
                    );
                }
            }

            updates
                .into_iter()
                .map(|(pos, state)| BlockUpdate {
                    location: NetworkPosition {
                        x: pos.pos.x,
                        y: pos.pos.y as i16,
                        z: pos.pos.z,
                    },
                    block_state_id: VarInt::from(state),
                })
                .collect::<Vec<_>>()
        }; // chunk lock released here

        send_ack(&query, event.player, event.sequence, "interact");

        // Broadcast BlockUpdate to all players within render distance
        let block_chunk = pos.chunk();
        let (block_cx, block_cz) = (block_chunk.x(), block_chunk.z());
        let render_distance = state.0.config.chunk_render_distance;

        for (_, conn, player_pos, client_info) in query.iter() {
            let pchunk = player_pos.chunk();
            let (pcx, pcz) = (pchunk.x(), pchunk.z());

            let player_render_distance = u32::from(client_info.view_distance).min(render_distance);

            if (block_cx - pcx).abs() <= player_render_distance as i32
                && (block_cz - pcz).abs() <= player_render_distance as i32
            {
                for update in &updates {
                    if let Err(e) = conn.send_packet_ref(update) {
                        error!("Failed to send block update: {:?}", e);
                    }
                }
            }
        }
    }
}
