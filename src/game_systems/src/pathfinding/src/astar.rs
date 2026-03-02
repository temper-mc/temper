use std::collections::{BinaryHeap, HashMap};

use temper_core::block_state_id::BlockStateId;
use temper_core::pos::BlockPos;
use temper_world::Dimension;

use crate::cost::{block_penalty, IMPASSABLE};

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
    pos: (i32, i32, i32),
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

/// Find a path for a 1-block-tall land mob using weighted A*.
///
/// `start` and `goal` are the block positions of the mob's feet.
/// Returns `None` if no path is found within `max_nodes` node expansions.
pub fn find_path(
    world: &temper_world::World,
    start: BlockPos,
    goal: BlockPos,
    max_nodes: usize,
) -> Option<Path> {
    let start_key = to_key(start);
    let goal_key = to_key(goal);

    if start_key == goal_key {
        return Some(Path { nodes: vec![goal] });
    }

    let mut open: BinaryHeap<Candidate> = BinaryHeap::new();
    let mut g_score: HashMap<(i32, i32, i32), i32> = HashMap::new();
    let mut came_from: HashMap<(i32, i32, i32), (i32, i32, i32)> = HashMap::new();

    g_score.insert(start_key, 0);
    open.push(Candidate {
        estimated_cost: heuristic(start, goal),
        real_cost: 0,
        pos: start_key,
    });

    let mut iterations = 0;
    while let Some(Candidate { real_cost, pos, .. }) = open.pop() {
        if iterations >= max_nodes {
            break;
        }
        iterations += 1;

        if pos == goal_key {
            return Some(reconstruct_path(came_from, pos, start_key));
        }

        if real_cost > *g_score.get(&pos).unwrap_or(&i32::MAX) {
            continue;
        }

        let current = from_key(pos);
        for (neighbor, move_cost) in neighbors(world, current) {
            let neighbor_key = to_key(neighbor);
            let tentative_g = real_cost + move_cost;

            if g_score
                .get(&neighbor_key)
                .is_none_or(|&best| tentative_g < best)
            {
                g_score.insert(neighbor_key, tentative_g);
                came_from.insert(neighbor_key, pos);
                open.push(Candidate {
                    estimated_cost: tentative_g + heuristic(neighbor, goal),
                    real_cost: tentative_g,
                    pos: neighbor_key,
                });
            }
        }
    }

    None
}

fn to_key(pos: BlockPos) -> (i32, i32, i32) {
    (pos.pos.x, pos.pos.y, pos.pos.z)
}

fn from_key((x, y, z): (i32, i32, i32)) -> BlockPos {
    BlockPos::of(x, y, z)
}

fn heuristic(a: BlockPos, b: BlockPos) -> i32 {
    (a.pos.x - b.pos.x).abs() + (a.pos.y - b.pos.y).abs() + (a.pos.z - b.pos.z).abs()
}

fn reconstruct_path(
    came_from: HashMap<(i32, i32, i32), (i32, i32, i32)>,
    target: (i32, i32, i32),
    start: (i32, i32, i32),
) -> Path {
    let mut current = target;
    let mut nodes = vec![from_key(current)];
    while current != start {
        current = came_from[&current];
        nodes.push(from_key(current));
    }
    nodes.reverse();
    Path { nodes }
}

const CARDINALS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// Generate passable neighbors for a 1-block-tall land mob (e.g. pig, height=0.9).
/// Handles flat walking, stepping up 1 block, and stepping down 1 block.
fn neighbors(world: &temper_world::World, pos: BlockPos) -> Vec<(BlockPos, i32)> {
    let mut result = Vec::with_capacity(5);

    for (dx, dz) in CARDINALS {
        let nx = pos.pos.x + dx;
        let nz = pos.pos.z + dz;

        // Walk flat
        if let Some(cost) = can_stand_at(world, nx, pos.pos.y, nz) {
            result.push((BlockPos::of(nx, pos.pos.y, nz), cost + 1));
            continue;
        }

        // Step up 1 block — need the block directly above current feet to be clear
        if block_penalty(get_block(world, pos.pos.x, pos.pos.y + 1, pos.pos.z)) != IMPASSABLE {
            if let Some(cost) = can_stand_at(world, nx, pos.pos.y + 1, nz) {
                result.push((BlockPos::of(nx, pos.pos.y + 1, nz), cost + 2));
                continue;
            }
        }

        // Step down 1 block — neighbor column must be open at current height
        if block_penalty(get_block(world, nx, pos.pos.y, nz)) != IMPASSABLE {
            if let Some(cost) = can_stand_at(world, nx, pos.pos.y - 1, nz) {
                result.push((BlockPos::of(nx, pos.pos.y - 1, nz), cost + 1));
            }
        }
    }

    result
}

/// Check if a 1-block-tall mob can stand with feet at (x, y, z):
/// - solid block at (x, y-1, z) as floor
/// - passable at (x, y, z) for the body
///
/// Returns `Some(terrain_cost)` if valid, `None` if not.
fn can_stand_at(world: &temper_world::World, x: i32, y: i32, z: i32) -> Option<i32> {
    if block_penalty(get_block(world, x, y - 1, z)) != IMPASSABLE {
        return None; // no solid floor
    }

    let body_penalty = block_penalty(get_block(world, x, y, z));
    if body_penalty == IMPASSABLE {
        return None;
    }

    Some(body_penalty.max(0))
}

fn get_block(world: &temper_world::World, x: i32, y: i32, z: i32) -> BlockStateId {
    let pos = BlockPos::of(x, y, z);
    // Only read from cache — never generate chunks during pathfinding
    world
        .get_cache()
        .get(&(pos.chunk(), Dimension::Overworld))
        .map(|chunk| chunk.get_block(pos.chunk_block_pos()))
        .unwrap_or_default() // unloaded chunk = air; pig won't path there (no solid floor)
}
