mod xoroshiro;

use crate::pos::BlockPos;
use std::borrow::Borrow;
use std::ops::RangeInclusive;

pub use xoroshiro::*;

/// Supplies various random functions for structs. Based on `net.minecraft.util.RandomSource`.
pub trait RandomSource {
    fn fork(&mut self) -> Self
    where
        Self: Sized;
    fn fork_positional(&mut self) -> impl PositionalRandom<Self>
    where
        Self: Sized;

    fn next_u32(&mut self) -> u32;
    fn next_u32_bounded(&mut self, limit: u32) -> u32;
    fn next_u32_range(&mut self, range: RangeInclusive<u32>) -> u32 {
        self.next_u32_bounded(range.end() - range.start() + 1) + range.start()
    }

    fn next_u64(&mut self) -> u64;
    fn next_bool(&mut self) -> bool;
    fn next_f32(&mut self) -> f32;
    fn next_f64(&mut self) -> f64;

    fn consume_count(&mut self, count: usize);
}

/// Provides a way to spawn various RandomSources from input seeds.
pub trait PositionalRandom<R: RandomSource> {
    fn spawn_at(&self, x: i32, y: i32, z: i32) -> R;
    fn spawn_from_seed(&self, seed: u64) -> R;
    fn spawn_from_hash<S: Borrow<str>>(&self, hash: S) -> R;

    fn spawn_at_pos(&self, pos: BlockPos) -> R {
        self.spawn_at(pos.pos.x, pos.pos.y, pos.pos.z)
    }
}

/// Converts the low and high bits (u64s) to a single u128
#[inline]
fn u64_to_u128(low: u64, high: u64) -> u128 {
    u128::from(high) << 64 | u128::from(low)
}

/// Returns the lowest 64 bits of a u128
#[inline]
fn u128_low(seed: u128) -> u64 {
    seed as u64
}

/// Returns the highest 64 bits of a u128
#[inline]
fn u128_high(seed: u128) -> u64 {
    (seed >> 64) as u64
}

fn mix_stafford13(val: u64) -> u64 {
    let val = (val ^ val >> 30).wrapping_mul(0xbf58476d1ce4e5b9);
    let val = (val ^ val >> 27).wrapping_mul(0x94d049bb133111eb);
    val ^ val >> 31
}

#[inline]
#[allow(dead_code)]
fn upgrade_seed_unmixed(seed: u64) -> u128 {
    let low = seed ^ 0x6a09e667f3bcc909;
    let high = low.wrapping_add(0x9e3779b97f4a7c15);
    u64_to_u128(low, high)
}

#[inline]
fn upgrade_seed_mixed(seed: u64) -> u128 {
    let low = seed ^ 0x6a09e667f3bcc909;
    let high = low.wrapping_add(0x9e3779b97f4a7c15);
    u64_to_u128(mix_stafford13(low), mix_stafford13(high))
}

#[inline]
fn seed_from_hash(hash: &str) -> u128 {
    let hash_bytes = md5::compute(hash).0;
    let [low, high]: [[u8; 8]; 2] = unsafe { std::mem::transmute(hash_bytes) };
    u64_to_u128(u64::from_be_bytes(low), u64::from_be_bytes(high))
}

#[inline]
fn seed_from_pos(x: i32, y: i32, z: i32) -> u64 {
    let seed = x.wrapping_mul(3129871) as u64 ^ z.wrapping_mul(116129781) as u64 ^ y as u64;
    let seed = seed
        .wrapping_mul(seed)
        .wrapping_mul(42317861)
        .wrapping_add(seed.wrapping_mul(11));
    seed >> 16
}
