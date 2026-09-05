use bevy_ecs::prelude::Res;
use bevy_math::DVec3;
use rand::prelude::SliceRandom;
use std::time::Duration;
use temper_components::player::position::Position;
use temper_core::block_state_id::BlockStateId;
use temper_core::dimension::Dimension::Overworld;
use temper_core::pos::{ChunkBlockPos, ChunkPos};
use temper_macros::match_block;
use temper_state::GlobalStateResource;
use temper_world::RefChunk;
use tracing::{error, info, trace};

const SPAWN_CENTER: (i32, i32) = (8, 8);
const SPAWN_SEARCH_BUDGET: Duration = Duration::from_millis(30);
const MAX_SPAWN_CHUNK_RADIUS: i32 = 128;

// Basically the easiest way to have available spawn positions is to just
// generate a bunch and chuck them in a queue that can be pulled from
pub fn generate_spawn_positions(state: Res<GlobalStateResource>) {
    let start = std::time::Instant::now();

    if state.0.spawn_positions.is_full() {
        return;
    }

    let mut found_coords = 0;
    let center_chunk =
        Position::new(f64::from(SPAWN_CENTER.0), 0.0, f64::from(SPAWN_CENTER.1)).chunk();

    for radius in 0..=MAX_SPAWN_CHUNK_RADIUS {
        if start.elapsed() > SPAWN_SEARCH_BUDGET {
            info!("Generating spawn positions is taking longer than expected.");
            return;
        }

        let mut chunks = chunk_ring(center_chunk, radius).collect::<Vec<_>>();
        chunks.shuffle(&mut rand::rng());

        for (chunk_x, chunk_z) in chunks {
            if state.0.spawn_positions.is_full() {
                trace!(
                    "Finished generating {} spawn positions in {:.2} ms",
                    found_coords,
                    start.elapsed().as_secs_f32() * 1000.0
                );
                return;
            }

            if start.elapsed() > SPAWN_SEARCH_BUDGET {
                info!("Generating spawn positions is taking longer than expected.");
                return;
            }

            let chunk_pos = ChunkPos::new(chunk_x, chunk_z);

            let chunk = state
                .0
                .world
                .get_or_generate_chunk(chunk_pos, Overworld)
                .expect("Failed to generate chunk");

            found_coords += enqueue_spawn_positions_from_chunk(&state, chunk_pos, &chunk);
        }
    }

    error!(
        "Failed to find enough spawn positions within {MAX_SPAWN_CHUNK_RADIUS} chunks of spawn. Falling back to (0, 100, 0)."
    );
    let _ = state.0.spawn_positions.push((0.0, 100.0, 0.0));
}

fn enqueue_spawn_positions_from_chunk(
    state: &GlobalStateResource,
    chunk_pos: ChunkPos,
    chunk: &RefChunk<'_>,
) -> usize {
    let mut found = 0;

    let mut coords = Vec::with_capacity(256);
    for x in 0..16 {
        for z in 0..16 {
            coords.push((x, z));
        }
    }
    coords.shuffle(&mut rand::rng());

    for (x, z) in coords {
        if state.0.spawn_positions.is_full() {
            return found;
        }

        if let Some(position) = spawn_position_for_column(chunk_pos, chunk, x, z) {
            state
                .0
                .spawn_positions
                .push(position.xyz())
                .expect("Cannot push to queue");
            found += 1;
        }
    }

    found
}

fn spawn_position_for_column(
    chunk_pos: ChunkPos,
    chunk: &RefChunk<'_>,
    local_x: u8,
    local_z: u8,
) -> Option<Position> {
    let height = chunk
        .heightmaps
        .motion_blocking
        .get_height(local_x, local_z);

    let candidate_block = chunk.get_block(ChunkBlockPos::new(local_x, height, local_z));

    if !is_valid_spawn_surface(candidate_block) {
        return None;
    }

    let world_x = chunk_pos.pos.x + i32::from(local_x);
    let world_z = chunk_pos.pos.y + i32::from(local_z);
    let surface_pos = Position::new(f64::from(world_x), f64::from(height), f64::from(world_z));

    Some(spawn_position_above_surface(surface_pos))
}

fn is_valid_spawn_surface(block: BlockStateId) -> bool {
    !(match_block!("air", block)
        || match_block!("void_air", block)
        || match_block!("water", block)
        || match_block!("lava", block))
}

fn spawn_position_above_surface(surface_pos: Position) -> Position {
    (*surface_pos + DVec3::new(0.5, 1.0, 0.5)).into()
}

fn chunk_ring(center: ChunkPos, radius: i32) -> impl Iterator<Item = (i32, i32)> {
    let center_x = center.x();
    let center_z = center.z();

    (-radius..=radius).flat_map(move |x| {
        (-radius..=radius).filter_map(move |z| {
            (x.abs().max(z.abs()) == radius).then_some((center_x + x, center_z + z))
        })
    })
}

#[cfg(test)]
mod tests {
    use bevy_ecs::schedule::Schedule;
    use temper_macros::block;
    use temper_state::create_test_state;

    use super::*;

    #[test]
    fn spawn_surface_rejects_air_and_fluids() {
        assert!(!is_valid_spawn_surface(block!("air")));
        assert!(!is_valid_spawn_surface(block!("void_air")));
        assert!(!is_valid_spawn_surface(block!("water", {level: 0})));
        assert!(!is_valid_spawn_surface(block!("lava", {level: 0})));
        assert!(is_valid_spawn_surface(
            block!("grass_block", {snowy: false})
        ));
        assert!(is_valid_spawn_surface(block!("dirt")));
    }

    #[test]
    fn spawn_position_sits_above_surface() {
        let spawn_pos = spawn_position_above_surface(Position::new(5.0, 32.0, 10.0));

        assert_eq!(spawn_pos.xyz(), (5.5, 33.0, 10.5));
    }

    #[test]
    fn chunk_ring_starts_at_center_chunk() {
        let chunks = chunk_ring(ChunkPos::new(3, -2), 0).collect::<Vec<_>>();

        assert_eq!(chunks, vec![(3, -2)]);
    }

    #[test]
    fn chunk_ring_visits_only_requested_radius() {
        let chunks = chunk_ring(ChunkPos::new(0, 0), 1).collect::<Vec<_>>();

        assert_eq!(chunks.len(), 8);
        assert!(chunks.contains(&(-1, -1)));
        assert!(chunks.contains(&(1, 1)));
        assert!(!chunks.contains(&(0, 0)));
    }

    #[test]
    fn generates_positions_into_queue() {
        let (state, _temp_dir) = create_test_state();
        let mut world = bevy_ecs::world::World::new();
        let mut schedule = Schedule::default();

        world.insert_resource(state.clone());
        schedule.add_systems(generate_spawn_positions);
        schedule.run(&mut world);

        assert!(!state.0.spawn_positions.is_empty());
    }
}
