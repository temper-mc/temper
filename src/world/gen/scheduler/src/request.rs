use std::collections::BinaryHeap;
use std::sync::Mutex;

use crossbeam_channel::{Receiver, Sender};

use crate::{JobKey, ReadyJob};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RequestId(u64);

impl RequestId {
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug)]
pub struct RequestEntry {
    pub id: RequestId,
    pub target: JobKey,
    local_ready: Mutex<BinaryHeap<ReadyJob>>,
    failed_job: Mutex<Option<JobKey>>,
    wake_sender: Sender<()>,
    wake_receiver: Receiver<()>,
}

impl RequestEntry {
    pub fn new(id: RequestId, target: JobKey) -> Self {
        let (wake_sender, wake_receiver) = crossbeam_channel::unbounded();

        Self {
            id,
            target,
            local_ready: Mutex::new(BinaryHeap::new()),
            failed_job: Mutex::new(None),
            wake_sender,
            wake_receiver,
        }
    }

    pub fn push_ready(&self, job: ReadyJob) {
        self.local_ready
            .lock()
            .expect("request ready queue poisoned")
            .push(job);
        let _ = self.wake_sender.send(());
    }

    pub fn pop_ready(&self) -> Option<ReadyJob> {
        self.local_ready
            .lock()
            .expect("request ready queue poisoned")
            .pop()
    }

    pub fn wake_receiver(&self) -> Receiver<()> {
        self.wake_receiver.clone()
    }

    pub fn wake(&self) {
        let _ = self.wake_sender.send(());
    }

    pub fn fail(&self, job: JobKey) {
        let mut failed_job = self
            .failed_job
            .lock()
            .expect("request failure state poisoned");
        if failed_job.is_none() {
            *failed_job = Some(job);
        }
        drop(failed_job);
        self.wake();
    }

    pub fn take_failed_job(&self) -> Option<JobKey> {
        self.failed_job
            .lock()
            .expect("request failure state poisoned")
            .take()
    }
}

#[cfg(test)]
mod tests {
    use gen_core::GenStage;
    use temper_core::dimension::Dimension;
    use temper_core::pos::ChunkPos;

    use crate::JobPriority;

    use super::*;

    #[test]
    fn request_queue_wakes_when_work_is_pushed() {
        let request = RequestEntry::new(
            RequestId::new(0),
            JobKey::new(Dimension::Overworld, ChunkPos::new(0, 0), GenStage::FULL),
        );
        let receiver = request.wake_receiver();
        let ready = ReadyJob::new(request.target, JobPriority::new(0, 0, 0));

        request.push_ready(ready);

        assert_eq!(request.pop_ready(), Some(ready));
        assert!(receiver.try_recv().is_ok());
    }

    #[test]
    fn request_records_failed_dependency_and_wakes_waiters() {
        let request = RequestEntry::new(
            RequestId::new(0),
            JobKey::new(Dimension::Overworld, ChunkPos::new(0, 0), GenStage::FULL),
        );
        let receiver = request.wake_receiver();
        let failed = JobKey::new(Dimension::Overworld, ChunkPos::new(-1, 1), GenStage::NOISE);

        request.fail(failed);

        assert_eq!(request.take_failed_job(), Some(failed));
        assert!(receiver.try_recv().is_ok());
    }
}
