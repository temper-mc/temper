use bevy_ecs::prelude::{Entity, MessageWriter, Query, Res};
use temper_codec::net_types::network_position::NetworkPosition;
use temper_codec::net_types::var_int::VarInt;
use temper_components::bounds::CollisionBounds;
use temper_components::player::position::Position;
use temper_core::pos::BlockPos;
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::outgoing::block_change_ack::BlockChangeAck;
use temper_protocol::outgoing::block_update::BlockUpdate;
use temper_protocol::PlaceBlockReceiver;
use temper_state::GlobalStateResource;
use tracing::{debug, error, trace};

use bevy_math::DVec3;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::str::FromStr;
use temper_components::player::rotation::Rotation;
use temper_config::server_config::get_global_config;
use temper_core::block_state_id::{BlockStateId, ITEM_TO_BLOCK_MAPPING};
use temper_core::dimension::Dimension;
use temper_core::mq;
use temper_inventories::hotbar::Hotbar;
use temper_inventories::inventory::Inventory;
use temper_macros::match_block;
use temper_messages::world_change::WorldChange;
use temper_text::{Color, NamedColor, TextComponentBuilder};

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
    )>,
    pos_q: Query<(&Position, &CollisionBounds)>,
    mut world_change: MessageWriter<WorldChange>,
) {
    'ev_loop: for (event, eid) in receiver.0.try_iter() {
        let Ok((entity, conn, inventory, hotbar, pos, rot)) = query.get(eid) else {
            debug!("Could not get connection for entity {:?}", eid);
            continue;
        };
        if !state.0.players.is_connected(entity) {
            trace!("Entity {:?} is not connected", entity);
            continue;
        }
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
                    let offset_pos = block_pos
                        + match event.face.0 {
                            0 => (0, -1, 0),
                            1 => (0, 1, 0),
                            2 => (0, 0, -1),
                            3 => (0, 0, 1),
                            4 => (-1, 0, 0),
                            5 => (1, 0, 0),
                            _ => (0, 0, 0),
                        };

                    let block_clicked = {
                        let chunk = state
                            .0
                            .world
                            .get_or_generate_chunk(block_pos.chunk(), Dimension::Overworld)
                            .expect("Failed to load or generate chunk");
                        chunk.get_block(block_pos.chunk_block_pos())
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
                                    offset_pos.pos.x as f64,
                                    offset_pos.pos.y as f64,
                                    offset_pos.pos.z as f64,
                                ),
                            )
                        })
                    };

                    if does_collide {
                        trace!("Block placement collided with entity");
                        continue 'ev_loop;
                    }

                    let block_at_pos = {
                        let chunk = state
                            .0
                            .world
                            .get_or_generate_chunk(offset_pos.chunk(), Dimension::Overworld)
                            .expect("Failed to load or generate chunk");
                        chunk.get_block(offset_pos.chunk_block_pos())
                    };

                    if !(match_block!("water", block_at_pos)
                        || match_block!("lava", block_at_pos)
                        || match_block!("air", block_at_pos))
                    {
                        debug!(
                            "Block placement failed because the block at the target position is not replaceable"
                        );
                        continue 'ev_loop;
                    }

                    let (remove_item, placed_block) = block_placing::place_item(
                        state.0.clone(),
                        block_placing::BlockPlaceContext {
                            block_clicked,
                            block_position: offset_pos,
                            face_clicked: match event.face.0 {
                                0 => block_placing::BlockFace::Bottom,
                                1 => block_placing::BlockFace::Top,
                                2 => block_placing::BlockFace::North,
                                3 => block_placing::BlockFace::South,
                                4 => block_placing::BlockFace::West,
                                5 => block_placing::BlockFace::East,
                                _ => {
                                    debug!("Invalid block face");
                                    continue 'ev_loop;
                                }
                            },
                            click_position: DVec3::new(
                                event.cursor_x as f64,
                                event.cursor_y as f64,
                                event.cursor_z as f64,
                            ),
                            player_position: *pos,
                            player_rotation: *rot,
                        },
                        item_id,
                    );

                    if let Some(placed_block) = placed_block {
                        world_change.write(WorldChange {
                            chunk: Some(offset_pos.chunk()),
                        });
                        let ack_packet = BlockChangeAck {
                            sequence: event.sequence,
                        };

                        let chunk_packet = BlockUpdate {
                            location: NetworkPosition {
                                x: offset_pos.pos.x,
                                y: offset_pos.pos.y as i16,
                                z: offset_pos.pos.z,
                            },
                            block_state_id: placed_block.to_varint(),
                        };

                        if let Err(err) = conn.send_packet_ref(&ack_packet) {
                            error!("Failed to send block change ack packet: {:?}", err);
                            continue 'ev_loop;
                        }

                        let offset_chunk = offset_pos.chunk();
                        let (offset_chunk_x, offset_chunk_z) = (offset_chunk.x(), offset_chunk.z());
                        let render_distance = get_global_config().chunk_render_distance as i32;
                        for (_, conn, _, _, pos, rot) in query.iter() {
                            let chunk = pos.chunk();
                            let (chunk_x, chunk_z) = (chunk.x(), chunk.z());

                            // Only send block update if the player is within the render distance of the block being updated
                            if (offset_chunk_x - chunk_x).abs() <= render_distance
                                && (offset_chunk_z - chunk_z).abs() <= render_distance
                                && let Err(err) = conn.send_packet_ref(&chunk_packet)
                            {
                                error!("Failed to send block update packet: {:?}", err);
                            }
                        }
                    }
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
