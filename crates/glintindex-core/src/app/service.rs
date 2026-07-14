use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::config::{AppConfig, AppPaths, loader};
use crate::error::{GlintIndexError, Result};
use crate::index::IndexService;
use crate::model::{IndexedFolder, SearchQuery, SearchResult};
use crate::scanner::{FilesystemScanner, NoopReporter, ProgressReporter, ScannerStatistics};
use crate::tasks::{JobId, JobStatus, Progress, TaskManager};
use crate::watcher::FileWatcher;

use super::statistics::ApplicationStatistics;

/// Status information about the filesystem watcher.
///
/// Provides a snapshot of the watcher's current state, including
/// whether it is running and which directories are being monitored.
///
/// # Examples
///
/// ```
/// use glintindex_core::app::WatcherStatus;
///
/// let status = WatcherStatus::new(false, vec![]);
/// assert!(!status.is_running);
/// assert!(status.watched_dirs.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatcherStatus {
    /// Whether the watcher is currently running.
    pub is_running: bool,
    /// Directories currently being monitored.
    pub watched_dirs: Vec<PathBuf>,
}

impl WatcherStatus {
    /// Creates a new `WatcherStatus` with the given values.
    pub fn new(is_running: bool, watched_dirs: Vec<PathBuf>) -> Self {
        Self {
            is_running,
            watched_dirs,
        }
    }
}

/// High-level application service that coordinates configuration, indexing,
/// scanning, searching, and filesystem watching.
///
/// `ApplicationService` is the main entry point for the application layer.
/// It hides internal subsystem details (Tantivy index, walkdir scanning,
/// filesystem notifications) from future GUI and CLI implementations,
/// providing a clean facade over the core functionality.
///
/// The service owns its configuration and index, and creates scanners
/// and watchers on-the-fly for their respective operations.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use glintindex_core::app::ApplicationService;
///
/// let mut service = ApplicationService::with_config_path(Path::new("index.toml")).unwrap();
/// let results = service.search(&glintindex_core::SearchQuery::new("hello")).unwrap();
/// ```
pub struct ApplicationService {
    config: AppConfig,
    config_path: Option<PathBuf>,
    index_service: Arc<Mutex<IndexService>>,
    index_path: PathBuf,
    watcher: Option<FileWatcher>,
    task_manager: TaskManager,
}

impl ApplicationService {
    /// Creates a new application service from an existing configuration.
    ///
    /// Initializes the search index at the path specified in `config`.
    /// If the index does not yet exist, it is created.
    ///
    /// # Errors
    ///
    /// Returns an error if the index cannot be opened or created.
    pub fn new(config: AppConfig) -> Result<Self> {
        let index_service = IndexService::open_or_create(&config.index_directory)?;
        let index_path = index_service.index_path().to_path_buf();
        let index_service = Arc::new(Mutex::new(index_service));
        let task_manager = TaskManager::new(index_service.clone());
        Ok(Self {
            config,
            config_path: None,
            index_service,
            index_path,
            watcher: None,
            task_manager,
        })
    }

    /// Creates a new application service using the default platform
    /// configuration file path.
    ///
    /// This is the primary constructor for both CLI and GUI entry points.
    /// It loads (or creates) the configuration file at the default
    /// platform-specific location via [`AppPaths`].
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration file exists but cannot be
    /// parsed, or if the index cannot be opened or created.
    pub fn with_default_config() -> Result<Self> {
        let paths = AppPaths::new();
        paths.ensure_directories()?;
        Self::with_config_path(&paths.config_file())
    }

    /// Loads configuration from the given TOML file path and initializes
    /// the application service.
    ///
    /// If the configuration file does not exist, default settings are used.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration file exists but cannot be
    /// parsed, or if the index cannot be opened or created.
    pub fn with_config_path(config_path: &Path) -> Result<Self> {
        let config = loader::load(config_path)?;
        let index_service = IndexService::open_or_create(&config.index_directory)?;
        let index_path = index_service.index_path().to_path_buf();
        let index_service = Arc::new(Mutex::new(index_service));
        let task_manager = TaskManager::new(index_service.clone());
        Ok(Self {
            config,
            config_path: Some(config_path.to_path_buf()),
            index_service,
            index_path,
            watcher: None,
            task_manager,
        })
    }

