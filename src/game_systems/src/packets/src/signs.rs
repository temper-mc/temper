use bevy_ecs::{
    message::MessageReader,
    system::{Query, Res},
};

use temper_block_properties::Direction;
use temper_blocks::BlockDispatch;
use temper_blocks_generated::{HangingSignBlock, SignBlock, WallHangingSignBlock};
use temper_codec::net_types::{network_position::NetworkPosition, var_int::VarInt};
use temper_components::player::{position::Position, rotation::Rotation};
use temper_core::{block_state_id::BlockStateId, pos::BlockPos};
use temper_messages::{BlockEntityPlaced, BlockInteractMessage};
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::{
    SignUpdateReceiver,
    outgoing::{block_entity_data::BlockEntityDataPacket, open_sign_editor::OpenSignEditor},
};
use temper_state::GlobalStateResource;
use temper_text::{TextComponent, TextContent};
use temper_world::Dimension;
use temper_world_format::{BlockEntityKind, SignBlockEntity};

use tracing::{error, trace};

/// A sanity bound on sign line length. Vanilla's client limits by rendered
/// pixel width rather than character count, which isn't worth replicating —
/// this exists to stop a modified client storing and broadcasting megabytes
/// of text per edit.
const MAX_SIGN_LINE_LEN: usize = 384;

pub fn handle_sign_placed(
    mut events: MessageReader<BlockEntityPlaced>,
    query: Query<&StreamWriter>,
) {
    for event in events.read() {
        if event.kind != BlockEntityKind::Sign {
            continue;
        }

        let Ok(conn) = query.get(event.player) else {
            continue;
        };

        if let Err(err) = conn.send_packet(OpenSignEditor {
            location: NetworkPosition {
                x: event.position.pos.x,
                y: event.position.pos.y as i16,
                z: event.position.pos.z,
            },
            is_front_text: true,
        }) {
            error!("Failed to send open sign editor packet: {:?}", err);
        }
    }
}

/// Stores text the client sent back after closing the sign editor.
pub fn handle_sign_update(
    receiver: Res<SignUpdateReceiver>,
    state: Res<GlobalStateResource>,
    players: Query<(&StreamWriter, &Position)>,
) {
    for (event, eid) in receiver.0.try_iter() {
        let block_pos: BlockPos = event.position.into();

        if [&event.line_1, &event.line_2, &event.line_3, &event.line_4]
            .iter()
            .any(|line| line.len() > MAX_SIGN_LINE_LEN)
        {
            trace!("Rejecting oversized sign update from {eid:?} at {block_pos}");
            continue;
        }

        let Ok(chunk) = state
            .0
            .world
            .get_chunk(block_pos.chunk(), Dimension::Overworld)
        // todo: dimensions
        else {
            error!("Failed to get chunk for sign update at {block_pos}");
            continue;
        };

        let Some(mut entry) = chunk.block_entities.get_mut(&block_pos.chunk_block_pos()) else {
            trace!("Sign update from {eid:?} for a position with no block entity: {block_pos}");
            continue;
        };

        if entry.kind != BlockEntityKind::Sign {
            trace!("Sign update from {eid:?} for a non-sign block entity: {block_pos}");
            continue;
        }

        let mut sign: SignBlockEntity = match serde_json::from_slice(&entry.blob) {
            Ok(sign) => sign,
            Err(err) => {
                error!("Failed to read sign at {block_pos}: {err}");
                continue;
            }
        };

        if sign.is_waxed {
            trace!("Sign update from {eid:?} for a waxed sign: {block_pos}");
            continue;
        }

        let text = if event.is_front_text {
            &mut sign.front_text
        } else {
            &mut sign.back_text
        };

        text.messages = [&event.line_1, &event.line_2, &event.line_3, &event.line_4]
            .into_iter()
            .map(|line| TextComponent {
                content: TextContent::Text { text: line.clone() },
                ..Default::default()
            })
            .collect();

        let blob = match sign.to_blob() {
            Ok(blob) => blob,
            Err(err) => {
                error!("Failed to write sign at {block_pos}: {err}");
                continue;
            }
        };
        let protocol_id = entry.protocol_id;
        entry.blob = blob.clone();

        drop(entry);
        chunk.mark_dirty();

        let nbt = match BlockEntityKind::Sign.to_network_nbt(&blob) {
            Ok(nbt) => nbt,
            Err(err) => {
                error!("Failed to build sign nbt for {block_pos}: {err}");
                continue;
            }
        };

        let packet = BlockEntityDataPacket {
            location: NetworkPosition {
                x: block_pos.pos.x,
                y: block_pos.pos.y as i16,
                z: block_pos.pos.z,
            },
            entity_type: VarInt::new(i32::from(protocol_id)),
            nbt,
        };

        let block_chunk = block_pos.chunk();
        let render_distance = state.0.config.chunk_render_distance as i32;

        for (conn, pos) in players.iter() {
            let player_chunk = pos.chunk();
            if (block_chunk.x() - player_chunk.x()).abs() <= render_distance
                && (block_chunk.z() - player_chunk.z()).abs() <= render_distance
                && let Err(err) = conn.send_packet_ref(&packet)
            {
                error!("Failed to send block entity data packet: {:?}", err);
            }
        }
    }
}

