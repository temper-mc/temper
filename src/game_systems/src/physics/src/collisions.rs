use bevy_ecs::message::MessageWriter;
use bevy_ecs::prelude::{DetectChanges, Entity, Has, Query, Res, With};
use bevy_ecs::world::Mut;
use bevy_math::Vec3A;
use bevy_math::bounding::BoundingVolume;
use bevy_math::ops::floor;
use std::cmp::max;
use std::time::Instant;
use temper_components::bounds::CollisionBounds;
use temper_components::entity_identity::Identity;
use temper_components::player::gamemode::{GameMode, GameModeComponent};
use temper_components::player::grounded::OnGround;
use temper_components::player::old_position::OldPosition;
use temper_components::player::player_marker::PlayerMarker;
use temper_components::player::position::Position;
use temper_components::player::velocity::Velocity;
use temper_core::block_properties;
use temper_core::dimension::Dimension;
use temper_core::pos::{BlockPos, ChunkPos};
use temper_data::attributes::Attribute;
use temper_entities::PhysicalRegistry;
use temper_entities::components::Baby;
use temper_entities::components::EntityMetadata;
use temper_entities::markers::HasCollisions;
use temper_messages::damage::{DamageEvent, DamageSource};
use temper_messages::entity_update::SendEntityUpdate;
use temper_physics::GRAVITY_ACCELERATION;
use temper_state::{GlobalState, GlobalStateResource};
use temper_world::RefChunk;
use tracing::{debug, error, info, trace};

type CollisionQueryItem<'a> = (
    Entity,
    Option<&'a OldPosition>,
    Mut<'a, Position>,
    Option<&'a EntityMetadata>,
    Option<&'a mut Velocity>,
    Option<&'a CollisionBounds>,
    Option<&'a GameModeComponent>,
    Has<Baby>,
    Mut<'a, OnGround>,
    &'a Identity,
    Has<PlayerMarker>,
);

