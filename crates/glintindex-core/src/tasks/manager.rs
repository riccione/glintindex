//! Background task manager for long-running indexing operations.
//!
//! The `TaskManager` is responsible for executing indexing and rebuild
//! operations on background threads while providing real-time progress
//! updates to the caller. Only one job may execute at a time.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::config::AppConfig;
use crate::error::{GlintIndexError, Result};
use crate::index::IndexService;
use crate::scanner::ScannerStatistics;
use crate::traits::DocumentIndexer;

use super::job::{JobId, JobState, JobStatus, JobType};
use super::progress::Progress;

/// Shared state between the main thread and background worker.
///
/// Protected by `Arc<Mutex<...>>` for safe concurrent access.
struct SharedState {
    /// The current job's state.
    state: JobState,
    /// The current job's progress.
    progress: Progress,
}

/// Manages background indexing and rebuild operations.
///
/// The `TaskManager` provides a thread-safe interface for starting
/// background jobs, querying their status, and receiving progress
/// updates. Only one job can run at a time — attempting to start a
/// second job while one is already running returns an error.
///
/// # Thread Safety
///
/// The background thread shares the same `IndexService` instance with
/// the main thread via `Arc<Mutex<IndexService>>`. This avoids
/// Tantivy lockfile conflicts that would occur if a second instance
/// were opened on the same path.
///
/// # Examples
///
/// ```
/// use glintindex_core::tasks::TaskManager;
///
/// // TaskManager requires a shared IndexService reference
/// // let manager = TaskManager::new(index_service);
/// ```
pub struct TaskManager {
    /// Shared reference to the index service.
    index_service: Arc<Mutex<IndexService>>,
    /// Shared state for the current job.
    current_job: Arc<Mutex<Option<SharedState>>>,
    /// Monotonically increasing job ID counter.
    next_id: AtomicU64,
}

impl TaskManager {
    /// Creates a new `TaskManager` with no active jobs.
    ///
    /// The `index_service` is shared between the main thread and any
    /// background workers via `Arc<Mutex<...>>`.
    pub fn new(index_service: Arc<Mutex<IndexService>>) -> Self {
        Self {
            index_service,
            current_job: Arc::new(Mutex::new(None)),
            next_id: AtomicU64::new(1),
        }
    }

    /// Returns the status of the current job, if any.
    ///
    /// Returns `None` if no job has been started or the last job
    /// has been cleaned up.
    pub fn job_status(&self) -> Option<JobStatus> {
        let guard = self.current_job.lock().ok()?;
        let shared = guard.as_ref()?;

        Some(JobStatus::new(
            JobId::new(0),
            JobType::IndexAll,
            shared.state.clone(),
            Some(shared.progress.clone()),
        ))
    }

    /// Returns the current progress, if a job is running.
    pub fn current_progress(&self) -> Option<Progress> {
        let guard = self.current_job.lock().ok()?;
        let shared = guard.as_ref()?;
        Some(shared.progress.clone())
    }

    /// Returns `true` if a job is currently running.
    pub fn is_running(&self) -> bool {
        self.current_job
            .lock()
            .ok()
            .and_then(|guard| {
                guard
                    .as_ref()
                    .map(|s| matches!(s.state, JobState::Running | JobState::Pending))
            })
            .unwrap_or(false)
    }

    /// Starts an "Index All" job on a background thread.
    ///
    /// Returns the job ID immediately. The actual indexing happens
    /// on a background thread. Use `job_status()` or
    /// `current_progress()` to monitor progress.
    ///
    /// # Errors
    ///
    /// Returns an error if a job is already running.
    pub fn start_index_all(&self, config: &AppConfig) -> Result<JobId> {
        self.start_job(JobType::IndexAll, config)
    }

    /// Starts a "Rebuild Index" job on a background thread.
    ///
    /// Returns the job ID immediately. The actual rebuild happens
    /// on a background thread.
    ///
    /// # Errors
    ///
    /// Returns an error if a job is already running.
    pub fn start_rebuild(&self, config: &AppConfig) -> Result<JobId> {
        self.start_job(JobType::RebuildIndex, config)
    }

