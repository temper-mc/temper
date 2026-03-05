use bevy_ecs::prelude::*;
use temper_components::player::grounded::OnGround;
use temper_components::player::player_identity::PlayerIdentity;
use temper_components::player::position::Position;
use temper_components::player::velocity::Velocity;
use temper_core::pos::BlockPos;
use temper_entities::markers::entity_types::Pig;
use temper_state::GlobalStateResource;

/// Pig walk speed in blocks per tick.
const PIG_WALK_SPEED: f32 = 0.1;

/// Jump impulse matching Minecraft's standard jump velocity (blocks/tick).
/// With GRAVITY_ACCELERATION = -0.08 blocks/tick², this peaks at ~1.1 blocks.
const JUMP_IMPULSE: f32 = 0.42;

/// Recompute the path every N ticks.
const REPATH_INTERVAL: u32 = 40;

/// Max A* node expansions per repath.
const MAX_PATH_NODES: usize = 100;

/// Per-pig AI state: cached path and repath cooldown.
#[derive(Component, Default)]
pub struct PigAI {
    path: Vec<BlockPos>,
    waypoint: usize,
    repath_cooldown: u32,
}

pub fn tick_pig(
    mut commands: Commands,
    mut pigs: Query<
        (
            Entity,
            &Position,
            &mut Velocity,
            &OnGround,
            Option<&mut PigAI>,
        ),
        With<Pig>,
    >,
    players: Query<&Position, With<PlayerIdentity>>,
    state: Res<GlobalStateResource>,
) {
    for (entity, pig_pos, mut velocity, grounded, ai) in pigs.iter_mut() {
        let mut ai = match ai {
            Some(ai) => ai,
            None => {
                commands.entity(entity).insert(PigAI::default());
                continue;
            }
        };

        ai.repath_cooldown = ai.repath_cooldown.saturating_sub(1);

        let current_block = pos_to_block(pig_pos);

        // Advance waypoint when the pig reaches it (same X/Z block)
        if let Some(next) = ai.path.get(ai.waypoint) {
            if next.pos.x == current_block.pos.x && next.pos.z == current_block.pos.z {
                ai.waypoint += 1;
            }
        }

        // Recompute path if cooldown expired or path exhausted
        if ai.repath_cooldown == 0 || ai.waypoint >= ai.path.len() {
            let Some(target_pos) = players
                .iter()
                .min_by_key(|p| ordered_float(pig_pos.coords.distance_squared(p.coords)))
            else {
                stop(&mut velocity);
                continue;
            };

            let goal = pos_to_block(target_pos);
            ai.path = pathfinding::find_path(&state.0.world, current_block, goal, MAX_PATH_NODES)
                .map(|p| p.nodes)
                .unwrap_or_default();
            ai.waypoint = 1; // node 0 is the current position
            ai.repath_cooldown = REPATH_INTERVAL;
        }

        let Some(next) = ai.path.get(ai.waypoint) else {
            stop(&mut velocity);
            continue;
        };

        // Jump if the next waypoint is 1 block above and the pig is on the ground.
        // We rely on OnGround (set/reset by the collision system each tick) rather than
        // a fractional-Y heuristic, which would fire mid-air and cause infinite flying.
        if next.pos.y > current_block.pos.y && grounded.0 {
            velocity.vec.y = JUMP_IMPULSE;
        }

        // Steer horizontally toward the center of the next waypoint block
        let dx = (next.pos.x as f64 + 0.5 - pig_pos.x) as f32;
        let dz = (next.pos.z as f64 + 0.5 - pig_pos.z) as f32;
        let len = (dx * dx + dz * dz).sqrt();

        if len > 0.1 {
            velocity.vec.x = (dx / len) * PIG_WALK_SPEED;
            velocity.vec.z = (dz / len) * PIG_WALK_SPEED;
        } else {
            velocity.vec.x = 0.0;
            velocity.vec.z = 0.0;
        }
    }
}

fn stop(velocity: &mut Velocity) {
    velocity.vec.x = 0.0;
    velocity.vec.z = 0.0;
}

fn pos_to_block(pos: &Position) -> BlockPos {
    // Add a small epsilon before flooring to avoid floating-point edge cases where
    // the entity is at exactly the block surface (e.g. y=64.9999... instead of 65.0),
    // which would incorrectly place the entity one block too low.
    const EPSILON: f64 = 1e-4;
    BlockPos::of(
        pos.x.floor() as i32,
        (pos.y + EPSILON).floor() as i32,
        pos.z.floor() as i32,
    )
}

fn ordered_float(v: f64) -> u64 {
    v.to_bits()
}
