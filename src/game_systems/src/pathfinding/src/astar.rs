use std::collections::BinaryHeap;

use arrayvec::ArrayVec;
use rustc_hash::FxHashMap;
use temper_components::physical::PhysicalProperties;
use temper_core::block_state_id::BlockStateId;
use temper_core::pos::BlockPos;
use temper_world::Dimension;

use crate::cost::{IMPASSABLE, block_penalty};

/// A path from start to goal, expressed as block positions (feet position).
pub struct Path {
    pub nodes: Vec<BlockPos>,
}

// Internal node in the priority queue.
// Ord is inverted so BinaryHeap acts as a min-heap on estimated_cost.
#[derive(Eq, PartialEq)]
struct Candidate {
    estimated_cost: i32, // f = g + h
    real_cost: i32,      // g
    pos: BlockPos,
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.estimated_cost.cmp(&self.estimated_cost)
    }
}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Entity dimensions for pathfinding, computed from PhysicalProperties.
#[derive(Clone, Copy)]
struct EntityDimensions {
    /// Height in blocks (rounded up). E.g. pig=1, zombie=2, enderman=3.
    height_blocks: u8,
    /// Half-width in blocks (rounded up). E.g. 0.45 -> 1 block.
    /// TODO: Use this for wider entities like spiders that occupy multiple blocks horizontally.
    #[allow(dead_code)]
    half_width_blocks: u8,
    // TODO: Use fire_immune from PhysicalProperties to avoid lava/fire penalties
    // for entities like blazes, striders, etc.
}

impl EntityDimensions {
    fn from_physical(props: &PhysicalProperties) -> Self {
        Self {
            height_blocks: props.bounding_box.height().ceil() as u8,
            half_width_blocks: (props.bounding_box.width() / 2.0).ceil() as u8,
        }
    }
}

/// Find a path for a land mob using weighted A*.
///
/// `start` and `goal` are the block positions of the mob's feet.
/// `physical` provides the entity's dimensions for collision checking.
/// Returns `None` if no path is found within `max_nodes` node expansions.
pub fn find_path(
    world: &temper_world::World,
    start: BlockPos,
    goal: BlockPos,
    max_nodes: usize,
    physical: &PhysicalProperties,
) -> Option<Path> {
    let dims = EntityDimensions::from_physical(physical);

    if start == goal {
        return Some(Path { nodes: vec![goal] });
    }

    let mut open: BinaryHeap<Candidate> = BinaryHeap::new();
    let mut g_score: FxHashMap<BlockPos, i32> = FxHashMap::default();
    let mut came_from: FxHashMap<BlockPos, BlockPos> = FxHashMap::default();

    g_score.insert(start, 0);
    open.push(Candidate {
        estimated_cost: heuristic(start, goal),
        real_cost: 0,
        pos: start,
    });

    let mut iterations = 0;
    while let Some(Candidate { real_cost, pos, .. }) = open.pop() {
        if iterations >= max_nodes {
            break;
        }
        iterations += 1;

        if pos == goal {
            return Some(reconstruct_path(came_from, pos, start));
        }

        if real_cost > *g_score.get(&pos).unwrap_or(&i32::MAX) {
            continue;
        }

        for (neighbor, move_cost) in neighbors(world, pos, dims) {
            let tentative_g = real_cost + move_cost;

            if g_score
                .get(&neighbor)
                .is_none_or(|&best| tentative_g < best)
            {
                g_score.insert(neighbor, tentative_g);
                came_from.insert(neighbor, pos);
                open.push(Candidate {
                    estimated_cost: tentative_g + heuristic(neighbor, goal),
                    real_cost: tentative_g,
                    pos: neighbor,
                });
            }
        }
    }

    None
}

/// Heuristic using octile distance (accounts for diagonal movement).
/// Returns cost estimate scaled to match movement costs (cardinal=10, diagonal=14).
fn heuristic(a: BlockPos, b: BlockPos) -> i32 {
    let dx = (a.pos.x - b.pos.x).abs();
    let dy = (a.pos.y - b.pos.y).abs();
    let dz = (a.pos.z - b.pos.z).abs();

    // Octile distance on XZ plane + vertical distance
    let min_xz = dx.min(dz);
    let max_xz = dx.max(dz);

    // Diagonal moves cost 14, cardinal moves cost 10
    // min_xz diagonals + (max_xz - min_xz) cardinals + dy vertical
    min_xz * COST_DIAGONAL + (max_xz - min_xz) * COST_CARDINAL + dy * COST_CARDINAL
}

fn reconstruct_path(
    came_from: FxHashMap<BlockPos, BlockPos>,
    target: BlockPos,
    start: BlockPos,
) -> Path {
    let mut current = target;
    let mut nodes = vec![current];
    while current != start {
        current = came_from[&current];
        nodes.push(current);
    }
    nodes.reverse();
    Path { nodes }
}

