use std::collections::{HashMap, HashSet};
use std::sync::{Condvar, Mutex};
use temper_core::dimension::Dimension;
use temper_core::pos::{ChunkPos, FullChunkPos};

pub type ChunkKey = FullChunkPos;

/// Coordinates chunk interest, generation, cache eviction, and persistence.
///
/// The cache and the generation scheduler still do their own work, but every
/// operation that can make a chunk unavailable passes through this lifecycle
/// coordinator first.
#[derive(Default)]
pub struct ChunkLifecycle {
    state: Mutex<LifecycleState>,
    changed: Condvar,
}

#[derive(Default)]
struct LifecycleState {
    chunks: HashMap<ChunkKey, ChunkStatus>,
    owners: HashMap<u64, HashSet<ChunkKey>>,
}

#[derive(Default)]
struct ChunkStatus {
    interests: usize,
    active_accesses: usize,
    active_generations: usize,
    operation: ChunkOperation,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ChunkOperation {
    #[default]
    Idle,
    Loading,
    Writing,
    Evicting,
    Saving,
}

impl ChunkLifecycle {
    /// Atomically replace the chunks held by one player/consumer.
    pub fn set_owner_chunks(&self, owner: u64, chunks: impl IntoIterator<Item = ChunkKey>) {
        let chunks = chunks.into_iter().collect::<HashSet<_>>();
        let mut state = self.lock();
        while chunks.iter().any(|key| {
            state.chunks.get(key).is_some_and(|status| {
                matches!(
                    status.operation,
                    ChunkOperation::Evicting | ChunkOperation::Writing
                )
            })
        }) {
            state = self.wait(state);
        }
        Self::replace_owner(&mut state, owner, chunks);
    }

    /// Replace all active owners from a fresh snapshot, dropping disconnected
    /// consumers at the same time.
    pub fn replace_owners(&self, owners: HashMap<u64, HashSet<ChunkKey>>) {
        let mut state = self.lock();
        while owners.values().flatten().any(|key| {
            state.chunks.get(key).is_some_and(|status| {
                matches!(
                    status.operation,
                    ChunkOperation::Evicting | ChunkOperation::Writing
                )
            })
        }) {
            state = self.wait(state);
        }
        let removed = state
            .owners
            .keys()
            .filter(|owner| !owners.contains_key(owner))
            .copied()
            .collect::<Vec<_>>();
        for owner in removed {
            Self::replace_owner(&mut state, owner, HashSet::new());
        }
        for (owner, chunks) in owners {
            Self::replace_owner(&mut state, owner, chunks);
        }
    }

    #[cfg(test)]
    fn is_interested(&self, key: ChunkKey) -> bool {
        self.lock()
            .chunks
            .get(&key)
            .is_some_and(|status| status.interests > 0)
    }

    pub fn wait_for_pending_saves(&self) {
        let mut state = self.lock();
        while state.chunks.values().any(|status| {
            matches!(
                status.operation,
                ChunkOperation::Saving | ChunkOperation::Evicting | ChunkOperation::Writing
            )
        }) {
            state = self.wait(state);
        }
    }

