use bevy_ecs::prelude::*;
use temper_codec::net_types::network_position::NetworkPosition;
use temper_codec::net_types::var_int::VarInt;
use temper_components::player::position::Position;
use temper_config::server_config::get_global_config;
use temper_messages::{BlockInteractMessage, BlockToggledEvent, DoorToggledEvent};
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::outgoing::block_change_ack::BlockChangeAck;
use temper_protocol::outgoing::block_update::BlockUpdate;
use temper_state::GlobalStateResource;
use temper_world::Dimension;
use tracing::{debug, error};

use crate::block_interactions::{try_interact, InteractionResult};
use crate::door_interaction::{door_other_half_y_offset, is_open};

pub fn handle_block_interact(
    mut events: MessageReader<BlockInteractMessage>,
    state: Res<GlobalStateResource>,
    query: Query<(Entity, &StreamWriter, &Position)>,
    mut toggled_writer: MessageWriter<BlockToggledEvent>,
    mut door_toggled_writer: MessageWriter<DoorToggledEvent>,
) {
    for event in events.read() {
        let pos = event.position;

        // Load the chunk and get current block state
        let mut chunk = match temper_world::World::get_or_generate_mut(
            &state.0.world,
            pos.chunk(),
            Dimension::Overworld,
        ) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to load chunk for block interaction: {:?}", e);
                continue;
            }
        };

        let (updates, is_active, new_state) = {
            let block_state = chunk.get_block(pos.chunk_block_pos());

            // Try to interact (toggle) the block
            let new_state = match try_interact(block_state) {
                InteractionResult::Toggled(new) => new,
                _ => continue,
            };

            chunk.set_block(pos.chunk_block_pos(), new_state);
            debug!(
                "Block interact: toggled ({}, {}, {}) from {} to {}",
                pos.pos.x,
                pos.pos.y,
                pos.pos.z,
                block_state.raw(),
                new_state.raw()
            );

            let updates = vec![BlockUpdate {
                location: NetworkPosition {
                    x: pos.pos.x,
                    y: pos.pos.y as i16,
                    z: pos.pos.z,
                },
                block_state_id: VarInt::from(new_state),
            }];

            let is_active = is_open(new_state).unwrap_or(false);
            (updates, is_active, new_state)
        }; // chunk lock released here

        // If it's a door, let the door handler toggle the other half
        if door_other_half_y_offset(new_state).is_some() {
            door_toggled_writer.write(DoorToggledEvent {
                position: pos,
                new_state,
            });
        }

        // Emit BlockToggledEvent for other systems to react
        toggled_writer.write(BlockToggledEvent {
            player: event.player,
            position: pos,
            is_active,
        });

        // Send BlockChangeAck to the player
        if let Ok((_, conn, _)) = query.get(event.player) {
            let ack = BlockChangeAck {
                sequence: event.sequence,
            };
            if let Err(e) = conn.send_packet_ref(&ack) {
                error!("Failed to send BlockChangeAck: {:?}", e);
            }
        }

        // Broadcast BlockUpdate to all players within render distance
        let block_chunk = pos.chunk();
        let (block_cx, block_cz) = (block_chunk.x(), block_chunk.z());
        let render_distance = get_global_config().chunk_render_distance as i32;

        for (_, conn, player_pos) in query.iter() {
            let pchunk = player_pos.chunk();
            let (pcx, pcz) = (pchunk.x(), pchunk.z());

            if (block_cx - pcx).abs() <= render_distance
                && (block_cz - pcz).abs() <= render_distance
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
