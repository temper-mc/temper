use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering::AcqRel;

use crossbeam_queue::SegQueue;
use dashmap::DashMap;
use gen_core::{ChunkGenerator, GenStage, StageDependencies};
use temper_core::dimension::Dimension;
use temper_core::pos::ChunkPos;

use crate::SchedulerError;
use crate::{
    ClaimedJob, JobEntry, JobKey, JobPriority, JobState, ReadyJob, ReadySource, RequestEntry,
    RequestId,
};

#[derive(Debug)]
pub struct SchedulerState {
    pub jobs: DashMap<JobKey, Arc<JobEntry>>,
    pub requests: DashMap<RequestId, Arc<RequestEntry>>,
    pub global_ready: SegQueue<JobKey>,
    registration_lock: Mutex<()>,
    next_request_id: AtomicU64,
    next_ready_sequence: AtomicU64,
}

impl Default for SchedulerState {
    fn default() -> Self {
        Self {
            jobs: DashMap::new(),
            requests: DashMap::new(),
            global_ready: SegQueue::new(),
            registration_lock: Mutex::new(()),
            next_request_id: AtomicU64::new(0),
            next_ready_sequence: AtomicU64::new(0),
        }
    }
}

impl SchedulerState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_request(
        &self,
        generator: &dyn ChunkGenerator,
        target: JobKey,
    ) -> Result<Arc<RequestEntry>, SchedulerError> {
        let _registration_lock = self
            .registration_lock
            .lock()
            .expect("registration lock poisoned");
        let request = Arc::new(RequestEntry::new(self.next_request_id(), target));
        self.requests.insert(request.id, Arc::clone(&request));

        let mut visited = HashSet::new();
        if let Err(err) = self.register_job(generator, &request, target, &mut visited) {
            self.requests.remove(&request.id);
            return Err(err);
        }

        Ok(request)
    }

    pub fn mark_complete(&self, key: JobKey) -> Result<(), SchedulerError> {
        let job = self.jobs.get(&key).ok_or(SchedulerError::UnknownJob(key))?;

        let Some(dependents) = job.complete_and_take_dependents() else {
            return Ok(());
        };

        for request_id in job.interested_requests() {
            let Some(request) = self.requests.get(&request_id) else {
                continue;
            };
            request.wake();
        }

        for dependent_key in dependents {
            let Some(dependent) = self.jobs.get(&dependent_key) else {
                continue;
            };
            let previous = dependent.remaining_dependencies.fetch_sub(1, AcqRel);

            if previous == 1 && dependent.state.mark_ready() {
                self.publish_ready(&dependent);
            }
        }

        Ok(())
    }

    pub fn unregister_request(&self, request: RequestId) {
        self.requests.remove(&request);
    }

    pub fn has_pending_jobs(&self, dimension: Dimension, pos: ChunkPos) -> bool {
        (0..=GenStage::FULL.raw()).any(|stage| {
            self.jobs
                .get(&JobKey::new(dimension, pos, GenStage::new(stage)))
                .is_some_and(|job| job.state.load() != JobState::Complete)
        })
    }

    pub fn forget_chunk(&self, dimension: Dimension, pos: ChunkPos) {
        let _registration_lock = self
            .registration_lock
            .lock()
            .expect("registration lock poisoned");

        for stage in 0..=GenStage::FULL.raw() {
            self.jobs
                .remove(&JobKey::new(dimension, pos, GenStage::new(stage)));
        }
    }

    pub fn forget_all(&self) {
        let _registration_lock = self
            .registration_lock
            .lock()
            .expect("registration lock poisoned");

        self.jobs.clear();
        self.requests.clear();
        while self.global_ready.pop().is_some() {}
    }

    pub fn claim_next_for_request(&self, request: &RequestEntry) -> Option<ClaimedJob> {
        while let Some(ready_job) = request.pop_ready() {
            if self.try_claim(ready_job.key) {
                return Some(ClaimedJob::new(ready_job.key, ReadySource::Local));
            }
        }

        while let Some(key) = self.global_ready.pop() {
            if self.try_claim(key) {
                return Some(ClaimedJob::new(key, ReadySource::Global));
            }
        }

        None
    }

    pub fn get_job(&self, key: JobKey) -> Option<Arc<JobEntry>> {
        self.jobs.get(&key).map(|job| Arc::clone(&job))
    }

    /// Releases a job that failed mid-run so it doesn't sit in `Running`
    /// forever, and wakes anything waiting on it so those callers can
    /// re-evaluate instead of sleeping indefinitely.
    pub fn fail_job(&self, key: JobKey) {
        let mut pending = vec![key];
        let mut visited = HashSet::new();

        while let Some(key) = pending.pop() {
            if !visited.insert(key) {
                continue;
            }
            let Some(job) = self.jobs.get(&key) else {
                continue;
            };

            if job.state.load() == JobState::Complete {
                continue;
            }
            if job.state.mark_failed() {
                for request_id in job.interested_requests() {
                    if let Some(request) = self.requests.get(&request_id) {
                        request.fail(key);
                    }
                }
            }

            pending.extend(job.dependents());
        }
    }

    pub fn next_request_id(&self) -> RequestId {
        RequestId::new(self.next_request_id.fetch_add(1, AcqRel))
    }

    pub fn next_ready_sequence(&self) -> u64 {
        self.next_ready_sequence.fetch_add(1, AcqRel)
    }

    fn register_job(
        &self,
        generator: &dyn ChunkGenerator,
        request: &RequestEntry,
        key: JobKey,
        visited: &mut HashSet<JobKey>,
    ) -> Result<Arc<JobEntry>, SchedulerError> {
        if !visited.insert(key) {
            return self.get_job(key).ok_or(SchedulerError::UnknownJob(key));
        }

        if let Some(job) = self.get_job(key) {
            self.register_job_interest(request, &job);
            return Ok(job);
        }

        let stage_spec = generator
            .stage_spec(key.stage)
            .ok_or(SchedulerError::UnknownStage(key))?;

        let job = Arc::new(JobEntry::new_pending(key));
        self.jobs.insert(key, Arc::clone(&job));
        job.add_interested_request(request.id);

        let dependency_keys = dependency_keys(key, stage_spec.dependencies);

        for dependency_key in dependency_keys {
            let dependency = self.register_job(generator, request, dependency_key, visited)?;

            if !dependency.add_dependent_or_already_complete(key) {
                job.remaining_dependencies.fetch_add(1, AcqRel);
            }
        }

        // Clear the placeholder dependency. If nothing real is outstanding
        // either, this is the moment the job becomes ready.
        if job.remaining_dependencies.fetch_sub(1, AcqRel) == 1 && job.state.mark_ready() {
            self.publish_ready(&job);
        }

        Ok(job)
    }

    fn register_job_interest(&self, request: &RequestEntry, job: &JobEntry) {
        if !job.add_interested_request(request.id) {
            return;
        }

        match job.state.load() {
            JobState::Ready => {
                request.push_ready(self.ready_job_for_request(job.key, request.target));
            }
            JobState::Failed => request.fail(job.key),
            JobState::Waiting | JobState::Running | JobState::Complete => {}
        }
    }

    fn publish_ready(&self, job: &JobEntry) {
        self.global_ready.push(job.key);

        for request_id in job.interested_requests() {
            let Some(request) = self.requests.get(&request_id) else {
                continue;
            };

            request.push_ready(self.ready_job_for_request(job.key, request.target));
        }
    }

    fn ready_job_for_request(&self, key: JobKey, target: JobKey) -> ReadyJob {
        ReadyJob::new(
            key,
            JobPriority::new(
                u32::from(target.stage.raw().saturating_sub(key.stage.raw())),
                chunk_distance(key.pos, target.pos),
                self.next_ready_sequence(),
            ),
        )
    }

    fn try_claim(&self, key: JobKey) -> bool {
        self.jobs.get(&key).is_some_and(|job| job.state.try_claim())
    }
}