    /// Internal method to start a job of the given type.
    fn start_job(&self, job_type: JobType, config: &AppConfig) -> Result<JobId> {
        // Check if a job is already running
        {
            let guard = self
                .current_job
                .lock()
                .map_err(|e| GlintIndexError::Other(format!("lock poisoned: {e}")))?;
            if let Some(ref shared) = *guard {
                if matches!(shared.state, JobState::Running | JobState::Pending) {
                    return Err(GlintIndexError::Other(
                        "A job is already running".to_string(),
                    ));
                }
            }
        }

        let id = JobId::new(self.next_id.fetch_add(1, Ordering::SeqCst));
        let status_message = format!("Starting {job_type}...");

        // Set the shared state to Running
        {
            let mut guard = self
                .current_job
                .lock()
                .map_err(|e| GlintIndexError::Other(format!("lock poisoned: {e}")))?;
            *guard = Some(SharedState {
                state: JobState::Running,
                progress: Progress::new(&status_message),
            });
        }

        // Clone what we need for the background thread
        let index_service = self.index_service.clone();
        let ignored_folders = config.ignored_folders.clone();
        let enabled_folders: Vec<PathBuf> = config
            .enabled_folders()
            .into_iter()
            .map(|f| f.path.clone())
            .collect();
        let internal_shared = self.current_job.clone();

        thread::spawn(move || {
            let result = match job_type {
                JobType::IndexAll => Self::run_index_all(
                    &index_service,
                    &ignored_folders,
                    &enabled_folders,
                    &internal_shared,
                ),
                JobType::RebuildIndex => Self::run_rebuild(&index_service, &internal_shared),
            };

            // Update final state
            if let Ok(mut guard) = internal_shared.lock() {
                if let Some(ref mut shared) = *guard {
                    match result {
                        Ok(stats) => {
                            shared.state = JobState::Completed;
                            shared.progress = Progress::from_statistics(&stats, "Completed");
                        }
                        Err(e) => {
                            shared.state = JobState::Failed(e.to_string());
                            shared.progress.status_message = format!("Failed: {e}");
                        }
                    }
                }
            }
        });

        Ok(id)
    }

    /// Runs the "Index All" operation on the background thread.
    ///
    /// Locks the shared `IndexService` to perform scanning and indexing.
    /// The lock is held for the duration of the operation, which is
    /// acceptable since only one job runs at a time.
    fn run_index_all(
        index_service: &Arc<Mutex<IndexService>>,
        ignored_folders: &[String],
        enabled_folders: &[PathBuf],
        shared: &Arc<Mutex<Option<SharedState>>>,
    ) -> Result<ScannerStatistics> {
        // Lock the index service for the duration of the operation
        let service = index_service
            .lock()
            .map_err(|e| GlintIndexError::Other(format!("index service lock poisoned: {e}")))?;

        let scanner =
            crate::scanner::FilesystemScanner::with_custom_ignores(&service, ignored_folders);

        // Update progress: scanning
        if let Ok(mut guard) = shared.lock() {
            if let Some(ref mut s) = *guard {
                s.progress.status_message = "Scanning directories...".to_string();
            }
        }

        let stats = scanner.scan_directories(enabled_folders)?;

        // Update progress before commit
        if let Ok(mut guard) = shared.lock() {
            if let Some(ref mut s) = *guard {
                s.progress = Progress::from_statistics(&stats, "Committing index...");
            }
        }

        service.commit()?;
        service.reload_reader()?;

        Ok(stats)
    }

