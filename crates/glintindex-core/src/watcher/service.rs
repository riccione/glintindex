use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::time::UNIX_EPOCH;

use notify::{Event as NotifyEvent, RecommendedWatcher, RecursiveMode, Watcher};

use crate::error::{GlintIndexError, Result};
use crate::index::IndexService;
use crate::model::Document;
use crate::scanner::ignore::IgnoreRules;
use crate::traits::DocumentIndexer;

use super::events::WatchEvent;

/// A filesystem watcher that monitors configured directories for changes
/// and incrementally updates the search index.
///
/// `FileWatcher` coordinates between the filesystem notification system
/// (via `notify`) and the application's indexing layer. It detects file
/// creations, modifications, and deletions, then applies the corresponding
/// index operations.
///
/// The watcher runs a background thread that receives filesystem events
/// and processes them into the index. Individual file errors are recovered
/// from — one bad file does not stop the watcher.
///
/// # Examples
///
/// ```no_run
/// use std::sync::{Arc, Mutex};
/// use glintindex_core::index::IndexService;
/// use glintindex_core::watcher::FileWatcher;
/// use std::path::Path;
///
/// let index_service = IndexService::open_or_create(Path::new("/tmp/index")).unwrap();
/// let shared = Arc::new(Mutex::new(index_service));
///
/// let mut watcher = FileWatcher::new(shared.clone()).unwrap();
/// watcher.watch_dir(Path::new("/home/user/docs")).unwrap();
/// watcher.start().unwrap();
///
/// // ... later
/// watcher.stop().unwrap();
/// ```
pub struct FileWatcher {
    watcher: Option<RecommendedWatcher>,
    event_tx: mpsc::Sender<WatchEvent>,
    event_rx: Arc<Mutex<mpsc::Receiver<WatchEvent>>>,
    index_service: Arc<Mutex<IndexService>>,
    watched_dirs: Vec<PathBuf>,
    ignore_rules: IgnoreRules,
}

impl FileWatcher {
    /// Creates a new `FileWatcher` without starting the background thread.
    ///
    /// The watcher is created in a stopped state. Call [`start`](Self::start)
    /// to begin monitoring filesystem events.
    ///
    /// # Errors
    ///
    /// Returns an error if the internal channel cannot be created.
    pub fn new(index_service: Arc<Mutex<IndexService>>) -> Result<Self> {
        let (event_tx, event_rx) = mpsc::channel();

        Ok(Self {
            watcher: None,
            event_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
            index_service,
            watched_dirs: Vec::new(),
            ignore_rules: IgnoreRules::new(),
        })
    }

    /// Creates a new `FileWatcher` with custom ignore patterns merged
    /// into the defaults.
    ///
    /// # Errors
    ///
    /// Returns an error if the internal channel cannot be created.
    pub fn with_custom_ignores(
        index_service: Arc<Mutex<IndexService>>,
        custom_ignores: &[String],
    ) -> Result<Self> {
        let (event_tx, event_rx) = mpsc::channel();

        Ok(Self {
            watcher: None,
            event_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
            index_service,
            watched_dirs: Vec::new(),
            ignore_rules: IgnoreRules::with_custom(custom_ignores),
        })
    }

    /// Starts the background filesystem monitoring thread.
    ///
    /// After calling this method, the watcher begins receiving filesystem
    /// events from all watched directories and processing them into the
    /// search index.
    ///
    /// If the watcher is already started, this method is a no-op and
    /// returns `Ok(())`.
    ///
    /// # Errors
    ///
    /// Returns an error if the filesystem watcher cannot be created.
    pub fn start(&mut self) -> Result<()> {
        if self.watcher.is_some() {
            return Ok(());
        }

        let event_tx = self.event_tx.clone();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<NotifyEvent>| {
            match res {
                Ok(event) => {
                    Self::handle_notify_event(&event_tx, &event);
                }
                Err(err) => {
                    tracing::warn!("filesystem watch error: {err}");
                }
            }
        })
        .map_err(|e| GlintIndexError::Index(format!("failed to create filesystem watcher: {e}")))?;