/// This whole thing is a complete mess since players have unreliable and largely unused velocity
/// but do have client-side collision prediction and mobs have velocities but no client-side collisions.
pub fn handle(
    query: Query<CollisionQueryItem, With<HasCollisions>>,
    mut entity_updates_writer: MessageWriter<SendEntityUpdate>,
    mut damage_writer: MessageWriter<DamageEvent>,
    state: Res<GlobalStateResource>,
    registry: Res<PhysicalRegistry>,
) {
    for (
        eid,
        old_pos,
        mut pos,
        metadata,
        vel,
        collision_bounds,
        gamemode,
        is_baby,
        mut grounded,
        identity,
        is_player,
    ) in query
    {
        if let Some(gamemode) = gamemode
            && matches!(gamemode.0, GameMode::Spectator)
        {
            continue;
        }
        if pos.is_changed() {
            let start = Instant::now();
            let static_hitbox = if let Some(bounds) = collision_bounds {
                bounds
            } else {
                if let Some(meta) = metadata
                    && let Some(physical) = registry.get_or_adult(meta.protocol_id(), is_baby)
                {
                    &physical.bounding_box
                } else {
                    debug!(
                        "Entity {} has no collision bounds and no physical definition, skipping collision check",
                        eid
                    );
                    continue;
                }
            };

            // Players and non-players have opposite update orderings:
            // - non-players already had Velocity applied, so sweep from `pos - vel -> pos`
            // - players only have their last observed step, so predict `pos -> pos + delta`.
            // I'm aware that just guessing a player's next position isn't a good idea, but I don't
            // have any better ideas that isn't "Do player physics serverside"
            let (sweep_start_pos, sweep_end_pos, sweep_delta) = if !is_player {
                if let Some(vel) = &vel {
                    let end_pos = pos.as_vec3a();
                    let delta = ***vel;
                    (end_pos - delta, end_pos, delta)
                } else {
                    error!(
                        "Entity {} has no velocity and is not a player, skipping collision check",
                        eid
                    );
                    continue;
                }
            } else if let Some(old_pos) = old_pos {
                let current_pos = pos.as_vec3a();
                let delta = (**pos - **old_pos).as_vec3a();
                (current_pos, current_pos + delta, delta)
            } else {
                error!(
                    "Player {} has no old position, skipping collision check",
                    eid
                );
                continue;
            };

            let current_hitbox = static_hitbox.translated_by(sweep_start_pos);
            let next_hitbox = static_hitbox.translated_by(sweep_end_pos);

            // Bail out on the raw delta rather than comparing the translated hitboxes -
            // at typical world coordinates the hitboxes can round to the same f32 value
            // through floating point cancellation even when a small but real delta exists,
            // which would wrongly skip collision detection on exactly the ticks (tiny final
            // approach to a block) where it matters most
            if sweep_delta == Vec3A::ZERO {
                continue;
            }

            // Any block we could hit has to be in here. Pad the scan by a small amount so an entity
            // resting exactly flush against a block face (e.g. landing perfectly on y=72.0
            // when the ground block spans [71, 72)) still has that block included - floor/ceil
            // on the raw bounds would otherwise miss it entirely since it never actually
            // crosses the boundary, just touches it
            let max_hitbox = next_hitbox.merge(&current_hitbox);
            const SCAN_PADDING: f32 = 1e-4;
            let scan_min = max_hitbox.min - Vec3A::splat(SCAN_PADDING);
            let scan_max = max_hitbox.max + Vec3A::splat(SCAN_PADDING);

            // Behold, some unhinged bullshit to find the first block hit. Don't ask me how it works,
            // only god and the bottle of Jack Daniel's that got me through this knows, I certainly don't
            let moving_min0 = current_hitbox.min;
            let moving_max0 = current_hitbox.max;
            let mut best_hit: Option<(f32, usize, BlockPos)> = None;
            let mut possible_hits = 0;
            let mut loaded_chunk: Option<(ChunkPos, RefChunk<'_>)> = None;

            'scan: for x in scan_min.x.floor() as i32..scan_max.x.ceil() as i32 {
                for y in scan_min.y.floor() as i32..scan_max.y.ceil() as i32 {
                    for z in scan_min.z.floor() as i32..scan_max.z.ceil() as i32 {
                        let block_pos = BlockPos::of(x, y, z);
                        let chunk_pos = block_pos.chunk();

                        let needs_chunk_load = loaded_chunk
                            .as_ref()
                            .is_none_or(|(loaded_pos, _)| *loaded_pos != chunk_pos);
                        if needs_chunk_load {
                            let chunk = state
                                .0
                                .world
                                .get_or_generate_chunk(chunk_pos, Dimension::Overworld)
                                .expect("Failed to load or generate chunk");
                            loaded_chunk = Some((chunk_pos, chunk));
                        }

                        let block_state = loaded_chunk
                            .as_ref()
                            .expect("Chunk should be loaded")
                            .1
                            .get_block(block_pos.chunk_block_pos());
                        if !block_properties::is_solid(block_state) {
                            continue;
                        }

                        possible_hits += 1;

                        let target_min = block_pos.pos.as_vec3a();
                        let target_max = target_min + Vec3A::ONE;

                        if let Some((entry_time, axis)) = sweep_aabb(
                            moving_min0,
                            moving_max0,
                            target_min,
                            target_max,
                            sweep_delta,
                        ) && best_hit.is_none_or(|(best_time, _, _)| entry_time < best_time)
                        {
                            best_hit = Some((entry_time, axis, block_pos));

                            if entry_time == 0.0 {
                                break 'scan;
                            }
                        }
                    }
                }
            }

            if let Some((_entry_time, axis, collided_block)) = best_hit {
                // Ground/ceiling contact needs to be recorded regardless of whether we can
                // correct the entity's position - players are client-authoritative so we
                // never touch their Position below, but OnGround still has to reflect that
                // their reported movement did hit something on the y-axis.
                let is_ground_hit = axis == 1 && sweep_delta.y < 0.0;

                if is_ground_hit {
                    grounded.currently_grounded = true;
                }

                if axis != 1 || grounded.just_landed() {
                    trace!(
                        "{} Hit block at {}, {} blocks checked, took {:?}",
                        identity.name.as_ref().expect("Entity has no name"),
                        collided_block,
                        possible_hits,
                        Instant::now() - start
                    );
                    if axis == 1 || sweep_delta.y < 0.0 {
                        let fall_vel = vel
                            .as_ref()
                            .map(|v| v.y)
                            .filter(|&y| y != 0.0)
                            .unwrap_or(sweep_delta.y);

                        // Check for downwards movement
                        if fall_vel < 0.0 && grounded.just_landed() {
                            let vy = fall_vel.abs();
                            let g = GRAVITY_ACCELERATION.y.abs();

                            let fall_dist = if g > 0.0 { vy.powi(2) / (2.0 * g) } else { vy };

                            let (sfd, fdm) = if let Some(meta) = metadata {
                                // For Mobs, get Attribute and there the value
                                let vanilla_data = meta.vanilla_data();

                                let safe_fall_distance = vanilla_data
                                    .get_attribute("safe_fall_distance")
                                    .expect("Failed to get 'safe_fall_distance' attribute from entity metadata");

                                let fall_damage_mult = vanilla_data
                                    .get_attribute("fall_damage_multiplier")
                                    .expect("Failed to get 'fall_damage_multiplier' attribute from entity metadata");

                                (safe_fall_distance as f32, fall_damage_mult as f32)
                            } else {
                                // For Players, get the default Attribute
                                // (gets it similar to how mobs get it, just a longer way there)
                                let safe_fall_distance = Attribute::from_name("safe_fall_distance")
                                    .expect("Failed to find default attribute definition for 'safe_fall_distance'");

                                let fall_damage_mult = Attribute::from_name("fall_damage_multiplier")
                                    .expect("Failed to find default attribute definition for 'fall_damage_multiplier'");

                                (
                                    safe_fall_distance.default_value as f32,
                                    fall_damage_mult.default_value as f32,
                                )
                            };

                            // Calculate Fall-Damage
                            let fall_damage = max(0, floor((fall_dist - sfd) * fdm) as i32);

                            // Only emit DamageEvent if damage was actually taken
                            if fall_damage > 0 {
                                damage_writer.write(DamageEvent {
                                    target: eid,
                                    source: DamageSource::Fall { last_ground: None },
                                    damage: fall_damage as f32,
                                    knockback: None,
                                    knockback_source: None,
                                });

                                info!("Entity {:?} fell taking {} damage", eid, fall_damage);
                            } else {
                                trace!("Entity {:?} fell without taking damage", eid);
                            }
                        }
                    }
                }

                // If it's not a player we need to set their position to not be colliding with the block.
                // If we ever get around to doing an anticheat system we can do server-side player
                // collision resolution but for now we just let the client handle it.
                if !is_player {
                    // Only the axis that actually collided needs correcting - the other two
                    // should keep their full intended movement (this is what lets an entity
                    // slide along a wall instead of stuttering to a stop diagonally).
                    let mut resolved = pos.as_vec3a();

                    // Snap the blocking axis directly to the block's boundary rather than
                    // trusting the interpolated `entry_time`, so we land exactly flush against
                    // it instead of a hair short/long due to the division above.
                    let block_min = collided_block.pos.as_vec3a();
                    resolved[axis] = if sweep_delta[axis] > 0.0 {
                        block_min[axis] - static_hitbox.max[axis]
                    } else {
                        block_min[axis] + 1.0 - static_hitbox.min[axis]
                    };

                    pos.coords = resolved.as_dvec3();

                    // Zero out velocity on the axis we actually collided on so we don't just
                    // re-collide (and re-correct) again next tick.
                    if let Some(mut vel) = vel {
                        vel.vec[axis] = 0.0;
                    }
                }
            }

            entity_updates_writer.write(SendEntityUpdate(eid));
        }
    }
}