    /// Runs the "Rebuild Index" operation on the background thread.
    ///
    /// Locks the shared `IndexService` to perform the rebuild.
    fn run_rebuild(
        index_service: &Arc<Mutex<IndexService>>,
        shared: &Arc<Mutex<Option<SharedState>>>,
    ) -> Result<ScannerStatistics> {
        let service = index_service
            .lock()
            .map_err(|e| GlintIndexError::Other(format!("index service lock poisoned: {e}")))?;

        // Update progress
        if let Ok(mut guard) = shared.lock() {
            if let Some(ref mut s) = *guard {
                s.progress.status_message = "Rebuilding index...".to_string();
            }
        }

        service.rebuild()?;
        service.commit()?;
        service.reload_reader()?;

        // Return empty stats for rebuild (no files processed)
        Ok(ScannerStatistics::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::model::IndexedFolder;
    use std::fs;
    use tempfile::TempDir;

    fn setup() -> (TempDir, Arc<Mutex<IndexService>>, AppConfig) {
        let tmp = TempDir::new().unwrap();
        let index_path = tmp.path().join("index");
        let index_service = IndexService::open_or_create(&index_path).unwrap();
        let index_service = Arc::new(Mutex::new(index_service));

        let scan_dir = tmp.path().join("scan");
        fs::create_dir(&scan_dir).unwrap();
        fs::write(scan_dir.join("hello.txt"), "hello world").unwrap();

        let config = AppConfig {
            indexed_folders: vec![IndexedFolder::enabled(scan_dir)],
            index_directory: index_path,
            ..AppConfig::default()
        };

        (tmp, index_service, config)
    }

    #[test]
    fn task_manager_new() {
        let (_tmp, index_service, _config) = setup();
        let manager = TaskManager::new(index_service);
        assert!(!manager.is_running());
        assert!(manager.job_status().is_none());
    }

    #[test]
    fn start_index_all() {
        let (_tmp, index_service, config) = setup();
        let manager = TaskManager::new(index_service);

        let id = manager.start_index_all(&config).unwrap();
        assert_eq!(id.as_u64(), 1);
        assert!(manager.is_running());

        // Wait for the job to complete
        let timeout = std::time::Duration::from_secs(5);
        let start = std::time::Instant::now();
        loop {
            if !manager.is_running() {
                break;
            }
            if start.elapsed() > timeout {
                panic!("Job did not complete within timeout");
            }
            thread::sleep(std::time::Duration::from_millis(50));
        }

        let status = manager.job_status().unwrap();
        assert!(status.is_completed() || status.is_failed());
    }

    #[test]
    fn duplicate_job_rejection() {
        let (_tmp, index_service, config) = setup();
        let manager = TaskManager::new(index_service);

        let _ = manager.start_index_all(&config);
        assert!(manager.is_running());

        // Second job should fail
        let result = manager.start_index_all(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already running"));

        // Wait for first job to finish
        while manager.is_running() {
            thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    #[test]
    fn start_rebuild() {
        let (_tmp, index_service, config) = setup();
        let manager = TaskManager::new(index_service);

        let id = manager.start_rebuild(&config).unwrap();
        assert_eq!(id.as_u64(), 1);

        // Wait for completion
        while manager.is_running() {
            thread::sleep(std::time::Duration::from_millis(50));
        }

        let status = manager.job_status().unwrap();
        assert!(status.is_completed() || status.is_failed());
    }

    #[test]
    fn concurrent_status_queries() {
        let (_tmp, index_service, config) = setup();
        let manager = TaskManager::new(index_service);

        let _ = manager.start_index_all(&config);

        // Query status multiple times while running
        for _ in 0..10 {
            let _ = manager.job_status();
            let _ = manager.current_progress();
        }

        while manager.is_running() {
            thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    #[test]
    fn progress_updates_during_indexing() {
        let (_tmp, index_service, config) = setup();
        let manager = TaskManager::new(index_service);

        let _ = manager.start_index_all(&config);

        // Check progress is updated
        let mut saw_running = false;
        for _ in 0..20 {
            if let Some(progress) = manager.current_progress() {
                if progress.status_message.contains("Scanning")
                    || progress.status_message.contains("Committing")
                    || progress.status_message.contains("Starting")
                {
                    saw_running = true;
                }
            }
            if !manager.is_running() {
                break;
            }
            thread::sleep(std::time::Duration::from_millis(50));
        }

        assert!(saw_running, "Should have seen running progress");
    }
}