/// Reopens the sign editor when a player right-clicks an unwaxed sign.
pub fn handle_sign_interact(
    mut events: MessageReader<BlockInteractMessage>,
    state: Res<GlobalStateResource>,
    query: Query<(&StreamWriter, &Rotation)>,
) {
    for event in events.read() {
        let block_pos = event.position;

        let Ok(chunk) = state
            .0
            .world
            .get_chunk(block_pos.chunk(), Dimension::Overworld)
        // todo: dimensions
        else {
            continue;
        };

        let Some(entry) = chunk.block_entities.get(&block_pos.chunk_block_pos()) else {
            continue;
        };

        if entry.kind != BlockEntityKind::Sign {
            continue;
        }

        let sign: SignBlockEntity = match serde_json::from_slice(&entry.blob) {
            Ok(sign) => sign,
            Err(err) => {
                error!("Failed to read sign at {block_pos}: {err}");
                continue;
            }
        };

        if sign.is_waxed {
            trace!("Ignoring interact on a waxed sign at {block_pos}");
            continue;
        }

        drop(entry);

        let block_state = chunk.get_block(block_pos.chunk_block_pos());

        let Ok((conn, rotation)) = query.get(event.player) else {
            continue;
        };

        let is_front_text = sign_front_faces_player(block_state, rotation.yaw);

        if let Err(err) = conn.send_packet(OpenSignEditor {
            location: NetworkPosition {
                x: block_pos.pos.x,
                y: block_pos.pos.y as i16,
                z: block_pos.pos.z,
            },
            is_front_text,
        }) {
            error!("Failed to send open sign editor packet: {:?}", err);
        }
    }
}

/// Whether the player is looking at the sign's front face. Signs with two
/// visible faces need this so right-clicking edits the side you can see;
/// wall signs have one visible face and always use the front.
fn sign_front_faces_player(block_state: BlockStateId, yaw: f32) -> bool {
    if let Some(sign) = block_state.try_cast::<SignBlock>() {
        return front_faces_player(sign.rotation, yaw);
    }

    if let Some(sign) = block_state.try_cast::<HangingSignBlock>() {
        return front_faces_player(sign.rotation, yaw);
    }

    if let Some(sign) = block_state.try_cast::<WallHangingSignBlock>() {
        let front = match sign.facing {
            Direction::South => 0.0,
            Direction::West => 90.0,
            Direction::North => 180.0,
            Direction::East => 270.0,
            _ => return true,
        };
        let diff = (yaw - front).rem_euclid(360.0);
        return diff > 90.0 && diff < 270.0;
    }
    true
}

/// Shared by standing and hanging signs, which both use a 16-step rotation.
fn front_faces_player(rotation: i32, yaw: f32) -> bool {
    let front = (rotation as f32) * 22.5;
    let diff = (yaw - front).rem_euclid(360.0);
    diff > 90.0 && diff < 270.0
}