    /// Indexes a single folder, scanning it for supported files and adding
    /// them to the search index.
    ///
    /// Uses the ignore patterns from the application configuration.
    /// Individual file errors are recovered from — one bad file does not
    /// stop the scan.
    ///
    /// # Errors
    ///
    /// Returns an error only if the root directory cannot be read or the
    /// index cannot accept documents.
    pub fn index_folder(&self, folder: &Path) -> Result<ScannerStatistics> {
        self.index_folder_with_progress(folder, &NoopReporter)
    }

    /// Indexes a single folder with progress reporting.
    ///
    /// The reporter is called during file processing to provide
    /// real-time progress feedback.
    pub fn index_folder_with_progress(
        &self,
        folder: &Path,
        reporter: &dyn ProgressReporter,
    ) -> Result<ScannerStatistics> {
        let service = self
            .index_service
            .lock()
            .map_err(|e| GlintIndexError::Other(format!("index service lock poisoned: {e}")))?;
        let scanner =
            FilesystemScanner::with_custom_ignores(&service, &self.config.ignored_folders)
                .with_progress(reporter);
        let stats = scanner.scan_directory(folder)?;
        service.commit()?;
        service.reload_reader()?;
        Ok(stats)
    }

    /// Indexes all enabled folders from the application configuration.
    ///
    /// Returns a vector of per-folder statistics, one entry per enabled
    /// folder that was scanned.
    ///
    /// # Errors
    ///
    /// Returns an error if any folder cannot be scanned or the index
    /// cannot accept documents.
    pub fn index_all(&self) -> Result<Vec<ScannerStatistics>> {
        self.index_all_with_progress(&NoopReporter)
    }

    /// Indexes all enabled folders with progress reporting.
    ///
    /// The reporter is called during file processing to provide
    /// real-time progress feedback.
    pub fn index_all_with_progress(
        &self,
        reporter: &dyn ProgressReporter,
    ) -> Result<Vec<ScannerStatistics>> {
        let service = self
            .index_service
            .lock()
            .map_err(|e| GlintIndexError::Other(format!("index service lock poisoned: {e}")))?;
        let scanner =
            FilesystemScanner::with_custom_ignores(&service, &self.config.ignored_folders)
                .with_progress(reporter);
        let folders: Vec<PathBuf> = self
            .config
            .enabled_folders()
            .into_iter()
            .map(|f| f.path.clone())
            .collect();
        let stats = scanner.scan_directories(&folders)?;
        service.commit()?;
        service.reload_reader()?;
        Ok(vec![stats])
    }