/// Cardinal directions (cost multiplier: 10).
const CARDINALS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// Diagonal directions (cost multiplier: 14, approximation of 10 * sqrt(2)).
const DIAGONALS: [(i32, i32); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

/// Base movement cost for cardinal directions.
const COST_CARDINAL: i32 = 10;

/// Base movement cost for diagonal directions (approx. 10 * sqrt(2)).
const COST_DIAGONAL: i32 = 14;

/// Extra cost for stepping up one block.
const COST_STEP_UP: i32 = 10;

/// Maximum number of neighbors per node (4 cardinal + 4 diagonal).
const MAX_NEIGHBORS: usize = 8;

/// Generate passable neighbors for a land mob.
/// Handles flat walking, stepping up 1 block, and stepping down 1 block.
/// Supports both cardinal and diagonal movement.
fn neighbors(
    world: &temper_world::World,
    pos: BlockPos,
    dims: EntityDimensions,
) -> ArrayVec<(BlockPos, i32), MAX_NEIGHBORS> {
    let mut result = ArrayVec::new();

    // Cardinal directions
    for (dx, dz) in CARDINALS {
        if let Some((dest, cost)) = try_move(world, pos, dx, dz, COST_CARDINAL, dims) {
            result.push((dest, cost));
        }
    }

    // Diagonal directions (require both adjacent cardinal directions to be passable)
    for (dx, dz) in DIAGONALS {
        // Check corner-cutting: both adjacent cells must be passable at feet level
        let side1_passable =
            block_penalty(get_block(world, pos.pos.x + dx, pos.pos.y, pos.pos.z)) != IMPASSABLE;
        let side2_passable =
            block_penalty(get_block(world, pos.pos.x, pos.pos.y, pos.pos.z + dz)) != IMPASSABLE;

        if side1_passable
            && side2_passable
            && let Some((dest, cost)) = try_move(world, pos, dx, dz, COST_DIAGONAL, dims)
        {
            result.push((dest, cost));
        }
    }

    result
}

/// Try to move from `pos` in direction `(dx, dz)` with base cost `base_cost`.
/// Returns the destination and total movement cost if the move is valid.
fn try_move(
    world: &temper_world::World,
    pos: BlockPos,
    dx: i32,
    dz: i32,
    base_cost: i32,
    dims: EntityDimensions,
) -> Option<(BlockPos, i32)> {
    let nx = pos.pos.x + dx;
    let nz = pos.pos.z + dz;

    // Walk flat
    if let Some(terrain_cost) = can_stand_at(world, nx, pos.pos.y, nz, dims) {
        return Some((BlockPos::of(nx, pos.pos.y, nz), base_cost + terrain_cost));
    }

    // Step up 1 block — need space above current position for the full entity height
    if is_clear_above(world, pos.pos.x, pos.pos.y, pos.pos.z, dims.height_blocks)
        && let Some(terrain_cost) = can_stand_at(world, nx, pos.pos.y + 1, nz, dims)
    {
        return Some((
            BlockPos::of(nx, pos.pos.y + 1, nz),
            base_cost + COST_STEP_UP + terrain_cost,
        ));
    }

    // Step down 1 block — neighbor column must be open at current height
    if block_penalty(get_block(world, nx, pos.pos.y, nz)) != IMPASSABLE
        && let Some(terrain_cost) = can_stand_at(world, nx, pos.pos.y - 1, nz, dims)
    {
        return Some((
            BlockPos::of(nx, pos.pos.y - 1, nz),
            base_cost + terrain_cost,
        ));
    }

    None
}

/// Check if an entity can stand with feet at (x, y, z):
/// - solid block at (x, y-1, z) as floor
/// - passable blocks for the full body height at (x, y, z) to (x, y+height-1, z)
///
/// Returns `Some(terrain_cost)` if valid, `None` if not.
fn can_stand_at(
    world: &temper_world::World,
    x: i32,
    y: i32,
    z: i32,
    dims: EntityDimensions,
) -> Option<i32> {
    // Check for solid floor
    if block_penalty(get_block(world, x, y - 1, z)) != IMPASSABLE {
        return None; // no solid floor
    }

    // Check all blocks occupied by the body
    let mut total_penalty = 0;
    for dy in 0..i32::from(dims.height_blocks) {
        let body_penalty = block_penalty(get_block(world, x, y + dy, z));
        if body_penalty == IMPASSABLE {
            return None;
        }
        total_penalty += body_penalty.max(0);
    }

    Some(total_penalty)
}

/// Check if there's enough vertical clearance above a position.
/// Used for step-up checks where the entity needs headroom.
fn is_clear_above(world: &temper_world::World, x: i32, y: i32, z: i32, height: u8) -> bool {
    for dy in 1..=i32::from(height) {
        if block_penalty(get_block(world, x, y + dy, z)) == IMPASSABLE {
            return false;
        }
    }
    true
}

fn get_block(world: &temper_world::World, x: i32, y: i32, z: i32) -> BlockStateId {
    let pos = BlockPos::of(x, y, z);
    // Only read from cache — never generate chunks during pathfinding
    world
        .get_cache()
        .get(&(pos.chunk(), Dimension::Overworld))
        .map(|chunk| chunk.get_block(pos.chunk_block_pos()))
        .unwrap_or_default() // unloaded chunk = air; mob won't path there (no solid floor)
}
