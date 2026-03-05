use bevy_ecs::message::MessageWriter;
use bevy_ecs::prelude::{DetectChanges, Entity, Has, Query, Res, With};
use bevy_ecs::world::Mut;
use bevy_math::bounding::{Aabb3d, BoundingVolume};
use bevy_math::IVec3;
use temper_components::player::grounded::OnGround;
use temper_components::player::position::Position;
use temper_components::player::velocity::Velocity;
use temper_core::block_state_id::BlockStateId;
use temper_core::dimension::Dimension;
use temper_core::pos::{ChunkBlockPos, ChunkPos};
use temper_entities::PhysicalRegistry;
use temper_entities::components::Baby;
use temper_entities::components::EntityMetadata;
use temper_entities::markers::HasCollisions;
use temper_macros::match_block;
use temper_messages::entity_update::SendEntityUpdate;
use temper_state::{GlobalState, GlobalStateResource};

type CollisionQueryItem<'a> = (
    Entity,
    Mut<'a, Velocity>,
    Mut<'a, Position>,
    &'a EntityMetadata,
    Has<Baby>,
    Mut<'a, OnGround>,
);

pub fn handle(
    query: Query<CollisionQueryItem, With<HasCollisions>>,
    mut writer: MessageWriter<SendEntityUpdate>,
    state: Res<GlobalStateResource>,
    registry: Res<PhysicalRegistry>,
) {
    for (eid, mut vel, mut pos, metadata, is_baby, mut grounded) in query {
        let Some(physical) = registry.get_or_adult(metadata.protocol_id(), is_baby) else {
            continue;
        };
        if pos.is_changed() || vel.is_changed() {
            // Reset grounded only when the entity is actually moving.
            // When grounded and at rest, gravity is skipped → vel/pos unchanged → this block
            // is skipped → grounded keeps its true value, preventing spurious falling.
            // When the entity jumps or falls, vel/pos change → grounded resets to false here,
            // then gets set back to true only when the MTV Y-resolution detects a landing.
            grounded.0 = false;
            // Figure out where the entity is going to be next tick
            let next_pos = pos.coords.as_vec3a() + **vel;
            let mut collided = false;
            let mut hit_blocks = vec![];

            // Merge the current and next bounding boxes to get the full area the entity will occupy
            // This helps catch fast-moving entities that might skip through thin blocks
            // At really high speeds this will create a very large bounding box, so further optimizations may be needed
            let current_hitbox = Aabb3d {
                min: physical.bounding_box.min + pos.coords.as_vec3a(),
                max: physical.bounding_box.max + pos.coords.as_vec3a(),
            };

            let next_hitbox = Aabb3d {
                min: physical.bounding_box.min + next_pos,
                max: physical.bounding_box.max + next_pos,
            };

            let merged_hitbox = current_hitbox.merge(&next_hitbox);

            // Get the block positions that the entity's bounding box will occupy
            let min_block_pos = merged_hitbox.min;
            let max_block_pos = merged_hitbox.max;

            // Check each block in the bounding box for solidity
            for x in min_block_pos.x.floor() as i32..=max_block_pos.x.floor() as i32 {
                for y in min_block_pos.y.floor() as i32..=max_block_pos.y.floor() as i32 {
                    for z in min_block_pos.z.floor() as i32..=max_block_pos.z.floor() as i32 {
                        let block_pos = IVec3::new(x, y, z);
                        if is_solid_block(&state.0, block_pos) {
                            collided = true;
                            hit_blocks.push(block_pos);
                        }
                    }
                }
            }
            // Resolve collisions using Minimum Translation Vector (MTV):
            // compute the penetration depth on each axis and push out along the
            // smallest one, zeroing only that velocity component. This preserves
            // jump velocity when hitting a wall horizontally.
            if collided {
                hit_blocks.sort_by(|a, b| {
                    let dist_a = (a.as_dvec3() - pos.coords).length_squared();
                    let dist_b = (b.as_dvec3() - pos.coords).length_squared();
                    dist_a.partial_cmp(&dist_b).unwrap()
                });
                let first_hit = hit_blocks.first().expect("At least one hit block expected");

                let entity_min = physical.bounding_box.min + pos.coords.as_vec3a();
                let entity_max = physical.bounding_box.max + pos.coords.as_vec3a();
                let block_min = first_hit.as_vec3a();
                let block_max = (first_hit + IVec3::ONE).as_vec3a();

                // Penetration depth on each axis from both sides
                let ox_pos = entity_max.x - block_min.x; // entity entering from -X
                let ox_neg = block_max.x - entity_min.x; // entity entering from +X
                let oy_pos = entity_max.y - block_min.y; // entity entering from below
                let oy_neg = block_max.y - entity_min.y; // entity entering from above
                let oz_pos = entity_max.z - block_min.z; // entity entering from -Z
                let oz_neg = block_max.z - entity_min.z; // entity entering from +Z

                // Only resolve if there is real penetration on all three axes
                if ox_pos > 0.0
                    && ox_neg > 0.0
                    && oy_pos > 0.0
                    && oy_neg > 0.0
                    && oz_pos > 0.0
                    && oz_neg > 0.0
                {
                    let mx = ox_pos.min(ox_neg);
                    let my = oy_pos.min(oy_neg);
                    let mz = oz_pos.min(oz_neg);

                    if mx <= my && mx <= mz {
                        let push = if ox_pos < ox_neg { -ox_pos } else { ox_neg };
                        pos.coords.x += push as f64;
                        vel.vec.x = 0.0;
                    } else if my <= mx && my <= mz {
                        let push = if oy_pos < oy_neg { -oy_pos } else { oy_neg };
                        pos.coords.y += push as f64;
                        vel.vec.y = 0.0;
                        if oy_neg <= oy_pos {
                            // Entity came from above: it's landing on the block
                            grounded.0 = true;
                        }
                    } else {
                        let push = if oz_pos < oz_neg { -oz_pos } else { oz_neg };
                        pos.coords.z += push as f64;
                        vel.vec.z = 0.0;
                    }
                }
            }

            // Floor contact check: catches the "exactly at surface" case that the MTV
            // misses when vel.y = 0. This happens when the entity moves horizontally
            // while standing: the merged hitbox uses floor(65.0) = 65, so block y=64
            // is excluded, no collision fires, and grounded stays false. We check the
            // block just below the entity's feet explicitly.
            if !grounded.0 && vel.vec.y <= 0.0 {
                let feet_y = physical.bounding_box.min.y as f64 + pos.coords.y;
                let floor_block_y = (feet_y - 1e-3).floor() as i32;
                let cx = pos.coords.x.floor() as i32;
                let cz = pos.coords.z.floor() as i32;
                if is_solid_block(&state.0, IVec3::new(cx, floor_block_y, cz)) {
                    let surface_y = (floor_block_y + 1) as f64;
                    if (feet_y - surface_y).abs() < 0.05 {
                        pos.coords.y = surface_y - physical.bounding_box.min.y as f64;
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

    !match_block!("air", block_state)
        && !match_block!("void_air", block_state)
        && !match_block!("water", block_state)
        && !match_block!("air", block_state)
}