    /// Executes a search query against the index and returns matching results.
    ///
    /// Delegates to the underlying search engine. Results are ordered by
    /// relevance score (highest first).
    ///
    /// # Errors
    ///
    /// Returns an error if the query cannot be parsed or the search fails.
    pub fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        use crate::traits::SearchEngine;
        let service = self
            .index_service
            .lock()
            .map_err(|e| GlintIndexError::Other(format!("index service lock poisoned: {e}")))?;
        service.search(query)
    }

    /// Rebuilds the entire search index from scratch.
    ///
    /// All existing index data is discarded. After calling this method,
    /// the index will be empty until documents are re-indexed.
    ///
    /// # Errors
    ///
    /// Returns an error if the index cannot be rebuilt.
    pub fn rebuild_index(&self) -> Result<()> {
        use crate::traits::DocumentIndexer;
        let service = self
            .index_service
            .lock()
            .map_err(|e| GlintIndexError::Other(format!("index service lock poisoned: {e}")))?;
        service.rebuild()?;
        service.commit()?;
        service.reload_reader()?;
        Ok(())
    }

    /// Returns application-level statistics combining index and folder
    /// information.
    ///
    /// The statistics include the number of indexed documents, the number
    /// of configured indexed folders, and the last indexing result (if any).
    ///
    /// # Errors
    ///
    /// Returns an error if index statistics cannot be retrieved.
    pub fn statistics(&self) -> Result<ApplicationStatistics> {
        let service = self
            .index_service
            .lock()
            .map_err(|e| GlintIndexError::Other(format!("index service lock poisoned: {e}")))?;
        let index_stats = service.statistics()?;
        Ok(ApplicationStatistics::new(
            index_stats.indexed_documents,
            self.config.indexed_folders.len() as u64,
        ))
    }

    /// Clears all indexed documents from the search index.
    ///
    /// Preserves the index structure, configuration, and indexed folders.
    /// After calling this method, the index will be empty.
    ///
    /// # Errors
    ///
    /// Returns an error if the index cannot be cleared or committed.
    pub fn clear_index(&self) -> Result<()> {
        use crate::traits::DocumentIndexer;
        let service = self
            .index_service
            .lock()
            .map_err(|e| GlintIndexError::Other(format!("index service lock poisoned: {e}")))?;
        service.rebuild()?;
        service.commit()?;
        service.reload_reader()?;
        Ok(())
    }

    /// Starts watching all enabled configured folders for filesystem changes.
    ///
    /// When files are created, modified, or deleted in watched directories,
    /// the search index is automatically and incrementally updated.
    ///
    /// If the watcher is already running, this method is a no-op and
    /// returns `Ok(())`.
    ///
    /// # Errors
    ///
    /// Returns an error if the watcher cannot be created or started.
    pub fn start_watching(&mut self) -> Result<()> {
        if self.watcher.is_some() {
            return Ok(());
        }

        let mut watcher = FileWatcher::with_custom_ignores(
            self.index_service.clone(),
            &self.config.ignored_folders,
        )?;

        for folder in self.config.enabled_folders() {
            watcher.watch_dir(&folder.path)?;
        }

        watcher.start()?;
        self.watcher = Some(watcher);
        Ok(())
    }

    /// Stops watching all configured folders for filesystem changes.
    ///
    /// The watcher is fully shut down and removed. To resume watching,
    /// call [`start_watching`](Self::start_watching) again.
    ///
    /// If the watcher is not running, this method is a no-op and
    /// returns `Ok(())`.
    ///
    /// # Errors
    ///
    /// Returns an error if the watcher cannot be stopped.
    pub fn stop_watching(&mut self) -> Result<()> {
        if let Some(mut watcher) = self.watcher.take() {
            watcher.stop()?;
        }
        Ok(())
    }

    /// Returns the current status of the filesystem watcher.
    ///
    /// Provides information about whether the watcher is running and
    /// which directories are being monitored.
    pub fn watcher_status(&self) -> WatcherStatus {
        match &self.watcher {
            Some(watcher) => {
                WatcherStatus::new(watcher.is_running(), watcher.watched_dirs().to_vec())
            }
            None => WatcherStatus::new(false, vec![]),
        }
    }

    /// Adds a folder to the indexed folders configuration.
    ///
    /// The path is canonicalized to an absolute path. If the folder
    /// is already configured, returns an error.
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be resolved or is already configured.
    pub fn add_folder(&mut self, path: &Path) -> Result<()> {
        let resolved = path
            .canonicalize()
            .map_err(|e| GlintIndexError::InvalidInput(format!("cannot resolve path: {e}")))?;

        if self
            .config
            .indexed_folders
            .iter()
            .any(|f| f.path == resolved)
        {
            return Err(GlintIndexError::InvalidInput(format!(
                "folder already configured: {}",
                resolved.display()
            )));
        }

        self.config
            .indexed_folders
            .push(IndexedFolder::enabled(resolved));
        self.save_config()?;
        Ok(())
    }

    /// Removes a folder from the indexed folders configuration.
    ///
    /// The path is canonicalized before comparison. If the folder
    /// is not configured, returns an error. Does not modify the
    /// existing search index.
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be resolved or is not configured.
    pub fn remove_folder(&mut self, path: &Path) -> Result<()> {
        let resolved = path
            .canonicalize()
            .map_err(|e| GlintIndexError::InvalidInput(format!("cannot resolve path: {e}")))?;

        let before = self.config.indexed_folders.len();
        self.config.indexed_folders.retain(|f| f.path != resolved);

        if self.config.indexed_folders.len() == before {
            return Err(GlintIndexError::InvalidInput(format!(
                "folder not configured: {}",
                resolved.display()
            )));
        }

        self.save_config()?;
        Ok(())
    }

    /// Enables a folder in the indexed folders configuration.
    ///
    /// The path is canonicalized before comparison. If the folder
    /// is not configured, returns an error. Does not trigger indexing.
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be resolved or is not configured.
    pub fn enable_folder(&mut self, path: &Path) -> Result<()> {
        let resolved = path
            .canonicalize()
            .map_err(|e| GlintIndexError::InvalidInput(format!("cannot resolve path: {e}")))?;

        let folder = self
            .config
            .indexed_folders
            .iter_mut()
            .find(|f| f.path == resolved)
            .ok_or_else(|| {
                GlintIndexError::InvalidInput(format!(
                    "folder not configured: {}",
                    resolved.display()
                ))
            })?;

        folder.enabled = true;
        self.save_config()?;
        Ok(())
    }

    /// Disables a folder in the indexed folders configuration.
    ///
    /// The path is canonicalized before comparison. If the folder
    /// is not configured, returns an error. Does not remove indexed
    /// documents from the search index.
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be resolved or is not configured.
    pub fn disable_folder(&mut self, path: &Path) -> Result<()> {
        let resolved = path
            .canonicalize()
            .map_err(|e| GlintIndexError::InvalidInput(format!("cannot resolve path: {e}")))?;

        let folder = self
            .config
            .indexed_folders
            .iter_mut()
            .find(|f| f.path == resolved)
            .ok_or_else(|| {
                GlintIndexError::InvalidInput(format!(
                    "folder not configured: {}",
                    resolved.display()
                ))
            })?;

        folder.enabled = false;
        self.save_config()?;
        Ok(())
    }

    /// Saves the current configuration to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if no config path is set or if the save fails.
    fn save_config(&self) -> Result<()> {
        let config_path = self
            .config_path
            .as_ref()
            .ok_or_else(|| GlintIndexError::Config("no configuration file path set".into()))?;
        loader::save(config_path, &self.config)
    }

    /// Returns a reference to the application configuration.
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    /// Returns references to all configured indexed folders.
    pub fn indexed_folders(&self) -> Vec<&IndexedFolder> {
        self.config.indexed_folders.iter().collect()
    }

    /// Returns references to only the enabled indexed folders.
    pub fn enabled_folders(&self) -> Vec<&IndexedFolder> {
        self.config.enabled_folders()
    }

    /// Returns the path where the search index is stored.
    pub fn index_path(&self) -> &Path {
        &self.index_path
    }

    // ── Background Job Methods ──────────────────────────────────────

    /// Starts indexing all enabled folders on a background thread.
    ///
    /// Returns the job ID immediately. The actual indexing happens
    /// on a background thread. Use `job_status()` or
    /// `current_progress()` to monitor progress.
    ///
    /// Only one job may run at a time. Attempting to start a second
    /// job while one is already running returns an error.
    ///
    /// # Errors
    ///
    /// Returns an error if a job is already running.
    pub fn start_indexing(&self) -> Result<JobId> {
        self.task_manager.start_index_all(&self.config)
    }

    /// Starts rebuilding the index on a background thread.
    ///
    /// Returns the job ID immediately. The actual rebuild happens
    /// on a background thread.
    ///
    /// Only one job may run at a time. Attempting to start a second
    /// job while one is already running returns an error.
    ///
    /// # Errors
    ///
    /// Returns an error if a job is already running.
    pub fn start_rebuild(&self) -> Result<JobId> {
        self.task_manager.start_rebuild(&self.config)
    }

    /// Returns the status of the current background job, if any.
    ///
    /// Returns `None` if no job has been started or the last job
    /// has been cleaned up.
    pub fn job_status(&self) -> Option<JobStatus> {
        self.task_manager.job_status()
    }

    /// Returns the current progress of the background job, if any.
    pub fn current_progress(&self) -> Option<Progress> {
        self.task_manager.current_progress()
    }

    /// Returns `true` if a background job is currently running.
    pub fn is_indexing(&self) -> bool {
        self.task_manager.is_running()
    }

    /// Returns a reference to the list of ignored folder names.
    ///
    /// These folder names are excluded from indexing operations.
    /// The default set includes `.git`, `.svn`, `.hg`, `node_modules`,
    /// `__pycache__`, and `.DS_Store`.
    pub fn ignored_folders(&self) -> &[String] {
        &self.config.ignored_folders
    }

    /// Adds a folder name to the ignored folders list.
    ///
    /// If the name is already in the list, returns `Ok(())` without
    /// modification. Changes are persisted to the configuration file
    /// immediately.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be saved.
    pub fn add_ignored_folder(&mut self, name: String) -> Result<()> {
        if self.config.ignored_folders.contains(&name) {
            return Ok(());
        }
        self.config.ignored_folders.push(name);
        self.save_config()?;
        Ok(())
    }

    /// Removes a folder name from the ignored folders list.
    ///
    /// If the name is not in the list, returns an error. Changes are
    /// persisted to the configuration file immediately.
    ///
    /// # Errors
    ///
    /// Returns an error if the name is not found or the configuration
    /// cannot be saved.
    pub fn remove_ignored_folder(&mut self, name: &str) -> Result<()> {
        let before = self.config.ignored_folders.len();
        self.config.ignored_folders.retain(|n| n != name);

        if self.config.ignored_folders.len() == before {
            return Err(GlintIndexError::InvalidInput(format!(
                "ignored folder not found: {name}"
            )));
        }

        self.save_config()?;
        Ok(())
    }

    /// Adds a query to the recent searches list.
    ///
    /// The query is added to the front of the list (newest first).
    /// Empty queries are ignored. Duplicates are removed.
    /// The list is limited to 20 entries maximum.
    /// Changes are persisted to the configuration file immediately.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be saved.
    pub fn add_recent_search(&mut self, query: String) -> Result<()> {
        self.config.add_recent_search(query);
        self.save_config()?;
        Ok(())
    }

    /// Returns a reference to the recent searches list.
    ///
    /// The list is ordered from newest to oldest, with a maximum
    /// of 20 entries.
    pub fn recent_searches(&self) -> &[String] {
        self.config.recent_searches()
    }

    /// Updates the application font size and persists the change.
    ///
    /// The value is clamped to the valid range (8–32) before saving.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be saved.
    pub fn set_font_size(&mut self, size: u32) -> Result<()> {
        self.config.font_size = size.clamp(8, 32);
        self.save_config()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::IndexedFolder;
    use std::fs;
    use tempfile::TempDir;

    fn test_config(tmp: &TempDir) -> AppConfig {
        AppConfig {
            index_directory: tmp.path().join("index"),
            ..AppConfig::default()
        }
    }

    fn indexed_folder_config(tmp: &TempDir, folders: Vec<IndexedFolder>) -> AppConfig {
        AppConfig {
            indexed_folders: folders,
            index_directory: tmp.path().join("index"),
            ..AppConfig::default()
        }
    }

    #[test]
    fn create_service_from_config() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);
        let service = ApplicationService::new(config).unwrap();
        assert!(service.index_path().exists());
    }

    #[test]
    fn create_service_with_config_path() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");
        crate::config::loader::save(&config_path, &AppConfig::default()).unwrap();
        let service = ApplicationService::with_config_path(&config_path).unwrap();
        assert!(service.index_path().exists());
    }

    #[test]
    fn create_service_missing_config_uses_defaults() {
        let config = crate::config::loader::load(std::path::Path::new("nonexistent.toml")).unwrap();
        assert!(config.indexed_folders.is_empty());
        assert!(!config.ignored_folders.is_empty());
    }

    #[test]
    fn index_single_folder() {
        let tmp = TempDir::new().unwrap();
        let scan_dir = tmp.path().join("scan");
        fs::create_dir(&scan_dir).unwrap();
        fs::write(scan_dir.join("hello.txt"), "hello world").unwrap();

        let config = test_config(&tmp);
        let service = ApplicationService::new(config).unwrap();
        let stats = service.index_folder(&scan_dir).unwrap();
        assert_eq!(stats.files_indexed, 1);
    }

    #[test]
    fn index_folder_with_ignored_dirs() {
        let tmp = TempDir::new().unwrap();
        let scan_dir = tmp.path().join("scan");
        fs::create_dir_all(scan_dir.join(".git/objects")).unwrap();
        fs::write(scan_dir.join("good.txt"), "content").unwrap();
        fs::write(scan_dir.join(".git/objects/abc"), "git object").unwrap();

        let config = test_config(&tmp);
        let service = ApplicationService::new(config).unwrap();
        let stats = service.index_folder(&scan_dir).unwrap();
        assert_eq!(stats.files_indexed, 1);
    }

    #[test]
    fn index_all_enabled_folders() {
        let tmp = TempDir::new().unwrap();
        let dir_a = tmp.path().join("a");
        let dir_b = tmp.path().join("b");
        fs::create_dir(&dir_a).unwrap();
        fs::create_dir(&dir_b).unwrap();
        fs::write(dir_a.join("file1.txt"), "one").unwrap();
        fs::write(dir_b.join("file2.txt"), "two").unwrap();

        let folders = vec![IndexedFolder::enabled(dir_a), IndexedFolder::enabled(dir_b)];
        let config = indexed_folder_config(&tmp, folders);
        let service = ApplicationService::new(config).unwrap();
        let results = service.index_all().unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].files_indexed, 2);
    }

    #[test]
    fn index_all_skips_disabled_folders() {
        let tmp = TempDir::new().unwrap();
        let dir_a = tmp.path().join("a");
        let dir_b = tmp.path().join("b");
        fs::create_dir(&dir_a).unwrap();
        fs::create_dir(&dir_b).unwrap();
        fs::write(dir_a.join("file1.txt"), "one").unwrap();
        fs::write(dir_b.join("file2.txt"), "two").unwrap();

        let folders = vec![
            IndexedFolder::enabled(dir_a),
            IndexedFolder::disabled(dir_b),
        ];
        let config = indexed_folder_config(&tmp, folders);
        let service = ApplicationService::new(config).unwrap();
        let results = service.index_all().unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].files_indexed, 1);
    }

    #[test]
    fn search_after_indexing() {
        let tmp = TempDir::new().unwrap();
        let scan_dir = tmp.path().join("scan");
        fs::create_dir(&scan_dir).unwrap();
        fs::write(scan_dir.join("hello.txt"), "hello world").unwrap();

        let config = test_config(&tmp);
        let service = ApplicationService::new(config).unwrap();
        service.index_folder(&scan_dir).unwrap();

        let query = SearchQuery::new("hello");
        let results = service.search(&query).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].document.filename(), "hello.txt");
    }

    #[test]
    fn search_empty_index() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);
        let service = ApplicationService::new(config).unwrap();

        let query = SearchQuery::new("anything");
        let results = service.search(&query).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn rebuild_index_clears_data() {
        let tmp = TempDir::new().unwrap();
        let scan_dir = tmp.path().join("scan");
        fs::create_dir(&scan_dir).unwrap();
        fs::write(scan_dir.join("file.txt"), "content").unwrap();

        let config = test_config(&tmp);
        let service = ApplicationService::new(config).unwrap();
        service.index_folder(&scan_dir).unwrap();

        let stats = service.statistics().unwrap();
        assert_eq!(stats.indexed_documents, 1);

        service.rebuild_index().unwrap();
        let stats = service.statistics().unwrap();
        assert_eq!(stats.indexed_documents, 0);
    }

    #[test]
    fn statistics_reflect_indexed_state() {
        let tmp = TempDir::new().unwrap();
        let scan_dir = tmp.path().join("scan");
        fs::create_dir(&scan_dir).unwrap();
        fs::write(scan_dir.join("a.txt"), "aaa").unwrap();
        fs::write(scan_dir.join("b.txt"), "bbb").unwrap();

        let folders = vec![IndexedFolder::enabled(scan_dir)];
        let config = indexed_folder_config(&tmp, folders);
        let service = ApplicationService::new(config).unwrap();
        service.index_all().unwrap();

        let stats = service.statistics().unwrap();
        assert_eq!(stats.indexed_documents, 2);
        assert_eq!(stats.indexed_folders, 1);
    }

    #[test]
    fn config_returns_read_only_reference() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);
        let service = ApplicationService::new(config).unwrap();
        let cfg = service.config();
        assert!(cfg.indexed_folders.is_empty());
        assert!(!cfg.ignored_folders.is_empty());
    }

    #[test]
    fn indexed_folders_returns_all() {
        let tmp = TempDir::new().unwrap();
        let folders = vec![
            IndexedFolder::enabled(tmp.path().join("a")),
            IndexedFolder::disabled(tmp.path().join("b")),
        ];
        let config = indexed_folder_config(&tmp, folders);
        let service = ApplicationService::new(config).unwrap();
        assert_eq!(service.indexed_folders().len(), 2);
    }

    #[test]
    fn enabled_folders_returns_only_enabled() {
        let tmp = TempDir::new().unwrap();
        let folders = vec![
            IndexedFolder::enabled(tmp.path().join("a")),
            IndexedFolder::disabled(tmp.path().join("b")),
        ];
        let config = indexed_folder_config(&tmp, folders);
        let service = ApplicationService::new(config).unwrap();
        assert_eq!(service.enabled_folders().len(), 1);
    }

    #[test]
    fn clear_index_removes_documents() {
        let tmp = TempDir::new().unwrap();
        let scan_dir = tmp.path().join("scan");
        fs::create_dir(&scan_dir).unwrap();
        fs::write(scan_dir.join("file.txt"), "content").unwrap();

        let config = test_config(&tmp);
        let service = ApplicationService::new(config).unwrap();
        service.index_folder(&scan_dir).unwrap();
        assert_eq!(service.statistics().unwrap().indexed_documents, 1);

        service.clear_index().unwrap();
        assert_eq!(service.statistics().unwrap().indexed_documents, 0);
    }

    #[test]
    fn add_folder_adds_to_config() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");
        let test_dir = tmp.path().join("docs");
        fs::create_dir(&test_dir).unwrap();

        let config = AppConfig {
            index_directory: tmp.path().join("index"),
            ..Default::default()
        };
        crate::config::loader::save(&config_path, &config).unwrap();

        let mut service = ApplicationService::with_config_path(&config_path).unwrap();
        service.add_folder(&test_dir).unwrap();

        let loaded = crate::config::loader::load(&config_path).unwrap();
        assert_eq!(loaded.indexed_folders.len(), 1);
        assert!(loaded.indexed_folders[0].enabled);
    }

    #[test]
    fn add_folder_rejects_duplicate() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");
        let test_dir = tmp.path().join("docs");
        fs::create_dir(&test_dir).unwrap();

        let resolved = test_dir.canonicalize().unwrap();
        let config = AppConfig {
            indexed_folders: vec![IndexedFolder::enabled(resolved)],
            index_directory: tmp.path().join("index"),
            ..Default::default()
        };
        crate::config::loader::save(&config_path, &config).unwrap();

        let mut service = ApplicationService::with_config_path(&config_path).unwrap();
        let result = service.add_folder(&test_dir);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("already configured")
        );
    }

    #[test]
    fn add_folder_rejects_invalid_path() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");

        let config = AppConfig {
            index_directory: tmp.path().join("index"),
            ..Default::default()
        };
        crate::config::loader::save(&config_path, &config).unwrap();

        let mut service = ApplicationService::with_config_path(&config_path).unwrap();
        let result = service.add_folder(Path::new("/nonexistent/path/that/does/not/exist"));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("cannot resolve path")
        );
    }

    #[test]
    fn remove_folder_removes_from_config() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");
        let test_dir = tmp.path().join("docs");
        fs::create_dir(&test_dir).unwrap();

        let resolved = test_dir.canonicalize().unwrap();
        let config = AppConfig {
            indexed_folders: vec![IndexedFolder::enabled(resolved)],
            index_directory: tmp.path().join("index"),
            ..Default::default()
        };
        crate::config::loader::save(&config_path, &config).unwrap();

        let mut service = ApplicationService::with_config_path(&config_path).unwrap();
        service.remove_folder(&test_dir).unwrap();

        let loaded = crate::config::loader::load(&config_path).unwrap();
        assert!(loaded.indexed_folders.is_empty());
    }

    #[test]
    fn remove_folder_rejects_unconfigured() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");
        let test_dir = tmp.path().join("docs");
        fs::create_dir(&test_dir).unwrap();

        let config = AppConfig {
            index_directory: tmp.path().join("index"),
            ..Default::default()
        };
        crate::config::loader::save(&config_path, &config).unwrap();

        let mut service = ApplicationService::with_config_path(&config_path).unwrap();
        let result = service.remove_folder(&test_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not configured"));
    }

    #[test]
    fn enable_folder_sets_enabled() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");
        let test_dir = tmp.path().join("docs");
        fs::create_dir(&test_dir).unwrap();

        let resolved = test_dir.canonicalize().unwrap();
        let config = AppConfig {
            indexed_folders: vec![IndexedFolder::disabled(resolved)],
            index_directory: tmp.path().join("index"),
            ..Default::default()
        };
        crate::config::loader::save(&config_path, &config).unwrap();

        let mut service = ApplicationService::with_config_path(&config_path).unwrap();
        service.enable_folder(&test_dir).unwrap();

        let loaded = crate::config::loader::load(&config_path).unwrap();
        assert!(loaded.indexed_folders[0].enabled);
    }

    #[test]
    fn disable_folder_sets_disabled() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");
        let test_dir = tmp.path().join("docs");
        fs::create_dir(&test_dir).unwrap();

        let resolved = test_dir.canonicalize().unwrap();
        let config = AppConfig {
            indexed_folders: vec![IndexedFolder::enabled(resolved)],
            index_directory: tmp.path().join("index"),
            ..Default::default()
        };
        crate::config::loader::save(&config_path, &config).unwrap();

        let mut service = ApplicationService::with_config_path(&config_path).unwrap();
        service.disable_folder(&test_dir).unwrap();

        let loaded = crate::config::loader::load(&config_path).unwrap();
        assert!(!loaded.indexed_folders[0].enabled);
    }

    #[test]
    fn enable_folder_rejects_unconfigured() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");
        let test_dir = tmp.path().join("docs");
        fs::create_dir(&test_dir).unwrap();

        let config = AppConfig {
            index_directory: tmp.path().join("index"),
            ..Default::default()
        };
        crate::config::loader::save(&config_path, &config).unwrap();

        let mut service = ApplicationService::with_config_path(&config_path).unwrap();
        let result = service.enable_folder(&test_dir);
        assert!(result.is_err());
    }

    #[test]
    fn disable_folder_rejects_unconfigured() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");
        let test_dir = tmp.path().join("docs");
        fs::create_dir(&test_dir).unwrap();

        let config = AppConfig {
            index_directory: tmp.path().join("index"),
            ..Default::default()
        };
        crate::config::loader::save(&config_path, &config).unwrap();

        let mut service = ApplicationService::with_config_path(&config_path).unwrap();
        let result = service.disable_folder(&test_dir);
        assert!(result.is_err());
    }

    #[test]
    fn save_config_fails_without_path() {
        let tmp = TempDir::new().unwrap();
        let test_dir = tmp.path().join("docs");
        fs::create_dir(&test_dir).unwrap();

        let config = AppConfig {
            index_directory: tmp.path().join("index"),
            ..Default::default()
        };
        let mut service = ApplicationService::new(config).unwrap();
        let result = service.add_folder(&test_dir);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("no configuration file")
        );
    }

    #[test]
    fn start_and_stop_watching() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);
        let mut service = ApplicationService::new(config).unwrap();

        let status = service.watcher_status();
        assert!(!status.is_running);

        service.start_watching().unwrap();
        let status = service.watcher_status();
        assert!(status.is_running);

        service.stop_watching().unwrap();
        let status = service.watcher_status();
        assert!(!status.is_running);
    }

    #[test]
    fn start_watching_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);
        let mut service = ApplicationService::new(config).unwrap();

        service.start_watching().unwrap();
        service.start_watching().unwrap();
        assert!(service.watcher_status().is_running);

        service.stop_watching().unwrap();
    }

    #[test]
    fn stop_watching_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);
        let mut service = ApplicationService::new(config).unwrap();

        service.stop_watching().unwrap();
        service.stop_watching().unwrap();
        assert!(!service.watcher_status().is_running);
    }

    #[test]
    fn watcher_status_shows_watched_dirs() {
        let tmp = TempDir::new().unwrap();
        let scan_dir = tmp.path().join("scan");
        fs::create_dir(&scan_dir).unwrap();

        let folders = vec![IndexedFolder::enabled(scan_dir.clone())];
        let config = indexed_folder_config(&tmp, folders);
        let mut service = ApplicationService::new(config).unwrap();

        service.start_watching().unwrap();
        let status = service.watcher_status();
        assert!(status.is_running);
        assert_eq!(status.watched_dirs.len(), 1);
        assert_eq!(status.watched_dirs[0], scan_dir);

        service.stop_watching().unwrap();
    }

    #[test]
    fn watcher_status_skips_disabled_folders() {
        let tmp = TempDir::new().unwrap();
        let dir_a = tmp.path().join("a");
        let dir_b = tmp.path().join("b");
        fs::create_dir(&dir_a).unwrap();
        fs::create_dir(&dir_b).unwrap();

        let folders = vec![
            IndexedFolder::enabled(dir_a.clone()),
            IndexedFolder::disabled(dir_b),
        ];
        let config = indexed_folder_config(&tmp, folders);
        let mut service = ApplicationService::new(config).unwrap();

        service.start_watching().unwrap();
        let status = service.watcher_status();
        assert_eq!(status.watched_dirs.len(), 1);
        assert_eq!(status.watched_dirs[0], dir_a);

        service.stop_watching().unwrap();
    }

    #[test]
    fn watcher_status_default_when_not_watching() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);
        let service = ApplicationService::new(config).unwrap();

        let status = service.watcher_status();
        assert!(!status.is_running);
        assert!(status.watched_dirs.is_empty());
    }
}