pub fn is_solid_block(state: &GlobalState, pos: BlockPos) -> bool {
    let chunk_coordinates = pos.chunk();
    let block_state = state
        .world
        .get_or_generate_mut(chunk_coordinates, Dimension::Overworld)
        .expect("Failed to load or generate chunk")
        .get_block(pos.chunk_block_pos());

    block_properties::is_solid(block_state)
}

/// Analytic swept-AABB test: given a moving box at `t=0` (`moving_min0`/`moving_max0`)
/// travelling by `velocity` over `t` in `[0, 1]`, finds the exact time it first touches the
/// static `target` box, along with which axis (0=x, 1=y, 2=z) the contact happened on.
/// Returns `None` if the boxes never touch during the sweep.
fn sweep_aabb(
    moving_min0: Vec3A,
    moving_max0: Vec3A,
    target_min: Vec3A,
    target_max: Vec3A,
    velocity: Vec3A,
) -> Option<(f32, usize)> {
    let mut entry = [f32::NEG_INFINITY; 3];
    let mut exit = [f32::INFINITY; 3];
    let mut any_moving_axis_constrained = false;

    for axis in 0..3 {
        let v = velocity[axis];
        if v > 0.0 {
            if moving_min0[axis] >= target_max[axis] {
                // Already flush against (or past) the target's far face and moving further
                // away - e.g. standing on top of a block and jumping off it.
            }
            entry[axis] = (target_min[axis] - moving_max0[axis]) / v;
            exit[axis] = (target_max[axis] - moving_min0[axis]) / v;
            any_moving_axis_constrained = true;
        } else if v < 0.0 {
            if moving_max0[axis] <= target_min[axis] {
                // Same thing for near face
                continue;
            }
            entry[axis] = (target_max[axis] - moving_min0[axis]) / v;
            exit[axis] = (target_min[axis] - moving_max0[axis]) / v;
            any_moving_axis_constrained = true;
        } else if moving_max0[axis] <= target_min[axis] || moving_min0[axis] >= target_max[axis] {
            // Never overlapping on this axis regardless of travel on the other two.
            return None;
        }
    }

    if !any_moving_axis_constrained {
        return None;
    }

    let entry_time = entry[0].max(entry[1]).max(entry[2]);
    let exit_time = exit[0].min(exit[1]).min(exit[2]);

    if entry_time > exit_time || entry_time > 1.0 || exit_time < 0.0 {
        return None;
    }

    let axis = if entry[1] >= entry[0] && entry[1] >= entry[2] {
        1
    } else if entry[0] >= entry[2] {
        0
    } else {
        2
    };

    Some((entry_time.max(0.0), axis))
}
