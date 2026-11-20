use bevy_ecs::prelude::Component;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::collections::{HashMap, HashSet, VecDeque};
use temper_core::pos::FullChunkPos;
use uuid::Uuid;

#[derive(Default)]
pub struct ChunkDeliveryState {
    needed: HashSet<FullChunkPos>,
    loading: VecDeque<FullChunkPos>,
    dirty: VecDeque<FullChunkPos>,
    loaded: HashSet<FullChunkPos>,
    in_flight: HashSet<FullChunkPos>,
    unloading: VecDeque<FullChunkPos>,
}

impl ChunkDeliveryState {
    /// Reconcile the client's chunk state with its current view area.
    pub fn set_needed(&mut self, needed: &HashSet<FullChunkPos>) {
        self.needed.clone_from(needed);

        // A chunk can become needed again before its queued unload is sent.
        // In that case the client still has it, so cancel the unload and keep
        // it loaded instead of sending an unload/load pair.
        let needed_unloads = self
            .unloading
            .iter()
            .filter(|chunk| needed.contains(chunk))
            .copied()
            .collect::<Vec<_>>();
        self.unloading.retain(|chunk| !needed.contains(chunk));
        self.loaded.extend(needed_unloads);

        let no_longer_needed = self
            .loaded
            .iter()
            .filter(|chunk| !needed.contains(chunk))
            .copied()
            .collect::<Vec<_>>();
        for chunk in no_longer_needed {
            self.loaded.remove(&chunk);
            self.unloading.push_back(chunk);
        }

        self.loading.retain(|chunk| needed.contains(chunk));
        self.dirty.retain(|chunk| needed.contains(chunk));

        for &chunk in needed {
            if !self.loaded.contains(&chunk)
                && !self.loading.contains(&chunk)
                && !self.dirty.contains(&chunk)
                && !self.in_flight.contains(&chunk)
            {
                self.loading.push_back(chunk);
            }
        }
    }

    pub fn is_loaded(&self, chunk: FullChunkPos) -> bool {
        self.loaded.contains(&chunk)
    }

    pub fn is_needed(&self, chunk: FullChunkPos) -> bool {
        self.needed.contains(&chunk)
    }

    pub fn queued_chunks(&self) -> &VecDeque<FullChunkPos> {
        &self.loading
    }

    pub fn sort_loading_by_distance(&mut self, center: FullChunkPos) {
        self.loading.make_contiguous().sort_by_key(|chunk| {
            chunk
                .x()
                .abs_diff(center.x())
                .max(chunk.z().abs_diff(center.z()))
        });
    }

    pub fn loading_len(&self) -> usize {
        self.loading.len()
    }

    pub fn in_flight_len(&self) -> usize {
        self.in_flight.len()
    }

    pub fn has_pending_work(&self) -> bool {
        !self.dirty.is_empty() || !self.loading.is_empty()
    }

    pub fn pop_loading(&mut self) -> Option<FullChunkPos> {
        self.loading.pop_front()
    }

    pub fn pop_dirty(&mut self) -> Option<FullChunkPos> {
        self.dirty.pop_front()
    }

    pub fn pop_unloading(&mut self) -> Option<FullChunkPos> {
        self.unloading.pop_front()
    }

    pub fn queue_loading(&mut self, chunk: FullChunkPos) {
        if self.needed.contains(&chunk)
            && !self.loaded.contains(&chunk)
            && !self.loading.contains(&chunk)
            && !self.dirty.contains(&chunk)
            && !self.in_flight.contains(&chunk)
        {
            self.loading.push_back(chunk);
        }
    }

    pub fn queue_dirty(&mut self, chunk: FullChunkPos) {
        if self.needed.contains(&chunk) && !self.dirty.contains(&chunk) {
            self.dirty.push_back(chunk);
        }
    }

    /// Returns false when a preparation for this chunk is already running.
    pub fn begin_preparation(&mut self, chunk: FullChunkPos) -> bool {
        self.needed.contains(&chunk) && self.in_flight.insert(chunk)
    }

    pub fn finish_preparation(&mut self, chunk: FullChunkPos) {
        self.in_flight.remove(&chunk);
    }

    pub fn mark_loaded(&mut self, chunk: FullChunkPos) -> bool {
        if !self.needed.contains(&chunk) {
            return false;
        }
        self.in_flight.remove(&chunk);
        self.loading.retain(|queued| *queued != chunk);
        self.dirty.retain(|queued| *queued != chunk);
        self.unloading.retain(|queued| *queued != chunk);
        self.loaded.insert(chunk);
        true
    }

    pub fn protected_chunks(&self) -> impl Iterator<Item = FullChunkPos> + '_ {
        self.needed
            .iter()
            .chain(
                self.in_flight
                    .iter()
                    .filter(|chunk| !self.needed.contains(*chunk)),
            )
            .copied()
    }

    /// Clear per-connection delivery state when a new play session starts.
    pub fn reset(&mut self) {
        self.needed.clear();
        self.loading.clear();
        self.dirty.clear();
        self.loaded.clear();
        self.in_flight.clear();
        self.unloading.clear();
    }
}

