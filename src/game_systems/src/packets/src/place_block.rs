use bevy_ecs::message::MessageWriter;
use bevy_ecs::prelude::{Entity, Query, Res};
use temper_codec::net_types::network_position::NetworkPosition;
use temper_components::player::position::Position;
use temper_components::{bounds::CollisionBounds, player::sneak::SneakState};
use temper_core::pos::BlockPos;
use temper_messages::{BlockEntityPlaced, BlockInteractMessage};

use bevy_math::{DVec3, IVec3};
use temper_blocks::BlockDispatch;
use temper_components::player::rotation::Rotation;
use temper_core::block_state_id::{BlockStateId, ITEM_TO_BLOCK_MAPPING};
use temper_core::dimension::Dimension;
use temper_core::mq;
use temper_inventories::hotbar::Hotbar;
use temper_inventories::inventory::Inventory;
use temper_messages::world_change::WorldChange;
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::PlaceBlockReceiver;
use temper_protocol::outgoing::block_change_ack::BlockChangeAck;
use temper_protocol::outgoing::block_update::BlockUpdate;
use temper_state::GlobalStateResource;
use temper_text::{Color, NamedColor, TextComponentBuilder};
use temper_world_format::{BlockEntityData, BlockEntityKind, SignBlockEntity};
use tracing::{debug, error, trace};