    /// Mark the target and its generation neighbourhood as active before any
    /// scheduler job can start. Eviction observes the same lock, so it cannot
    /// slip between the pending-job check and generation registration.
    pub(crate) fn begin_generation(
        &self,
        dimension: Dimension,
        pos: ChunkPos,
        radius: i32,
    ) -> GenerationLease<'_> {
        let side = (radius as usize).saturating_mul(2).saturating_add(1);
        let mut keys = Vec::with_capacity(side.saturating_mul(side));
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                keys.push(FullChunkPos::of(pos.x() + dx, pos.z() + dz, dimension));
            }
        }

        let mut state = self.lock();
        loop {
            let blocked = keys.iter().any(|key| {
                state
                    .chunks
                    .get(key)
                    .is_some_and(|status| status.operation != ChunkOperation::Idle)
            });
            if !blocked {
                for key in &keys {
                    state.chunks.entry(*key).or_default().active_generations += 1;
                }
                break;
            }
            state = self.wait(state);
        }

        GenerationLease {
            lifecycle: self,
            keys,
        }
    }

    /// Keep a cache/storage lookup from racing an eviction or asynchronous
    /// save. The returned guard is dropped as soon as the cache reference has
    /// been acquired; DashMap's own reference then protects the entry.
    pub(crate) fn begin_access(&self, key: ChunkKey) -> AccessLease<'_> {
        let mut state = self.lock();
        loop {
            let blocked = state
                .chunks
                .get(&key)
                .is_some_and(|status| status.operation != ChunkOperation::Idle);
            if !blocked {
                state.chunks.entry(key).or_default().active_accesses += 1;
                break;
            }
            state = self.wait(state);
        }

        AccessLease {
            lifecycle: self,
            key,
        }
    }

    /// Reserve a storage write that is not part of an eviction save (for
    /// example, world import). Readers and generation wait until the write is
    /// complete, and writes are rejected while the chunk has active owners.
    pub(crate) fn begin_storage_write(&self, key: ChunkKey) -> Option<StorageWriteLease<'_>> {
        let mut state = self.lock();
        let status = state.chunks.entry(key).or_default();
        if status.interests > 0
            || status.active_accesses > 0
            || status.active_generations > 0
            || status.operation != ChunkOperation::Idle
        {
            return None;
        }
        status.operation = ChunkOperation::Writing;
        Some(StorageWriteLease {
            lifecycle: self,
            key,
        })
    }

    /// Serialize cache misses for a key. The caller must recheck the cache
    /// after acquiring this lease because another reader may have populated it.
    pub(crate) fn begin_cache_load(&self, key: ChunkKey) -> CacheLoadLease<'_> {
        let mut state = self.lock();
        loop {
            let operation = state
                .chunks
                .get(&key)
                .map_or(ChunkOperation::Idle, |status| status.operation);
            if operation == ChunkOperation::Idle {
                state.chunks.entry(key).or_default().operation = ChunkOperation::Loading;
                break;
            }
            state = self.wait(state);
        }

        CacheLoadLease {
            lifecycle: self,
            key,
        }
    }

    /// Reserve a chunk for eviction. A dirty chunk remains unavailable until
    /// its asynchronous write finishes.
    pub(crate) fn begin_eviction(&self, key: ChunkKey) -> bool {
        let mut state = self.lock();
        let status = state.chunks.entry(key).or_default();
        if status.interests > 0
            || status.active_accesses > 0
            || status.active_generations > 0
            || status.operation != ChunkOperation::Idle
        {
            return false;
        }
        status.operation = ChunkOperation::Evicting;
        true
    }

    pub(crate) fn eviction_removed(&self, key: ChunkKey, needs_save: bool) {
        let mut state = self.lock();
        if let Some(status) = state.chunks.get_mut(&key) {
            debug_assert_eq!(status.operation, ChunkOperation::Evicting);
            if needs_save {
                status.operation = ChunkOperation::Saving;
            }
        }
        // Keep clean evictions reserved until entity cleanup and generator
        // bookkeeping have finished.
    }

    pub(crate) fn eviction_finished(&self, key: ChunkKey) {
        self.release_operation(key, ChunkOperation::Evicting);
    }

    pub(crate) fn save_finished(&self, key: ChunkKey) {
        self.release_operation(key, ChunkOperation::Saving);
    }

    pub(crate) fn eviction_cancelled(&self, key: ChunkKey) {
        self.release_operation(key, ChunkOperation::Evicting);
    }

    fn release_operation(&self, key: ChunkKey, operation: ChunkOperation) {
        let mut state = self.lock();
        if let Some(status) = state.chunks.get_mut(&key) {
            debug_assert_eq!(status.operation, operation);
            if status.operation == operation {
                status.operation = ChunkOperation::Idle;
            }
        }
        Self::remove_if_idle(&mut state, key);
        self.changed.notify_all();
    }

    fn replace_owner(state: &mut LifecycleState, owner: u64, next: HashSet<ChunkKey>) {
        let previous = state.owners.remove(&owner).unwrap_or_default();
        for key in previous.difference(&next) {
            if let Some(status) = state.chunks.get_mut(key) {
                status.interests -= 1;
            }
            Self::remove_if_idle(state, *key);
        }
        for key in next.difference(&previous) {
            state.chunks.entry(*key).or_default().interests += 1;
        }
        if !next.is_empty() {
            state.owners.insert(owner, next);
        }
    }

    fn remove_if_idle(state: &mut LifecycleState, key: ChunkKey) {
        let idle = state.chunks.get(&key).is_some_and(|status| {
            status.interests == 0
                && status.active_accesses == 0
                && status.active_generations == 0
                && status.operation == ChunkOperation::Idle
        });
        if idle {
            state.chunks.remove(&key);
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, LifecycleState> {
        self.state.lock().unwrap_or_else(|err| err.into_inner())
    }

    fn wait<'a>(
        &self,
        guard: std::sync::MutexGuard<'a, LifecycleState>,
    ) -> std::sync::MutexGuard<'a, LifecycleState> {
        self.changed
            .wait(guard)
            .unwrap_or_else(|err| err.into_inner())
    }
}

pub(crate) struct GenerationLease<'a> {
    lifecycle: &'a ChunkLifecycle,
    keys: Vec<ChunkKey>,
}

impl Drop for GenerationLease<'_> {
    fn drop(&mut self) {
        let mut state = self.lifecycle.lock();
        for key in &self.keys {
            if let Some(status) = state.chunks.get_mut(key) {
                status.active_generations -= 1;
            }
            ChunkLifecycle::remove_if_idle(&mut state, *key);
        }
        self.lifecycle.changed.notify_all();
    }
}