fn dependency_keys(key: JobKey, dependencies: StageDependencies) -> Vec<JobKey> {
    let mut keys = Vec::new();

    if let Some(stage) = dependencies.own_stage {
        keys.push(JobKey::new(key.dimension, key.pos, stage));
    }

    if let Some(stage) = dependencies.neighbor_stage {
        let radius = i32::from(dependencies.neighbor_radius);

        for x in -radius..=radius {
            for z in -radius..=radius {
                if x == 0 && z == 0 {
                    continue;
                }

                let pos = offset_chunk_pos(key.pos, x, z);
                let neighbor_key = JobKey::new(key.dimension, pos, stage);

                if !keys.contains(&neighbor_key) {
                    keys.push(neighbor_key);
                }
            }
        }
    }

    keys
}

fn offset_chunk_pos(pos: ChunkPos, x: i32, z: i32) -> ChunkPos {
    let (chunk_x, chunk_z) = chunk_coords(pos);
    ChunkPos::new(chunk_x + x, chunk_z + z)
}

fn chunk_distance(lhs: ChunkPos, rhs: ChunkPos) -> u32 {
    let (lhs_x, lhs_z) = chunk_coords(lhs);
    let (rhs_x, rhs_z) = chunk_coords(rhs);

    lhs_x.abs_diff(rhs_x).max(lhs_z.abs_diff(rhs_z))
}