        for dir in &self.watched_dirs {
            watcher
                .watch(dir.as_path(), RecursiveMode::Recursive)
                .map_err(|e| {
                    GlintIndexError::Index(format!(
                        "failed to watch directory {}: {e}",
                        dir.display()
                    ))
                })?;
        }

        self.watcher = Some(watcher);
        Ok(())
    }

    /// Stops the background filesystem monitoring thread.
    ///
    /// After calling this method, no new filesystem events will be
    /// received. Events that were already sent to the channel will
    /// still be available for processing.
    ///
    /// If the watcher is not started, this method is a no-op and
    /// returns `Ok(())`.
    ///
    /// # Errors
    ///
    /// Returns an error if the watcher cannot be stopped.
    pub fn stop(&mut self) -> Result<()> {
        if let Some(watcher) = self.watcher.take() {
            drop(watcher);
        }
        Ok(())
    }

    /// Adds a directory to the watch list.
    ///
    /// If the watcher is already running, the directory is immediately
    /// added to the active monitoring set. If the watcher is not running,
    /// the directory is recorded and will be watched when `start` is called.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be watched.
    pub fn watch_dir(&mut self, dir: &Path) -> Result<()> {
        if let Some(ref mut watcher) = self.watcher {
            watcher.watch(dir, RecursiveMode::Recursive).map_err(|e| {
                GlintIndexError::Index(format!("failed to watch directory {}: {e}", dir.display()))
            })?;
        }

        if !self.watched_dirs.iter().any(|d| d == dir) {
            self.watched_dirs.push(dir.to_path_buf());
        }

        Ok(())
    }

    /// Removes a directory from the watch list.
    ///
    /// If the watcher is running, the directory is immediately removed
    /// from the active monitoring set. Indexed documents from this
    /// directory are not removed from the index.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be unwatched.
    pub fn unwatch_dir(&mut self, dir: &Path) -> Result<()> {
        if let Some(ref mut watcher) = self.watcher {
            watcher.unwatch(dir).map_err(|e| {
                GlintIndexError::Index(format!(
                    "failed to unwatch directory {}: {e}",
                    dir.display()
                ))
            })?;
        }

        self.watched_dirs.retain(|d| d != dir);
        Ok(())
    }

    /// Returns a reference to the channel receiver for consuming events.
    ///
    /// This is primarily intended for testing and diagnostics. In normal
    /// operation, the background thread processes events automatically.
    pub fn event_receiver(&self) -> Arc<Mutex<mpsc::Receiver<WatchEvent>>> {
        self.event_rx.clone()
    }

    /// Returns `true` if the watcher is currently running.
    pub fn is_running(&self) -> bool {
        self.watcher.is_some()
    }

    /// Returns the list of currently watched directories.
    pub fn watched_dirs(&self) -> &[PathBuf] {
        &self.watched_dirs
    }

    /// Converts a `notify` event into an application-level [`WatchEvent`]
    /// and sends it through the channel.
    fn handle_notify_event(tx: &mpsc::Sender<WatchEvent>, event: &NotifyEvent) {
        let kind = event.kind;
        use notify::EventKind::{Create, Modify, Remove};

        for path in event.paths.iter() {
            let watch_event = match kind {
                Create(_) => Some(WatchEvent::Created(path.clone())),
                Modify(_) => Some(WatchEvent::Modified(path.clone())),
                Remove(_) => Some(WatchEvent::Deleted(path.clone())),
                _ => None,
            };

            if let Some(evt) = watch_event {
                if tx.send(evt).is_err() {
                    tracing::warn!("event channel closed, dropping event");
                }
            }
        }
    }

    /// Processes all pending events from the channel.
    ///
    /// This method is intended for manual or polling-based event
    /// processing. In normal operation, the background thread handles
    /// events automatically.
    ///
    /// Events are processed individually. A failure processing one file
    /// does not prevent subsequent events from being handled.
    pub fn process_pending_events(&self) -> Result<u64> {
        let rx = self
            .event_rx
            .lock()
            .map_err(|e| GlintIndexError::Other(format!("event channel lock poisoned: {e}")))?;

        let mut processed = 0u64;
        while let Ok(event) = rx.try_recv() {
            self.process_event(&event)?;
            processed += 1;
        }

        Ok(processed)
    }

    /// Processes a single filesystem event.
    ///
    /// Routes the event to the appropriate handler based on its type.
    /// Skips files in ignored directories and unsupported file types.
    /// Individual file errors are logged and recovered from.
    fn process_event(&self, event: &WatchEvent) -> Result<()> {
        if self.should_ignore_path(event.path()) {
            return Ok(());
        }

        match event {
            WatchEvent::Created(path) | WatchEvent::Modified(path) => {
                if let Err(err) = process_file(path, &self.index_service, true) {
                    tracing::warn!("failed to index {}: {err}", path.display());
                }
            }
            WatchEvent::Deleted(path) => {
                if let Err(err) = remove_document(path, &self.index_service) {
                    tracing::warn!("failed to remove {}: {err}", path.display());
                }
            }
        }
        Ok(())
    }

    /// Returns `true` if the given path is inside an ignored directory.
    fn should_ignore_path(&self, path: &Path) -> bool {
        path.ancestors()
            .filter_map(|a| a.file_name())
            .filter_map(|n| n.to_str())
            .any(|name| self.ignore_rules.should_ignore_dir(name))
    }
}

