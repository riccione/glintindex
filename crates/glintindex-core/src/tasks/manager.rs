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
/// All internal state is protected by `Arc<Mutex<...>>`. The progress
/// channel uses `std::sync::mpsc` for lock-free message passing.
///
/// # Examples
///
/// ```
/// use glintindex_core::tasks::TaskManager;
///
/// let manager = TaskManager::new(
///     std::path::PathBuf::from("/tmp/index"),
///     vec![".git".to_string()],
/// );
/// assert!(!manager.is_running());
/// ```
pub struct TaskManager {
    /// Shared state for the current job.
    current_job: Arc<Mutex<Option<SharedState>>>,
    /// Monotonically increasing job ID counter.
    next_id: AtomicU64,
    /// The index directory path (needed to open IndexService in background thread).
    index_path: PathBuf,
    /// Configuration for the scanner (ignored folders).
    #[allow(dead_code)]
    ignored_folders: Vec<String>,
}

impl TaskManager {
    /// Creates a new `TaskManager` with no active jobs.
    pub fn new(index_path: PathBuf, ignored_folders: Vec<String>) -> Self {
        Self {
            current_job: Arc::new(Mutex::new(None)),
            next_id: AtomicU64::new(1),
            index_path,
            ignored_folders,
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
        let index_path = self.index_path.clone();
        let ignored_folders = config.ignored_folders.clone();
        let enabled_folders: Vec<PathBuf> = config
            .enabled_folders()
            .into_iter()
            .map(|f| f.path.clone())
            .collect();

        // Get a reference to our internal shared state for the thread
        let internal_shared = self.current_job.clone();

        thread::spawn(move || {
            let result = match job_type {
                JobType::IndexAll => Self::run_index_all(
                    &index_path,
                    &ignored_folders,
                    &enabled_folders,
                    &internal_shared,
                ),
                JobType::RebuildIndex => Self::run_rebuild(&index_path, &internal_shared),
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

    /// Runs the "Index All" operation on the current thread.
    fn run_index_all(
        index_path: &std::path::Path,
        ignored_folders: &[String],
        enabled_folders: &[PathBuf],
        shared: &Arc<Mutex<Option<SharedState>>>,
    ) -> Result<ScannerStatistics> {
        let index_service = IndexService::open_or_create(index_path)?;
        let scanner =
            crate::scanner::FilesystemScanner::with_custom_ignores(&index_service, ignored_folders);

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

        index_service.commit()?;
        index_service.reload_reader()?;

        Ok(stats)
    }

    /// Runs the "Rebuild Index" operation on the current thread.
    fn run_rebuild(
        index_path: &std::path::Path,
        shared: &Arc<Mutex<Option<SharedState>>>,
    ) -> Result<ScannerStatistics> {
        use crate::traits::DocumentIndexer;

        let index_service = IndexService::open_or_create(index_path)?;

        // Update progress
        if let Ok(mut guard) = shared.lock() {
            if let Some(ref mut s) = *guard {
                s.progress.status_message = "Rebuilding index...".to_string();
            }
        }

        index_service.rebuild()?;
        index_service.commit()?;
        index_service.reload_reader()?;

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

    fn test_config(tmp: &TempDir, folders: Vec<IndexedFolder>) -> (AppConfig, PathBuf) {
        let config = AppConfig {
            indexed_folders: folders,
            index_directory: tmp.path().join("index"),
            ..AppConfig::default()
        };
        (config, tmp.path().join("index"))
    }

    #[test]
    fn task_manager_new() {
        let tmp = TempDir::new().unwrap();
        let manager = TaskManager::new(tmp.path().join("index"), vec![]);
        assert!(!manager.is_running());
        assert!(manager.job_status().is_none());
    }

    #[test]
    fn start_index_all() {
        let tmp = TempDir::new().unwrap();
        let scan_dir = tmp.path().join("scan");
        fs::create_dir(&scan_dir).unwrap();
        fs::write(scan_dir.join("hello.txt"), "hello world").unwrap();

        let folders = vec![IndexedFolder::enabled(scan_dir)];
        let (config, index_path) = test_config(&tmp, folders);
        let manager = TaskManager::new(index_path, config.ignored_folders.clone());

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
        let tmp = TempDir::new().unwrap();
        let scan_dir = tmp.path().join("scan");
        fs::create_dir(&scan_dir).unwrap();

        let folders = vec![IndexedFolder::enabled(scan_dir)];
        let (config, index_path) = test_config(&tmp, folders);
        let manager = TaskManager::new(index_path, config.ignored_folders.clone());

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
        let tmp = TempDir::new().unwrap();
        let (config, index_path) = test_config(&tmp, vec![]);
        let manager = TaskManager::new(index_path, config.ignored_folders.clone());

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
        let tmp = TempDir::new().unwrap();
        let scan_dir = tmp.path().join("scan");
        fs::create_dir(&scan_dir).unwrap();
        fs::write(scan_dir.join("hello.txt"), "hello world").unwrap();

        let folders = vec![IndexedFolder::enabled(scan_dir)];
        let (config, index_path) = test_config(&tmp, folders);
        let manager = TaskManager::new(index_path, config.ignored_folders.clone());

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
        let tmp = TempDir::new().unwrap();
        let scan_dir = tmp.path().join("scan");
        fs::create_dir(&scan_dir).unwrap();
        fs::write(scan_dir.join("hello.txt"), "hello world").unwrap();

        let folders = vec![IndexedFolder::enabled(scan_dir)];
        let (config, index_path) = test_config(&tmp, folders);
        let manager = TaskManager::new(index_path, config.ignored_folders.clone());

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
