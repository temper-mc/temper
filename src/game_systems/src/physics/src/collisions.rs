use bevy_ecs::message::MessageWriter;
use bevy_ecs::prelude::{DetectChanges, Entity, Has, Query, Res, With};
use bevy_ecs::world::Mut;
use bevy_math::IVec3;
use bevy_math::bounding::{Aabb3d, BoundingVolume};
use std::cmp::max;
use tracing::info;
use temper_components::bounds::CollisionBounds;
use temper_components::game_id::GameID;
use temper_components::player::grounded::OnGround;
use temper_components::player::position::Position;
use temper_components::player::velocity::Velocity;
use temper_core::block_properties;
use temper_core::dimension::Dimension;
use temper_core::pos::{ChunkBlockPos, ChunkPos};
use temper_data::generated::attributes::Attribute;
use temper_entities::PhysicalRegistry;
use temper_entities::components::Baby;
use temper_entities::components::EntityMetadata;
use temper_messages::damage::{DamageEvent, DamageSource};
use temper_messages::entity_update::SendEntityUpdate;
use temper_physics::GRAVITY_ACCELERATION;
use temper_state::{GlobalState, GlobalStateResource};

type CollisionQueryItem<'a> = (
    Entity,
    Mut<'a, Velocity>,
    Mut<'a, Position>,
    Option<&'a EntityMetadata>,   // For mobs
    Option<&'a CollisionBounds>,  // For players
    Has<Baby>,
    Mut<'a, OnGround>,
    &'a GameID,
);

