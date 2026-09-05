use super::identifiers::OpaqueId;
use serde::Serialize;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    CancellationRequested,
    Cancelled,
    Completed,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobTransitionError {
    InvalidTransition { from: JobState, to: JobState },
    InvalidProgress,
}

impl Display for JobTransitionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(formatter, "job cannot move from {from:?} to {to:?}")
            }
            Self::InvalidProgress => formatter.write_str("job progress exceeds its total"),
        }
    }
}

impl std::error::Error for JobTransitionError {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobProgressEvent {
    pub job_id: OpaqueId,
    pub correlation_id: OpaqueId,
    pub state: JobState,
    pub completed_units: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_units: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobTracker {
    job_id: OpaqueId,
    correlation_id: OpaqueId,
    state: JobState,
}

impl JobTracker {
    pub fn new(job_id: OpaqueId, correlation_id: OpaqueId) -> Self {
        Self {
            job_id,
            correlation_id,
            state: JobState::Queued,
        }
    }

    pub fn state(&self) -> JobState {
        self.state
    }

    pub fn start(&mut self) -> Result<(), JobTransitionError> {
        self.transition(&[JobState::Queued], JobState::Running)
    }

    pub fn request_cancellation(&mut self) -> Result<(), JobTransitionError> {
        if self.state == JobState::CancellationRequested {
            return Ok(());
        }
        self.transition(
            &[JobState::Queued, JobState::Running],
            JobState::CancellationRequested,
        )
    }

    pub fn acknowledge_cancellation(&mut self) -> Result<(), JobTransitionError> {
        self.transition(&[JobState::CancellationRequested], JobState::Cancelled)
    }

    pub fn complete(&mut self) -> Result<(), JobTransitionError> {
        self.transition(&[JobState::Running], JobState::Completed)
    }

    pub fn fail(&mut self) -> Result<(), JobTransitionError> {
        self.transition(&[JobState::Queued, JobState::Running], JobState::Failed)
    }

    pub fn progress(
        &self,
        completed_units: u64,
        total_units: Option<u64>,
    ) -> Result<JobProgressEvent, JobTransitionError> {
        if total_units.is_some_and(|total| completed_units > total) {
            return Err(JobTransitionError::InvalidProgress);
        }

        Ok(JobProgressEvent {
            job_id: self.job_id.clone(),
            correlation_id: self.correlation_id.clone(),
            state: self.state,
            completed_units,
            total_units,
        })
    }

    fn transition(
        &mut self,
        allowed_from: &[JobState],
        to: JobState,
    ) -> Result<(), JobTransitionError> {
        if !allowed_from.contains(&self.state) {
            return Err(JobTransitionError::InvalidTransition {
                from: self.state,
                to,
            });
        }
        self.state = to;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::identifiers::{IdGenerator, IdKind};
    use crate::foundation::test_support::FakeIdGenerator;
    use serde_json::json;

    fn tracker() -> JobTracker {
        let ids = FakeIdGenerator::new(1);
        JobTracker::new(ids.next_id(IdKind::Job), ids.next_id(IdKind::Correlation))
    }

    #[test]
    fn follows_the_cooperative_cancellation_states() {
        let mut job = tracker();

        assert_eq!(job.state(), JobState::Queued);
        job.start().expect("queued job should start");
        assert_eq!(job.state(), JobState::Running);
        job.request_cancellation()
            .expect("running job should accept cancellation");
        assert_eq!(job.state(), JobState::CancellationRequested);
        job.request_cancellation()
            .expect("duplicate cancellation should be idempotent");
        job.acknowledge_cancellation()
            .expect("worker should acknowledge cancellation");
        assert_eq!(job.state(), JobState::Cancelled);
    }

    #[test]
    fn terminal_jobs_reject_cancellation_and_other_transitions() {
        let mut job = tracker();
        job.start().expect("queued job should start");
        job.complete().expect("running job should complete");

        assert_eq!(job.state(), JobState::Completed);
        assert_eq!(
            job.request_cancellation(),
            Err(JobTransitionError::InvalidTransition {
                from: JobState::Completed,
                to: JobState::CancellationRequested,
            })
        );
        assert!(job.fail().is_err());
    }

    #[test]
    fn serializes_only_coarse_progress_without_product_payloads() {
        let mut job = tracker();
        job.start().expect("queued job should start");
        let event = job
            .progress(2, Some(5))
            .expect("bounded progress should be valid");

        assert_eq!(
            serde_json::to_value(event).expect("progress should serialize"),
            json!({
                "jobId": "job_00000000000000000000000000000001",
                "correlationId": "correlation_00000000000000000000000000000002",
                "state": "running",
                "completedUnits": 2,
                "totalUnits": 5,
            })
        );
        assert_eq!(
            job.progress(6, Some(5)),
            Err(JobTransitionError::InvalidProgress)
        );
    }
}