fn chunk_coords(pos: ChunkPos) -> (i32, i32) {
    (pos.pos.x.div_euclid(16), pos.pos.y.div_euclid(16))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::Ordering::Acquire;

    use dashmap::DashMap;
    use gen_core::GenStage;
    use gen_core::{GenerationError, GeneratorId, StageDependencies, StageInput, StageSpec};
    use temper_core::dimension::Dimension;
    use temper_threadpool::ThreadPool;

    use super::*;

    struct TestGenerator;

    impl ChunkGenerator for TestGenerator {
        fn id(&self) -> GeneratorId {
            GeneratorId::new("test")
        }

        fn final_stage(&self) -> GenStage {
            GenStage::SURFACE
        }

        fn stage_spec(&self, stage: GenStage) -> Option<StageSpec> {
            match stage {
                GenStage::EMPTY => Some(StageSpec::new(
                    GenStage::EMPTY,
                    "empty",
                    StageDependencies::NONE,
                )),
                GenStage::NOISE => Some(StageSpec::new(
                    GenStage::NOISE,
                    "noise",
                    StageDependencies::only_own(GenStage::EMPTY),
                )),
                GenStage::SURFACE => Some(StageSpec::new(
                    GenStage::SURFACE,
                    "surface",
                    StageDependencies::with_neighbors(GenStage::NOISE, GenStage::NOISE, 1),
                )),
                _ => None,
            }
        }

        fn advance_stage(&self, _input: StageInput<'_>) -> Result<(), GenerationError> {
            Ok(())
        }
    }

    #[test]
    fn register_request_expands_stage_dependencies() {
        let scheduler = SchedulerState::new();
        let target = JobKey::new(Dimension::Overworld, ChunkPos::new(0, 0), GenStage::SURFACE);

        let request = scheduler
            .register_request(&TestGenerator, target)
            .expect("request should register");

        assert_eq!(request.target, target);
        assert_eq!(scheduler.jobs.len(), 19);
        assert_eq!(
            scheduler
                .get_job(target)
                .expect("target should exist")
                .remaining_dependencies
                .load(Acquire),
            9
        );

        let mut ready_jobs = 0;
        while request.pop_ready().is_some() {
            ready_jobs += 1;
        }
        assert_eq!(ready_jobs, 9);
    }

    #[test]
    fn failed_dependency_is_reported_to_its_waiting_request() {
        let scheduler = SchedulerState::new();
        let target = JobKey::new(Dimension::Overworld, ChunkPos::new(0, 0), GenStage::SURFACE);
        let request = scheduler
            .register_request(&TestGenerator, target)
            .expect("request should register");
        let dependency = JobKey::new(Dimension::Overworld, ChunkPos::new(1, 1), GenStage::NOISE);

        scheduler.fail_job(dependency);

        assert_eq!(request.take_failed_job(), Some(dependency));

        let retry = scheduler
            .register_request(&TestGenerator, target)
            .expect("retry request should register");
        assert_eq!(retry.take_failed_job(), Some(target));
    }

    #[test]
    fn completing_dependency_publishes_dependent_work() {
        let scheduler = SchedulerState::new();
        let target = JobKey::new(Dimension::Overworld, ChunkPos::new(0, 0), GenStage::NOISE);
        let dependency = JobKey::new(Dimension::Overworld, ChunkPos::new(0, 0), GenStage::EMPTY);
        let request = scheduler
            .register_request(&TestGenerator, target)
            .expect("request should register");

        scheduler
            .mark_complete(dependency)
            .expect("dependency should complete");

        let target_entry = scheduler.get_job(target).expect("target should exist");
        assert_eq!(target_entry.state.load(), JobState::Ready);
        assert_eq!(target_entry.remaining_dependencies.load(Acquire), 0);
        assert_eq!(
            request.pop_ready().expect("ready job should exist").key,
            target
        );
    }

    #[test]
    fn completing_job_twice_does_not_double_decrement_dependents() {
        let scheduler = SchedulerState::new();
        let target = JobKey::new(Dimension::Overworld, ChunkPos::new(0, 0), GenStage::NOISE);
        let dependency = JobKey::new(Dimension::Overworld, ChunkPos::new(0, 0), GenStage::EMPTY);

        scheduler
            .register_request(&TestGenerator, target)
            .expect("request should register");

        scheduler
            .mark_complete(dependency)
            .expect("dependency should complete");
        scheduler
            .mark_complete(dependency)
            .expect("second completion should be ignored");

        assert_eq!(
            scheduler
                .get_job(target)
                .expect("target should exist")
                .remaining_dependencies
                .load(Acquire),
            0
        );
    }

    #[test]
    fn claiming_work_prefers_local_queue_before_global_queue() {
        let scheduler = SchedulerState::new();
        let far_target = JobKey::new(Dimension::Overworld, ChunkPos::new(8, 8), GenStage::NOISE);
        let near_target = JobKey::new(Dimension::Overworld, ChunkPos::new(0, 0), GenStage::NOISE);
        let near_dependency =
            JobKey::new(Dimension::Overworld, ChunkPos::new(0, 0), GenStage::EMPTY);

        scheduler
            .register_request(&TestGenerator, far_target)
            .expect("far request should register");
        let near_request = scheduler
            .register_request(&TestGenerator, near_target)
            .expect("near request should register");

        assert_eq!(
            scheduler.claim_next_for_request(&near_request),
            Some(ClaimedJob::new(near_dependency, ReadySource::Local))
        );
    }

    #[test]
    fn claiming_work_climbs_toward_the_target_before_clearing_all_lower_stage_work() {
        let scheduler = SchedulerState::new();
        let target = JobKey::new(Dimension::Overworld, ChunkPos::new(0, 0), GenStage::SURFACE);
        let center_empty = JobKey::new(Dimension::Overworld, ChunkPos::new(0, 0), GenStage::EMPTY);
        let center_noise = JobKey::new(Dimension::Overworld, ChunkPos::new(0, 0), GenStage::NOISE);
        let request = scheduler
            .register_request(&TestGenerator, target)
            .expect("request should register");

        assert_eq!(
            scheduler.claim_next_for_request(&request),
            Some(ClaimedJob::new(center_empty, ReadySource::Local))
        );

        scheduler
            .mark_complete(center_empty)
            .expect("center empty should complete");

        assert_eq!(
            scheduler.claim_next_for_request(&request),
            Some(ClaimedJob::new(center_noise, ReadySource::Local))
        );
    }

    #[test]
    fn claiming_work_falls_back_to_global_queue_after_stale_local_work() {
        let scheduler = SchedulerState::new();
        let shared_target = JobKey::new(Dimension::Overworld, ChunkPos::new(0, 0), GenStage::NOISE);
        let other_target = JobKey::new(Dimension::Overworld, ChunkPos::new(8, 8), GenStage::NOISE);
        let shared_dependency =
            JobKey::new(Dimension::Overworld, ChunkPos::new(0, 0), GenStage::EMPTY);
        let other_dependency =
            JobKey::new(Dimension::Overworld, ChunkPos::new(8, 8), GenStage::EMPTY);

        let first_request = scheduler
            .register_request(&TestGenerator, shared_target)
            .expect("first request should register");
        let second_request = scheduler
            .register_request(&TestGenerator, shared_target)
            .expect("second request should register");
        scheduler
            .register_request(&TestGenerator, other_target)
            .expect("other request should register");

        assert_eq!(
            scheduler.claim_next_for_request(&first_request),
            Some(ClaimedJob::new(shared_dependency, ReadySource::Local))
        );

        assert_eq!(
            scheduler.claim_next_for_request(&second_request),
            Some(ClaimedJob::new(other_dependency, ReadySource::Global))
        );
    }

    #[test]
    fn concurrent_requests_share_one_dependency_graph() {
        let scheduler = Arc::new(SchedulerState::new());
        let generator = Arc::new(TestGenerator);
        let pool = ThreadPool::new();
        let mut requests = pool.batch();

        for x in 0..8 {
            for z in 0..8 {
                let scheduler = Arc::clone(&scheduler);
                let generator = Arc::clone(&generator);
                requests.execute(move || {
                    let target =
                        JobKey::new(Dimension::Overworld, ChunkPos::new(x, z), GenStage::SURFACE);

                    scheduler
                        .register_request(&*generator, target)
                        .expect("request should register")
                        .id
                });
            }
        }

        let request_ids = requests.wait();

        assert_eq!(request_ids.len(), 64);
        assert_eq!(scheduler.requests.len(), 64);
        assert_eq!(scheduler.jobs.len(), 264);

        let completed = Arc::new(DashMap::<JobKey, usize>::new());
        let mut workers = pool.batch();

        for worker_id in 0..8 {
            let scheduler = Arc::clone(&scheduler);
            let completed = Arc::clone(&completed);
            let requests = request_ids
                .iter()
                .map(|id| {
                    Arc::clone(
                        &scheduler
                            .requests
                            .get(id)
                            .expect("request should still exist"),
                    )
                })
                .collect::<Vec<_>>();

            workers.execute(move || {
                let mut completed_jobs = 0;
                let mut idle_spins = 0;

                while completed.len() < scheduler.jobs.len() {
                    let request = &requests[(worker_id + completed_jobs) % requests.len()];

                    if let Some(claimed) = scheduler.claim_next_for_request(request) {
                        scheduler
                            .mark_complete(claimed.key)
                            .expect("claimed job should complete");
                        completed
                            .entry(claimed.key)
                            .and_modify(|count| *count += 1)
                            .or_insert(1);
                        completed_jobs += 1;
                        idle_spins = 0;
                        continue;
                    }

                    idle_spins += 1;
                    assert!(
                        idle_spins < 10_000,
                        "worker should not stay idle while jobs remain incomplete"
                    );
                    std::thread::yield_now();
                }

                completed_jobs
            });
        }

        let worker_counts = workers.wait();
        let completed_jobs = worker_counts.into_iter().sum::<usize>();

        assert_eq!(completed_jobs, scheduler.jobs.len());
        assert_eq!(completed.len(), scheduler.jobs.len());
        assert!(
            completed.iter().all(|job| *job.value() == 1),
            "every job should be completed exactly once"
        );

        for x in 0..8 {
            for z in 0..8 {
                let target =
                    JobKey::new(Dimension::Overworld, ChunkPos::new(x, z), GenStage::SURFACE);

                let target = scheduler.get_job(target).expect("target should exist");
                assert_eq!(target.state.load(), JobState::Complete);
                assert_eq!(target.remaining_dependencies.load(Acquire), 0);
            }
        }
    }

    #[test]
    fn forgetting_a_chunk_removes_its_generation_jobs() {
        let scheduler = SchedulerState::new();
        let target = JobKey::new(Dimension::Overworld, ChunkPos::new(0, 0), GenStage::SURFACE);

        scheduler
            .register_request(&TestGenerator, target)
            .expect("request should register");

        scheduler.forget_chunk(Dimension::Overworld, ChunkPos::new(0, 0));

        assert!(scheduler.get_job(target).is_none());
        assert!(
            scheduler
                .get_job(JobKey::new(
                    Dimension::Overworld,
                    ChunkPos::new(0, 0),
                    GenStage::NOISE,
                ))
                .is_none()
        );
        assert!(
            scheduler
                .get_job(JobKey::new(
                    Dimension::Overworld,
                    ChunkPos::new(1, 0),
                    GenStage::NOISE,
                ))
                .is_some()
        );
    }
}