pub(crate) struct AccessLease<'a> {
    lifecycle: &'a ChunkLifecycle,
    key: ChunkKey,
}

pub(crate) struct StorageWriteLease<'a> {
    lifecycle: &'a ChunkLifecycle,
    key: ChunkKey,
}

pub(crate) struct CacheLoadLease<'a> {
    lifecycle: &'a ChunkLifecycle,
    key: ChunkKey,
}

impl Drop for CacheLoadLease<'_> {
    fn drop(&mut self) {
        self.lifecycle
            .release_operation(self.key, ChunkOperation::Loading);
    }
}

impl Drop for StorageWriteLease<'_> {
    fn drop(&mut self) {
        self.lifecycle
            .release_operation(self.key, ChunkOperation::Writing);
    }
}

impl Drop for AccessLease<'_> {
    fn drop(&mut self) {
        let mut state = self.lifecycle.lock();
        if let Some(status) = state.chunks.get_mut(&self.key) {
            status.active_accesses -= 1;
        }
        ChunkLifecycle::remove_if_idle(&mut state, self.key);
        self.lifecycle.changed.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::ChunkLifecycle;
    use temper_core::dimension::Dimension;
    use temper_core::pos::{ChunkPos, FullChunkPos};

    #[test]
    fn eviction_waits_for_owner_interest_and_generation() {
        let lifecycle = ChunkLifecycle::default();
        let pos = ChunkPos::new(3, -2);
        let key = FullChunkPos::new(pos, Dimension::Overworld);
        lifecycle.set_owner_chunks(7, [key]);
        assert!(!lifecycle.begin_eviction(key));

        lifecycle.set_owner_chunks(7, []);
        let generation = lifecycle.begin_generation(Dimension::Overworld, pos, 1);
        assert!(!lifecycle.begin_eviction(key));
        drop(generation);
        assert!(lifecycle.begin_eviction(key));
        lifecycle.eviction_cancelled(key);
    }

    #[test]
    fn generation_pins_its_neighbourhood() {
        let lifecycle = ChunkLifecycle::default();
        let pos = ChunkPos::new(0, 0);
        let generation = lifecycle.begin_generation(Dimension::Overworld, pos, 2);

        assert!(!lifecycle.begin_eviction(FullChunkPos::of(2, -1, Dimension::Overworld)));
        let outside = FullChunkPos::of(3, 0, Dimension::Overworld);
        assert!(lifecycle.begin_eviction(outside));
        lifecycle.eviction_cancelled(outside);
        drop(generation);
        let neighbor = FullChunkPos::of(2, -1, Dimension::Overworld);
        assert!(lifecycle.begin_eviction(neighbor));
        lifecycle.eviction_cancelled(neighbor);
    }

    #[test]
    fn evicted_chunk_stays_unavailable_until_its_save_finishes() {
        let lifecycle = ChunkLifecycle::default();
        let key = FullChunkPos::of(-4, 8, Dimension::Overworld);

        assert!(lifecycle.begin_eviction(key));
        lifecycle.eviction_removed(key, true);
        assert!(!lifecycle.begin_eviction(key));

        lifecycle.save_finished(key);
        let access = lifecycle.begin_access(key);
        assert!(!lifecycle.begin_eviction(key));
        drop(access);
        assert!(lifecycle.begin_eviction(key));
        lifecycle.eviction_cancelled(key);
    }

    #[test]
    fn one_owner_cannot_release_another_owners_chunk() {
        let lifecycle = ChunkLifecycle::default();
        let key = FullChunkPos::of(5, 12, Dimension::Overworld);
        lifecycle.set_owner_chunks(11, [key]);
        lifecycle.set_owner_chunks(22, [key]);

        lifecycle.set_owner_chunks(11, []);
        assert!(lifecycle.is_interested(key));
        assert!(!lifecycle.begin_eviction(key));

        lifecycle.set_owner_chunks(22, []);
        assert!(!lifecycle.is_interested(key));
        assert!(lifecycle.begin_eviction(key));
        lifecycle.eviction_cancelled(key);
    }

    #[test]
    fn storage_write_reservation_requires_an_unowned_chunk() {
        let lifecycle = ChunkLifecycle::default();
        let key = FullChunkPos::of(-8, 4, Dimension::Overworld);
        lifecycle.set_owner_chunks(9, [key]);
        assert!(lifecycle.begin_storage_write(key).is_none());

        lifecycle.set_owner_chunks(9, []);
        let write = lifecycle
            .begin_storage_write(key)
            .expect("unowned chunk should allow a storage write");
        assert!(!lifecycle.begin_eviction(key));
        drop(write);
        assert!(lifecycle.begin_eviction(key));
        lifecycle.eviction_cancelled(key);
    }
}