// TODO: in the future this should be reworked so that if a block update exits early the client is informed that the block never updated.
//      Currently it spawns ghost blocks if this function exits early (ie continues on to the next update)
//
// TODO: also create a better block update propagation system. All block updates should happen (preferably) on the same tick to prevent
//      visible slowdowns. When a block is placed, all adjacent blocks should be updated. There's already a built in `update` function on
//      BlockBehavior that should return whether or not the block changed (if it did then we should keep updating blocks next to that)
pub fn handle(
    receiver: Res<PlaceBlockReceiver>,
    state: Res<GlobalStateResource>,
    query: Query<(
        Entity,
        &StreamWriter,
        &Inventory,
        &Hotbar,
        &Position,
        &Rotation,
        &SneakState,
    )>,
    pos_q: Query<(&Position, &CollisionBounds)>,
    mut world_change: MessageWriter<WorldChange>,
    mut block_interact: MessageWriter<BlockInteractMessage>,
    mut block_entity_placed: MessageWriter<BlockEntityPlaced>,
) {
    'ev_loop: for (event, eid) in receiver.0.try_iter() {
        let Ok((entity, conn, inventory, hotbar, _pos, rot, sneak)) = query.get(eid) else {
            debug!("Could not get connection for entity {:?}", eid);
            continue;
        };
        if !state.0.players.is_connected(entity) {
            trace!("Entity {:?} is not connected", entity);
            continue;
        }
        // If the clicked block is interactive and the player is not sneaking,
        // dispatch an interaction and skip block placement entirely.
        {
            let clicked_pos: BlockPos = event.position.clone().into();
            let chunk = state
                .0
                .world
                .get_or_generate_chunk(clicked_pos.chunk(), Dimension::Overworld)
                .expect("Failed to load chunk for interaction check");
            let clicked_block = chunk.get_block(clicked_pos.chunk_block_pos());
            if !sneak.is_sneaking && clicked_block.is_interactable() {
                block_interact.write(BlockInteractMessage {
                    player: entity,
                    position: clicked_pos,
                    sequence: event.sequence,
                });
                continue 'ev_loop;
            }
        }

        let ack_packet = BlockChangeAck {
            sequence: event.sequence,
        };

        match event.hand.0 {
            0 => {
                let Ok(slot) = hotbar.get_selected_item(inventory) else {
                    error!("Could not fetch {:?}", eid);
                    continue 'ev_loop;
                };
                if let Some(selected_item) = slot {
                    let Some(item_id) = selected_item.item_id else {
                        error!("Selected item has no item ID");
                        continue 'ev_loop;
                    };
                    let block_pos: BlockPos = event.position.into();
                    if block_pos.pos.y >= 319 {
                        mq::queue(
                            TextComponentBuilder::new(
                                "Build limit is 319! Cannot place block here.".to_string(),
                            )
                            .color(Color::Named(NamedColor::Red))
                            .bold()
                            .build(),
                            true,
                            entity,
                        );
                        trace!("Block placement out of bounds: {}", block_pos);
                        continue 'ev_loop;
                    } else if block_pos.pos.y <= -64 {
                        mq::queue(
                            TextComponentBuilder::new(
                                "Cannot place block below Y=-64.".to_string(),
                            )
                            .color(Color::Named(NamedColor::Red))
                            .bold()
                            .build(),
                            true,
                            entity,
                        );
                        trace!("Block placement out of bounds: {}", block_pos);
                        continue 'ev_loop;
                    }

                    let mut offset_pos = block_pos
                        + IVec3::new(
                            (event.cursor_x * 2.0 - 1.0) as i32,
                            (event.cursor_y * 2.0 - 1.0) as i32,
                            (event.cursor_z * 2.0 - 1.0) as i32,
                        );

                    let Ok(curr_state) = state.0.world.get_block(offset_pos, Dimension::Overworld)
                    else {
                        error!("Can't get block at {}", offset_pos);
                        continue 'ev_loop;
                    };

                    // Check if the block collides with any entities
                    let does_collide = {
                        pos_q.into_iter().any(|(pos, bounds)| {
                            bounds.collides(
                                (pos.x, pos.y, pos.z),
                                &CollisionBounds {
                                    x_offset_start: 0.0,
                                    x_offset_end: 1.0,
                                    y_offset_start: 0.0,
                                    y_offset_end: 1.0,
                                    z_offset_start: 0.0,
                                    z_offset_end: 1.0,
                                },
                                (
                                    f64::from(offset_pos.pos.x),
                                    f64::from(offset_pos.pos.y),
                                    f64::from(offset_pos.pos.z),
                                ),
                            )
                        })
                    };

                    if does_collide && curr_state.is_solid() {
                        trace!("Block placement collided with entity");
                        continue 'ev_loop;
                    }

                    let Some(mut block_state) = ITEM_TO_BLOCK_MAPPING
                        .get()
                        .unwrap()
                        .get(&(item_id.as_u32() as i32))
                        .copied()
                    else {
                        // Not a placeable item, nothing to do here until item-on-block
                        // interactions exist.
                        continue 'ev_loop;
                    };

                    let mut placement_context = temper_blocks::PlacementContext {
                        face: event.face.clone(),
                        cursor: DVec3::new(
                            f64::from(event.cursor_x),
                            f64::from(event.cursor_y),
                            f64::from(event.cursor_z),
                        ),
                        block_clicked: block_pos,
                        block_pos: offset_pos,
                        level: &state.0.world,
                        dimension: Dimension::Overworld,
                        player_rotation: rot,
                        default_placement_state: block_state,
                    };

                    // Try to replace the block from the offset calculated
                    if !curr_state.can_be_replaced(placement_context.clone()) {
                        // If the block cannot be replaced, try to replace the block adjacent to the face clicked
                        offset_pos = block_pos + event.face.get_normal();

                        let Ok(curr_state) =
                            state.0.world.get_block(offset_pos, Dimension::Overworld)
                        else {
                            error!("Can't get block at {}", offset_pos);
                            continue 'ev_loop;
                        };

                        if !curr_state.can_be_replaced(placement_context.clone()) {
                            if let Err(err) = conn.send_packet_ref(&ack_packet) {
                                error!("Failed to send block change ack packet: {:?}", err);
                                continue 'ev_loop;
                            }

                            if let Err(err) = conn.send_packet(BlockUpdate {
                                location: NetworkPosition {
                                    x: offset_pos.pos.x,
                                    y: offset_pos.pos.y as i16,
                                    z: offset_pos.pos.z,
                                },
                                block_state_id: curr_state.to_varint(),
                            }) {
                                error!("Failed to send block update packet to player: {err}");
                                continue 'ev_loop;
                            }

                            continue 'ev_loop;
                        }

                        placement_context.block_pos = offset_pos;
                    }

                    let mut placed_blocks = block_state.get_placement_state(placement_context);

                    if placed_blocks.place_original {
                        placed_blocks.blocks.insert(offset_pos, block_state);
                    }

                    for (block_pos, block_state) in placed_blocks.blocks.iter() {
                        state
                            .0
                            .world
                            .set_block(*block_pos, Dimension::Overworld, *block_state)
                            .unwrap_or_else(|_| error!("Failed to update block {}", block_pos));

                        if let Some(kind) = create_block_entity(&state, *block_pos, *block_state) {
                            block_entity_placed.write(BlockEntityPlaced {
                                player: entity,
                                position: *block_pos,
                                kind,
                            });
                        }

                        let block_chunk = block_pos.chunk();
                        world_change.write(WorldChange {
                            chunk: Some(block_chunk),
                        });

                        let chunk_packet = BlockUpdate {
                            location: NetworkPosition {
                                x: block_pos.pos.x,
                                y: block_pos.pos.y as i16,
                                z: block_pos.pos.z,
                            },
                            block_state_id: block_state.to_varint(),
                        };

                        let (block_chunk_x, block_chunk_z) = (block_chunk.x(), block_chunk.z());
                        let render_distance = state.0.config.chunk_render_distance as i32;
                        for (_, conn, _, _, pos, _, _) in query.iter() {
                            let chunk = pos.chunk();
                            let (chunk_x, chunk_z) = (chunk.x(), chunk.z());

                            // Only send block update if the player is within the render distance of the block being updated
                            if (block_chunk_x - chunk_x).abs() <= render_distance
                                && (block_chunk_z - chunk_z).abs() <= render_distance
                                && let Err(err) = conn.send_packet_ref(&chunk_packet)
                            {
                                error!("Failed to send block update packet: {:?}", err);
                            }
                        }
                    }
                }

                if let Err(err) = conn.send_packet_ref(&ack_packet) {
                    error!("Failed to send block change ack packet: {:?}", err);
                    continue 'ev_loop;
                }
            }
            1 => {
                trace!("Offhand block placement not implemented");
            }
            _ => {
                debug!("Invalid hand");
            }
        }
    }
}