pub struct ReadyChunk {
    pub chunk_pos: FullChunkPos,
    pub packet_data: Vec<u8>,
    pub entities: Vec<(Uuid, u16)>,
    pub is_new_load: bool,
}

pub enum PreparedChunk {
    Ready(ReadyChunk),
    Failed { chunk_pos: FullChunkPos },
}

impl PreparedChunk {
    /// The chunk this message is about, whether it succeeded or not.
    pub fn chunk_pos(&self) -> FullChunkPos {
        match self {
            Self::Ready(ready) => ready.chunk_pos,
            Self::Failed { chunk_pos } => *chunk_pos,
        }
    }
}

#[derive(Component)]
pub struct ChunkReceiver {
    /// Owns all per-connection chunk delivery transitions and queues.
    pub chunks: ChunkDeliveryState,
    pub chunks_per_tick: f32,
    pub ready_tx: Sender<PreparedChunk>,
    pub ready_rx: Receiver<PreparedChunk>,
    pub retry_counts: HashMap<FullChunkPos, u8>,
}

impl ChunkReceiver {
    pub fn new() -> Self {
        let (ready_tx, ready_rx) = unbounded();
        Self {
            chunks: ChunkDeliveryState::default(),
            chunks_per_tick: 32.5,
            ready_tx,
            ready_rx,
            retry_counts: HashMap::new(),
        }
    }
}

impl Default for ChunkReceiver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::ChunkDeliveryState;
    use std::collections::HashSet;
    use temper_core::dimension::Dimension;
    use temper_core::pos::FullChunkPos;

    #[test]
    fn needed_chunks_move_through_one_delivery_state_owner() {
        let first = FullChunkPos::of(0, 0, Dimension::Overworld);
        let second = FullChunkPos::of(1, 0, Dimension::Overworld);
        let third = FullChunkPos::of(2, 0, Dimension::Overworld);
        let mut state = ChunkDeliveryState::default();
        state.set_needed(&HashSet::from([first]));
        assert_eq!(state.loading_len(), 1);

        assert_eq!(state.pop_loading(), Some(first));
        assert!(state.begin_preparation(first));
        state.finish_preparation(first);
        state.mark_loaded(first);

        state.set_needed(&HashSet::from([second, third]));
        assert!(!state.is_loaded(first));
        assert_eq!(state.pop_unloading(), Some(first));
        assert_eq!(state.loading_len(), 2);
        assert_eq!(
            state
                .queued_chunks()
                .iter()
                .copied()
                .collect::<HashSet<_>>(),
            HashSet::from([second, third]),
        );
    }

    #[test]
    fn repeated_dispatch_cannot_prepare_the_same_chunk_twice() {
        let chunk = FullChunkPos::of(-3, 9, Dimension::Overworld);
        let mut state = ChunkDeliveryState::default();
        state.set_needed(&HashSet::from([chunk]));

        assert!(state.begin_preparation(chunk));
        assert!(!state.begin_preparation(chunk));
        state.finish_preparation(chunk);
        assert!(state.begin_preparation(chunk));
    }

    #[test]
    fn reentering_a_chunk_before_unload_cancels_the_unload() {
        let chunk = FullChunkPos::of(4, -2, Dimension::Overworld);
        let mut state = ChunkDeliveryState::default();
        state.set_needed(&HashSet::from([chunk]));
        assert_eq!(state.pop_loading(), Some(chunk));
        assert!(state.mark_loaded(chunk));

        state.set_needed(&HashSet::new());
        assert!(!state.is_loaded(chunk));
        state.set_needed(&HashSet::from([chunk]));

        assert!(state.is_loaded(chunk));
        assert_eq!(state.pop_unloading(), None);
        assert_eq!(state.pop_loading(), None);
    }

    #[test]
    fn failed_preparation_is_not_requeued_after_the_chunk_leaves_view() {
        let chunk = FullChunkPos::of(-8, 6, Dimension::Overworld);
        let mut state = ChunkDeliveryState::default();
        state.set_needed(&HashSet::from([chunk]));
        assert!(state.begin_preparation(chunk));
        state.set_needed(&HashSet::new());
        state.finish_preparation(chunk);
        state.queue_loading(chunk);

        assert_eq!(state.loading_len(), 0);
        assert!(!state.is_needed(chunk));
    }

    #[test]
    fn needed_chunk_stays_protected_between_queue_pop_and_dispatch() {
        let chunk = FullChunkPos::of(12, -5, Dimension::Overworld);
        let mut state = ChunkDeliveryState::default();
        state.set_needed(&HashSet::from([chunk]));
        assert_eq!(state.pop_loading(), Some(chunk));

        assert_eq!(
            state.protected_chunks().collect::<HashSet<_>>(),
            HashSet::from([chunk])
        );
    }
}