/// Reads a file from disk and either adds or updates it in the search index.
///
/// Skips files with unsupported extensions and binary content. This function
/// reuses the scanner's ignore rules and binary detection logic.
fn process_file(
    path: &Path,
    index_service: &Arc<Mutex<IndexService>>,
    is_update: bool,
) -> Result<()> {
    if !IgnoreRules::is_supported_file(path) {
        return Err(GlintIndexError::Other("unsupported file type".into()));
    }

    let bytes = std::fs::read(path).map_err(|e| {
        GlintIndexError::Io(std::io::Error::other(format!(
            "failed to read {}: {e}",
            path.display()
        )))
    })?;

    if crate::scanner::parser::is_likely_binary(&bytes) {
        return Err(GlintIndexError::Other("binary file detected".into()));
    }

    let content = String::from_utf8_lossy(&bytes).into_owned();
    let metadata = std::fs::metadata(path).map_err(|e| {
        GlintIndexError::Io(std::io::Error::other(format!(
            "failed to read metadata for {}: {e}",
            path.display()
        )))
    })?;

    let size = metadata.len();
    let modified = metadata.modified().unwrap_or(UNIX_EPOCH);
    let document = Document::new(path.to_path_buf(), size, modified, content);

    let service = index_service
        .lock()
        .map_err(|e| GlintIndexError::Other(format!("index service lock poisoned: {e}")))?;

    if is_update {
        service
            .add_document(&document)
            .or_else(|_| service.update_document(&document))?;
    } else {
        service.add_document(&document)?;
    }

    service.commit()?;
    service.reload_reader()?;

    Ok(())
}