/// Creates default block entity data for a freshly placed block, if its
/// blockstate has an associated block entity type we support. Returns the
/// kind created, so the caller can notify type-specific systems.
fn create_block_entity(
    state: &GlobalStateResource,
    block_pos: BlockPos,
    block_state: BlockStateId,
) -> Option<BlockEntityKind> {
    let protocol_id = temper_data::blocks::block_entity_type_for_state(block_state.raw())?;

    let name = temper_data::blocks::BLOCK_ENTITY_TYPE_NAMES
        .get(protocol_id as usize)
        .copied()
        .unwrap_or_default();

    let (kind, blob) = match name {
        "sign" | "hanging_sign" => (BlockEntityKind::Sign, SignBlockEntity::default().to_blob()),
        other => {
            trace!("No block entity support for {other} at {block_pos}");
            return None;
        }
    };

    let blob = match blob {
        Ok(blob) => blob,
        Err(err) => {
            error!("Failed to serialize block entity at {block_pos}: {err}");
            return None;
        }
    };

    let Ok(chunk) = state
        .0
        .world
        .get_chunk(block_pos.chunk(), Dimension::Overworld)
    else {
        error!("Failed to get chunk for block entity at {block_pos}");
        return None;
    };

    chunk.block_entities.insert(
        block_pos.chunk_block_pos(),
        BlockEntityData {
            kind,
            protocol_id,
            blob,
        },
    );
    chunk.mark_dirty();

    Some(kind)
}
