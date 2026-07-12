//! Background job types and lifecycle management.
//!
//! Defines the types of background operations supported by the task
//! manager, along with their state machine lifecycle.

use std::fmt;

use super::progress::Progress;

/// Unique identifier for a background job.
///
/// Each job receives a monotonically increasing ID when created.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JobId(u64);

impl JobId {
    /// Creates a new `JobId` with the given raw value.
    pub(crate) fn new(id: u64) -> Self {
        Self(id)
    }

    /// Returns the raw ID value.
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "job-{}", self.0)
    }
}

/// The type of background operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobType {
    /// Index all enabled folders from the configuration.
    IndexAll,
    /// Rebuild the entire search index from scratch.
    RebuildIndex,
}

impl fmt::Display for JobType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobType::IndexAll => write!(f, "Index All"),
            JobType::RebuildIndex => write!(f, "Rebuild Index"),
        }
    }
}

/// The lifecycle state of a background job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    /// The job is queued but has not started yet.
    Pending,
    /// The job is currently executing.
    Running,
    /// The job completed successfully.
    Completed,
    /// The job failed with an error message.
    Failed(String),
}

impl fmt::Display for JobState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobState::Pending => write!(f, "Pending"),
            JobState::Running => write!(f, "Running"),
            JobState::Completed => write!(f, "Completed"),
            JobState::Failed(msg) => write!(f, "Failed: {msg}"),
        }
    }
}

/// A snapshot of a background job's current status.
///
/// Combines the job's identity, type, state, and progress into a
/// single read-only view suitable for display in the GUI.
#[derive(Debug, Clone)]
pub struct JobStatus {
    /// Unique identifier for this job.
    pub id: JobId,
    /// The type of operation being performed.
    pub job_type: JobType,
    /// Current lifecycle state.
    pub state: JobState,
    /// Current progress information, if the job is running.
    pub progress: Option<Progress>,
}

impl JobStatus {
    /// Creates a new `JobStatus` snapshot.
    pub fn new(id: JobId, job_type: JobType, state: JobState, progress: Option<Progress>) -> Self {
        Self {
            id,
            job_type,
            state,
            progress,
        }
    }

    /// Returns `true` if the job is currently running.
    pub fn is_running(&self) -> bool {
        matches!(self.state, JobState::Running)
    }

    /// Returns `true` if the job has completed successfully.
    pub fn is_completed(&self) -> bool {
        matches!(self.state, JobState::Completed)
    }

    /// Returns `true` if the job has failed.
    pub fn is_failed(&self) -> bool {
        matches!(self.state, JobState::Failed(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_id_display() {
        let id = JobId::new(42);
        assert_eq!(id.to_string(), "job-42");
        assert_eq!(id.as_u64(), 42);
    }

    #[test]
    fn job_type_display() {
        assert_eq!(JobType::IndexAll.to_string(), "Index All");
        assert_eq!(JobType::RebuildIndex.to_string(), "Rebuild Index");
    }

    #[test]
    fn job_state_display() {
        assert_eq!(JobState::Pending.to_string(), "Pending");
        assert_eq!(JobState::Running.to_string(), "Running");
        assert_eq!(JobState::Completed.to_string(), "Completed");
        assert_eq!(
            JobState::Failed("test error".into()).to_string(),
            "Failed: test error"
        );
    }

    #[test]
    fn job_status_is_running() {
        let status = JobStatus::new(JobId::new(1), JobType::IndexAll, JobState::Running, None);
        assert!(status.is_running());
        assert!(!status.is_completed());
        assert!(!status.is_failed());
    }

    #[test]
    fn job_status_is_completed() {
        let status = JobStatus::new(JobId::new(1), JobType::IndexAll, JobState::Completed, None);
        assert!(!status.is_running());
        assert!(status.is_completed());
        assert!(!status.is_failed());
    }

    #[test]
    fn job_status_is_failed() {
        let status = JobStatus::new(
            JobId::new(1),
            JobType::IndexAll,
            JobState::Failed("error".into()),
            None,
        );
        assert!(!status.is_running());
        assert!(!status.is_completed());
        assert!(status.is_failed());
    }

    #[test]
    fn job_status_with_progress() {
        let progress = Progress::new("Indexing").with_current_file("test.txt");
        let status = JobStatus::new(
            JobId::new(1),
            JobType::IndexAll,
            JobState::Running,
            Some(progress),
        );
        assert!(status.progress.is_some());
        assert_eq!(
            status.progress.unwrap().current_file.as_deref(),
            Some("test.txt")
        );
    }
}
