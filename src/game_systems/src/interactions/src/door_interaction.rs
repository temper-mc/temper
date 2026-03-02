use bevy_ecs::prelude::*;
use temper_codec::net_types::network_position::NetworkPosition;
use temper_codec::net_types::var_int::VarInt;
use temper_components::player::position::Position;
use temper_config::server_config::get_global_config;
use temper_core::block_state_id::BlockStateId;
use temper_core::pos::BlockPos;
use temper_messages::{BlockBrokenEvent, DoorToggledEvent};
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::outgoing::block_update::BlockUpdate;
use temper_state::GlobalStateResource;
use temper_world::Dimension;
use tracing::{debug, error};

use crate::block_interactions::{InteractionResult, try_interact};

/// Given a block state, if it's a door, returns the Y offset to the other half.
/// Lower half -> +1, upper half -> -1, not a door -> None.
pub fn door_other_half_y_offset(block_state_id: BlockStateId) -> Option<i32> {
    let data = block_state_id.to_block_data()?;
    if !data.name.ends_with("_door") {
        return None;
    }
    let props = data.properties.as_ref()?;
    let half = props.get("half")?;
    match half.as_str() {
        "lower" => Some(1),
        "upper" => Some(-1),
        _ => None,
    }
}

/// Gets the "open" state of a door/trapdoor/fence gate.
pub fn is_open(block_state_id: BlockStateId) -> Option<bool> {
    let block_data = block_state_id.to_block_data()?;
    let properties = block_data.properties.as_ref()?;
    let open_value = properties.get("open")?;
    Some(open_value == "true")
}

/// Breaks a block and its door-pair (if applicable).
/// Sets both positions to air and emits `BlockBrokenEvent` for each.
/// Returns the list of all positions that were broken (always includes `pos`,
/// and may include the other door half).
pub fn break_block_with_door_half(
    chunk: &mut temper_world::MutChunk,
    pos: BlockPos,
    block_break_writer: &mut MessageWriter<BlockBrokenEvent>,
) -> Vec<BlockPos> {
    let current_state = chunk.get_block(pos.chunk_block_pos());
    let other_half = door_other_half_y_offset(current_state).map(|y_off| pos + (0, y_off, 0));

    chunk.set_block(pos.chunk_block_pos(), BlockStateId::default());
    block_break_writer.write(BlockBrokenEvent { position: pos });

    let mut broken = vec![pos];

    if let Some(other_pos) = other_half {
        chunk.set_block(other_pos.chunk_block_pos(), BlockStateId::default());
        block_break_writer.write(BlockBrokenEvent {
            position: other_pos,
        });
        debug!(
            "Also broke other door half at ({}, {}, {})",
            other_pos.pos.x, other_pos.pos.y, other_pos.pos.z
        );
        broken.push(other_pos);
    }

    broken
}

/// Reacts to `DoorToggledEvent` and toggles the other half of the door block.
pub fn handle_door_toggled(
    mut events: MessageReader<DoorToggledEvent>,
    state: Res<GlobalStateResource>,
    query: Query<(&StreamWriter, &Position)>,
) {
    for event in events.read() {
        let pos = event.position;

        let Some(y_offset) = door_other_half_y_offset(event.new_state) else {
            continue;
        };
        let other_pos = pos + (0, y_offset, 0);

        let update = {
            let mut chunk = match temper_world::World::get_or_generate_mut(
                &state.0.world,
                other_pos.chunk(),
                Dimension::Overworld,
            ) {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to load chunk for door other half toggle: {:?}", e);
                    continue;
                }
            };

            let other_state = chunk.get_block(other_pos.chunk_block_pos());
            let InteractionResult::Toggled(other_new) = try_interact(other_state) else {
                continue;
            };

            chunk.set_block(other_pos.chunk_block_pos(), other_new);
            debug!(
                "Door other half: toggled ({}, {}, {}) to {}",
                other_pos.pos.x,
                other_pos.pos.y,
                other_pos.pos.z,
                other_new.raw()
            );

            BlockUpdate {
                location: NetworkPosition {
                    x: other_pos.pos.x,
                    y: other_pos.pos.y as i16,
                    z: other_pos.pos.z,
                },
                block_state_id: VarInt::from(other_new),
            }
        }; // chunk lock released here

        let block_chunk = other_pos.chunk();
        let (block_cx, block_cz) = (block_chunk.x(), block_chunk.z());
        let render_distance = get_global_config().chunk_render_distance as i32;

        for (conn, player_pos) in query.iter() {
            let pchunk = player_pos.chunk();
            let (pcx, pcz) = (pchunk.x(), pchunk.z());

            if (block_cx - pcx).abs() <= render_distance
                && (block_cz - pcz).abs() <= render_distance
            {
                if let Err(e) = conn.send_packet_ref(&update) {
                    error!("Failed to send door half block update: {:?}", e);
                }
            }
        }
    }
}