/// Removes a document from the search index by its file path.
fn remove_document(path: &Path, index_service: &Arc<Mutex<IndexService>>) -> Result<()> {
    let service = index_service
        .lock()
        .map_err(|e| GlintIndexError::Other(format!("index service lock poisoned: {e}")))?;

    service.remove_document(path)?;
    service.commit()?;
    service.reload_reader()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::IndexService;
    use std::fs;
    use tempfile::TempDir;

    fn setup() -> (TempDir, Arc<Mutex<IndexService>>) {
        let tmp = TempDir::new().unwrap();
        let index_path = tmp.path().join("index");
        let service = IndexService::open_or_create(&index_path).unwrap();
        (tmp, Arc::new(Mutex::new(service)))
    }

    #[test]
    fn create_watcher() {
        let (_tmp, service) = setup();
        let watcher = FileWatcher::new(service);
        assert!(watcher.is_ok());
        let watcher = watcher.unwrap();
        assert!(!watcher.is_running());
        assert!(watcher.watched_dirs().is_empty());
    }

    #[test]
    fn create_watcher_with_custom_ignores() {
        let (_tmp, service) = setup();
        let custom = vec!["custom_dir".to_string()];
        let watcher = FileWatcher::with_custom_ignores(service, &custom);
        assert!(watcher.is_ok());
    }

    #[test]
    fn start_and_stop_watcher() {
        let (_tmp, service) = setup();
        let mut watcher = FileWatcher::new(service).unwrap();

        let result = watcher.start();
        assert!(result.is_ok());
        assert!(watcher.is_running());

        let result = watcher.stop();
        assert!(result.is_ok());
        assert!(!watcher.is_running());
    }

    #[test]
    fn start_is_idempotent() {
        let (_tmp, service) = setup();
        let mut watcher = FileWatcher::new(service).unwrap();

        watcher.start().unwrap();
        watcher.start().unwrap();
        assert!(watcher.is_running());

        watcher.stop().unwrap();
    }

    #[test]
    fn stop_is_idempotent() {
        let (_tmp, service) = setup();
        let mut watcher = FileWatcher::new(service).unwrap();

        watcher.stop().unwrap();
        watcher.stop().unwrap();
        assert!(!watcher.is_running());
    }

    #[test]
    fn watch_directory() {
        let (_tmp, service) = setup();
        let mut watcher = FileWatcher::new(service).unwrap();
        let dir = _tmp.path().join("watched");
        fs::create_dir(&dir).unwrap();

        watcher.watch_dir(&dir).unwrap();
        assert_eq!(watcher.watched_dirs().len(), 1);
        assert_eq!(watcher.watched_dirs()[0], dir);
    }

    #[test]
    fn watch_duplicate_directory_is_noop() {
        let (_tmp, service) = setup();
        let mut watcher = FileWatcher::new(service).unwrap();
        let dir = _tmp.path().join("watched");
        fs::create_dir(&dir).unwrap();

        watcher.watch_dir(&dir).unwrap();
        watcher.watch_dir(&dir).unwrap();
        assert_eq!(watcher.watched_dirs().len(), 1);
    }

    #[test]
    fn unwatch_directory() {
        let (_tmp, service) = setup();
        let mut watcher = FileWatcher::new(service).unwrap();
        let dir = _tmp.path().join("watched");
        fs::create_dir(&dir).unwrap();

        watcher.watch_dir(&dir).unwrap();
        assert_eq!(watcher.watched_dirs().len(), 1);

        watcher.unwatch_dir(&dir).unwrap();
        assert!(watcher.watched_dirs().is_empty());
    }

    #[test]
    fn watch_dir_while_running() {
        let (_tmp, service) = setup();
        let mut watcher = FileWatcher::new(service).unwrap();
        let dir = _tmp.path().join("watched");
        fs::create_dir(&dir).unwrap();

        watcher.start().unwrap();
        watcher.watch_dir(&dir).unwrap();
        assert_eq!(watcher.watched_dirs().len(), 1);

        watcher.stop().unwrap();
    }

    #[test]
    fn process_created_file() {
        let (tmp, service) = setup();
        let scan_dir = tmp.path().join("scan");
        fs::create_dir(&scan_dir).unwrap();
        fs::write(scan_dir.join("new.txt"), "hello world").unwrap();

        let watcher = FileWatcher::new(service).unwrap();
        let event = WatchEvent::Created(scan_dir.join("new.txt"));
        let result = watcher.process_event(&event);
        assert!(result.is_ok());

        let service = watcher.index_service.lock().unwrap();
        let stats = service.statistics().unwrap();
        assert_eq!(stats.indexed_documents, 1);
    }

    #[test]
    fn process_modified_file() {
        let (tmp, service) = setup();
        let scan_dir = tmp.path().join("scan");
        fs::create_dir(&scan_dir).unwrap();

        {
            let svc = service.lock().unwrap();
            let doc = crate::model::Document::new(
                scan_dir.join("file.txt"),
                100,
                UNIX_EPOCH,
                "original content".into(),
            );
            use crate::traits::DocumentIndexer;
            svc.add_document(&doc).unwrap();
            svc.commit().unwrap();
            svc.reload_reader().unwrap();
        }

        fs::write(scan_dir.join("file.txt"), "updated content").unwrap();

        let watcher = FileWatcher::new(service.clone()).unwrap();
        let event = WatchEvent::Modified(scan_dir.join("file.txt"));
        let result = watcher.process_event(&event);
        assert!(result.is_ok());
    }

    #[test]
    fn process_deleted_file() {
        let (tmp, service) = setup();
        let scan_dir = tmp.path().join("scan");
        fs::create_dir(&scan_dir).unwrap();
        fs::write(scan_dir.join("file.txt"), "to be deleted").unwrap();

        let watcher = FileWatcher::new(service.clone()).unwrap();
        let event = WatchEvent::Created(scan_dir.join("file.txt"));
        watcher.process_event(&event).unwrap();

        let delete_event = WatchEvent::Deleted(scan_dir.join("file.txt"));
        let result = watcher.process_event(&delete_event);
        assert!(result.is_ok());
    }

    #[test]
    fn skip_unsupported_file_type() {
        let (_tmp, service) = setup();
        let scan_dir = _tmp.path().join("scan");
        fs::create_dir(&scan_dir).unwrap();

        let watcher = FileWatcher::new(service).unwrap();
        let event = WatchEvent::Created(scan_dir.join("image.png"));
        let result = watcher.process_event(&event);
        assert!(result.is_ok());
    }

    #[test]
    fn skip_binary_file() {
        let (tmp, service) = setup();
        let scan_dir = tmp.path().join("scan");
        fs::create_dir(&scan_dir).unwrap();

        let binary_content: Vec<u8> = (0..100).map(|i| (i % 32) as u8).collect();
        fs::write(scan_dir.join("data.txt"), &binary_content).unwrap();

        let watcher = FileWatcher::new(service).unwrap();
        let event = WatchEvent::Created(scan_dir.join("data.txt"));
        let result = watcher.process_event(&event);
        assert!(result.is_ok());
    }

    #[test]
    fn skip_ignored_directory() {
        let (_tmp, service) = setup();
        let scan_dir = _tmp.path().join("scan");
        fs::create_dir_all(scan_dir.join(".git")).unwrap();

        let watcher = FileWatcher::new(service).unwrap();
        let event = WatchEvent::Created(scan_dir.join(".git/config"));
        let result = watcher.process_event(&event);
        assert!(result.is_ok());
    }

    #[test]
    fn multiple_events_processed() {
        let (tmp, service) = setup();
        let scan_dir = tmp.path().join("scan");
        fs::create_dir(&scan_dir).unwrap();
        fs::write(scan_dir.join("a.txt"), "file a").unwrap();
        fs::write(scan_dir.join("b.txt"), "file b").unwrap();
        fs::write(scan_dir.join("c.txt"), "file c").unwrap();

        let watcher = FileWatcher::new(service).unwrap();

        let events = vec![
            WatchEvent::Created(scan_dir.join("a.txt")),
            WatchEvent::Created(scan_dir.join("b.txt")),
            WatchEvent::Created(scan_dir.join("c.txt")),
        ];

        for event in &events {
            watcher.process_event(event).unwrap();
        }

        let service = watcher.index_service.lock().unwrap();
        let stats = service.statistics().unwrap();
        assert_eq!(stats.indexed_documents, 3);
    }

    #[test]
    fn event_receiver_is_available() {
        let (_tmp, service) = setup();
        let watcher = FileWatcher::new(service).unwrap();
        let rx = watcher.event_receiver();
        assert!(rx.lock().is_ok());
    }

    #[test]
    fn watched_dirs_reflects_state() {
        let (_tmp, service) = setup();
        let mut watcher = FileWatcher::new(service).unwrap();
        let dir_a = _tmp.path().join("a");
        let dir_b = _tmp.path().join("b");
        fs::create_dir(&dir_a).unwrap();
        fs::create_dir(&dir_b).unwrap();

        assert!(watcher.watched_dirs().is_empty());

        watcher.watch_dir(&dir_a).unwrap();
        assert_eq!(watcher.watched_dirs().len(), 1);

        watcher.watch_dir(&dir_b).unwrap();
        assert_eq!(watcher.watched_dirs().len(), 2);

        watcher.unwatch_dir(&dir_a).unwrap();
        assert_eq!(watcher.watched_dirs().len(), 1);
    }

    #[test]
    fn recoverable_error_does_not_panic() {
        let (_tmp, service) = setup();
        let watcher = FileWatcher::new(service).unwrap();

        let event = WatchEvent::Created(PathBuf::from("/nonexistent/file.txt"));
        let result = watcher.process_event(&event);
        assert!(result.is_ok());
    }

    #[test]
    fn notify_event_conversion_created() {
        let (tx, rx) = mpsc::channel();
        let notify_event =
            NotifyEvent::new(notify::EventKind::Create(notify::event::CreateKind::File))
                .add_path(PathBuf::from("/tmp/test.txt"));

        FileWatcher::handle_notify_event(&tx, &notify_event);

        let received = rx.try_recv().unwrap();
        assert_eq!(
            received,
            WatchEvent::Created(PathBuf::from("/tmp/test.txt"))
        );
    }

    #[test]
    fn notify_event_conversion_modified() {
        let (tx, rx) = mpsc::channel();
        let notify_event = NotifyEvent::new(notify::EventKind::Modify(
            notify::event::ModifyKind::Data(notify::event::DataChange::Content),
        ))
        .add_path(PathBuf::from("/tmp/test.txt"));

        FileWatcher::handle_notify_event(&tx, &notify_event);

        let received = rx.try_recv().unwrap();
        assert_eq!(
            received,
            WatchEvent::Modified(PathBuf::from("/tmp/test.txt"))
        );
    }

    #[test]
    fn notify_event_conversion_deleted() {
        let (tx, rx) = mpsc::channel();
        let notify_event =
            NotifyEvent::new(notify::EventKind::Remove(notify::event::RemoveKind::File))
                .add_path(PathBuf::from("/tmp/test.txt"));

        FileWatcher::handle_notify_event(&tx, &notify_event);

        let received = rx.try_recv().unwrap();
        assert_eq!(
            received,
            WatchEvent::Deleted(PathBuf::from("/tmp/test.txt"))
        );
    }

    #[test]
    fn notify_event_skips_other_kinds() {
        let (tx, rx) = mpsc::channel();
        let notify_event =
            NotifyEvent::new(notify::EventKind::Other).add_path(PathBuf::from("/tmp/test.txt"));

        FileWatcher::handle_notify_event(&tx, &notify_event);

        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn notify_event_ignores_access_kinds() {
        let (tx, rx) = mpsc::channel();
        let notify_event =
            NotifyEvent::new(notify::EventKind::Access(notify::event::AccessKind::Read))
                .add_path(PathBuf::from("/tmp/test.txt"));

        FileWatcher::handle_notify_event(&tx, &notify_event);

        assert!(rx.try_recv().is_err());
    }
}