pub fn handle(
    query: Query<CollisionQueryItem>,
    mut writer: MessageWriter<SendEntityUpdate>,
    mut dmg_writer: MessageWriter<DamageEvent>,
    state: Res<GlobalStateResource>,
    registry: Res<PhysicalRegistry>,
) {
    for (eid, mut vel, mut pos, metadata, bounds, is_baby, mut grounded, _) in query {
        // Extract the bounding box depending on whether it's a player or mob
        let bounding_box = if let Some(collision_bounds) = bounds {
            // Convert player's custom CollisionBounds into an Aabb3d
            Aabb3d {
                min: bevy_math::Vec3A::new(
                    collision_bounds.x_offset_start as f32,
                    collision_bounds.y_offset_start as f32,
                    collision_bounds.z_offset_start as f32,
                ),
                max: bevy_math::Vec3A::new(
                    collision_bounds.x_offset_end as f32,
                    collision_bounds.y_offset_end as f32,
                    collision_bounds.z_offset_end as f32,
                ),
            }
        } else if let Some(meta) = metadata {
            // Use PhysicalRegistry for mobs based on protocol ID and baby status
            let Some(physical) = registry.get_or_adult(meta.protocol_id(), is_baby) else {
                continue;
            };
            *physical.bounding_box
        } else {
            continue;
        };

        if pos.is_changed() || vel.is_changed() {
            grounded.0 = false;

            if vel.vec.y < 0.0 {
                let old_pos = pos.coords - vel.as_dvec3();
                let feet_y = f64::from(bounding_box.min.y) + pos.coords.y;
                let old_feet_y = f64::from(bounding_box.min.y) + old_pos.y;

                let min_x = (f64::from(bounding_box.min.x) + pos.coords.x)
                    .min(f64::from(bounding_box.min.x) + old_pos.x)
                    .floor() as i32;
                let max_x = (f64::from(bounding_box.max.x) + pos.coords.x)
                    .max(f64::from(bounding_box.max.x) + old_pos.x)
                    .floor() as i32;
                let min_z = (f64::from(bounding_box.min.z) + pos.coords.z)
                    .min(f64::from(bounding_box.min.z) + old_pos.z)
                    .floor() as i32;
                let max_z = (f64::from(bounding_box.max.z) + pos.coords.z)
                    .max(f64::from(bounding_box.max.z) + old_pos.z)
                    .floor() as i32;

                let min_y = feet_y.floor() as i32;
                let max_y = old_feet_y.ceil() as i32 - 1;

                'floor_crossing: for y in (min_y..=max_y).rev() {
                    let surface_y = f64::from(y + 1);
                    if old_feet_y < surface_y || feet_y > surface_y {
                        continue;
                    }

                    for x in min_x..=max_x {
                        for z in min_z..=max_z {
                            if is_solid_block(&state.0, IVec3::new(x, y, z)) {
                                pos.coords.y = surface_y - f64::from(bounding_box.min.y);
                                vel.vec.y = 0.0;
                                grounded.0 = true;
                                break 'floor_crossing;
                            }
                        }
                    }
                }
            }

            let old_pos = pos.coords - vel.as_dvec3();

            let mut collided = false;
            let mut hit_blocks = vec![];

            // Build hitboxes using old_pos (where we started) and pos.coords (where we landed)
            let current_hitbox = Aabb3d {
                min: bounding_box.min + old_pos.as_vec3a(),
                max: bounding_box.max + old_pos.as_vec3a(),
            };

            let next_hitbox = Aabb3d {
                min: bounding_box.min + pos.coords.as_vec3a(),
                max: bounding_box.max + pos.coords.as_vec3a(),
            };

            let merged_hitbox = current_hitbox.merge(&next_hitbox);
            let min_block_pos = merged_hitbox.min;
            let max_block_pos = merged_hitbox.max;

            info!(
                "Checking entity {:?} | Old Pos: {:?}, New Pos: {:?} | Merged Block Range: min({:?}) to max({:?})",
                eid, old_pos, pos.coords, min_block_pos, max_block_pos
            );

            // Check each block in the bounding box for solidity
            for x in min_block_pos.x.floor() as i32..=max_block_pos.x.floor() as i32 {
                for y in min_block_pos.y.floor() as i32..=max_block_pos.y.floor() as i32 {
                    for z in min_block_pos.z.floor() as i32..=max_block_pos.z.floor() as i32 {
                        let block_pos = IVec3::new(x, y, z);
                        if is_solid_block(&state.0, block_pos) {
                            info!("-> Found solid block at {:?}!", block_pos);
                            collided = true;
                            hit_blocks.push(block_pos);
                        }
                    }
                }
            }

            let impact_speed = vel.vec.y.abs();

            if collided {
                hit_blocks.sort_by(|a, b| {
                    let dist_a = (a.as_dvec3() - pos.coords).length_squared();
                    let dist_b = (b.as_dvec3() - pos.coords).length_squared();
                    dist_a.partial_cmp(&dist_b).unwrap()
                });
                let first_hit = hit_blocks.first().expect("At least one hit block expected");

                let entity_min = bounding_box.min + pos.coords.as_vec3a();
                let entity_max = bounding_box.max + pos.coords.as_vec3a();
                let block_min = first_hit.as_vec3a();
                let block_max = (first_hit + IVec3::ONE).as_vec3a();

                let ox_pos = entity_max.x - block_min.x;
                let ox_neg = block_max.x - entity_min.x;
                let oy_pos = entity_max.y - block_min.y;
                let oy_neg = block_max.y - entity_min.y;
                let oz_pos = entity_max.z - block_min.z;
                let oz_neg = block_max.z - entity_min.z;

                info!("Collided");

                // Trigger fall damage if the entity was falling downwards significantly
                if impact_speed > 0.0 && (oy_neg >= 0.0 || oy_pos >= 0.0) {
                    info!("Entity fell with impact speed: {}", impact_speed);
                    let fall_height = (impact_speed * impact_speed) / (2.0 * GRAVITY_ACCELERATION.y as f32);

                    let fall_dmg = max(
                        0,
                        ((fall_height - Attribute::SAFE_FALL_DISTANCE.default_value as f32)
                            * Attribute::FALL_DAMAGE_MULTIPLIER.default_value as f32)
                            as i32,
                    );

                    if fall_dmg > 0 {
                        info!("Falldamage: {}", fall_dmg);
                        let msg = DamageEvent {
                            target: eid,
                            source: DamageSource::Fall { last_ground: None },
                            damage: fall_dmg as f32,
                            knockback: None,
                            knockback_source: None,
                        };
                        dmg_writer.write(msg);
                    }
                }

                // Use >= 0.0 to catch touching/surface contact as well as deep penetration
                if ox_pos >= 0.0
                    && ox_neg >= 0.0
                    && oy_pos >= 0.0
                    && oy_neg >= 0.0
                    && oz_pos >= 0.0
                    && oz_neg >= 0.0
                {
                    let mx = ox_pos.min(ox_neg);
                    let my = oy_pos.min(oy_neg);
                    let mz = oz_pos.min(oz_neg);

                    if mx <= my && mx <= mz {
                        let push = if ox_pos < ox_neg { -ox_pos } else { ox_neg };
                        pos.coords.x += f64::from(push);
                        vel.vec.x = 0.0;
                    } else if my <= mx && my <= mz {
                        let push = if oy_pos < oy_neg { -oy_pos } else { oy_neg };
                        pos.coords.y += f64::from(push);
                        vel.vec.y = 0.0;
                        if oy_neg <= oy_pos {
                            grounded.0 = true;
                        }
                    } else {
                        let push = if oz_pos < oz_neg { -oz_pos } else { oz_neg };
                        pos.coords.z += f64::from(push);
                        vel.vec.z = 0.0;
                    }
                }
            }

            if !grounded.0 && vel.vec.y <= 0.0 {
                let feet_y = f64::from(bounding_box.min.y) + pos.coords.y;
                let floor_block_y = (feet_y - 1e-3).floor() as i32;
                let cx = pos.coords.x.floor() as i32;
                let cz = pos.coords.z.floor() as i32;
                if is_solid_block(&state.0, IVec3::new(cx, floor_block_y, cz)) {
                    let surface_y = f64::from(floor_block_y + 1);
                    if (feet_y - surface_y).abs() < 0.05 {
                        pos.coords.y = surface_y - f64::from(bounding_box.min.y);
                        vel.vec.y = 0.0;
                        grounded.0 = true;
                    }
                }
            }

            writer.write(SendEntityUpdate(eid));
        }
    }
}

pub fn is_solid_block(state: &GlobalState, pos: IVec3) -> bool {
    let chunk_coordinates = ChunkPos::from(pos.as_dvec3());
    let block_state = state
        .world
        .get_or_generate_mut(chunk_coordinates, Dimension::Overworld)
        .expect("Failed to load or generate chunk")
        .get_block(ChunkBlockPos::from(pos));

    block_properties::is_solid(block_state)
}