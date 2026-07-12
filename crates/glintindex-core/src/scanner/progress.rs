//! Progress reporting for scanning and indexing operations.
//!
//! Provides a [`ProgressReporter`] trait that the scanner calls during
//! file processing, allowing frontends (CLI, GUI) to display real-time
//! progress without duplicating scan logic.

use std::path::Path;

/// A callback interface for reporting scanning progress.
///
/// The scanner calls these methods as files are discovered and processed.
/// Frontends implement this trait to display progress (e.g., a progress
/// bar in the CLI, a native widget in the GUI).
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` because the reporter may be
/// called from background threads.
///
/// # Examples
///
/// ```
/// use glintindex_core::scanner::ProgressReporter;
/// use std::path::Path;
///
/// struct ConsoleReporter;
///
/// impl ProgressReporter for ConsoleReporter {
///     fn on_file_discovered(&self, path: &Path) {
///         println!("Found: {}", path.display());
///     }
///
///     fn on_file_indexed(&self, _path: &Path) {}
///     fn on_file_skipped(&self, _path: &Path) {}
///     fn on_file_failed(&self, _path: &Path, _reason: &str) {}
///     fn on_parser_error(&self, _path: &Path, _parser: &str, _reason: &str) {}
///     fn on_parser_panic(&self, _path: &Path, _parser: &str) {}
///     fn set_total_files(&self, _total: u64) {}
///     fn on_operation_started(&self, _operation: &str) {}
///     fn on_operation_completed(&self) {}
/// }
/// ```
pub trait ProgressReporter: Send + Sync {
    /// Called when a file is discovered during directory traversal.
    fn on_file_discovered(&self, path: &Path);

    /// Called after a file is successfully indexed.
    fn on_file_indexed(&self, path: &Path);

    /// Called when a file is skipped (unsupported type, binary, etc.).
    fn on_file_skipped(&self, path: &Path);

    /// Called when a file fails to process (I/O error, etc.).
    fn on_file_failed(&self, path: &Path, reason: &str);

    /// Called when a parser returns an error for a file.
    fn on_parser_error(&self, path: &Path, parser: &str, reason: &str);

    /// Called when a parser panics while processing a file.
    fn on_parser_panic(&self, path: &Path, parser: &str);

    /// Sets the total number of files to process (if known ahead of time).
    fn set_total_files(&self, total: u64);

    /// Called when an indexing operation starts.
    fn on_operation_started(&self, operation: &str);

    /// Called when an indexing operation completes.
    fn on_operation_completed(&self);
}

/// A no-op reporter that discards all progress information.
///
/// Used when no progress reporting is needed (e.g., tests, headless
/// operation).
pub struct NoopReporter;

impl ProgressReporter for NoopReporter {
    fn on_file_discovered(&self, _path: &Path) {}
    fn on_file_indexed(&self, _path: &Path) {}
    fn on_file_skipped(&self, _path: &Path) {}
    fn on_file_failed(&self, _path: &Path, _reason: &str) {}
    fn on_parser_error(&self, _path: &Path, _parser: &str, _reason: &str) {}
    fn on_parser_panic(&self, _path: &Path, _parser: &str) {}
    fn set_total_files(&self, _total: u64) {}
    fn on_operation_started(&self, _operation: &str) {}
    fn on_operation_completed(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct TestReporter {
        files_discovered: AtomicU64,
        files_indexed: AtomicU64,
        files_skipped: AtomicU64,
        files_failed: AtomicU64,
        parser_errors: AtomicU64,
        parser_panics: AtomicU64,
    }

    impl TestReporter {
        fn new() -> Self {
            Self {
                files_discovered: AtomicU64::new(0),
                files_indexed: AtomicU64::new(0),
                files_skipped: AtomicU64::new(0),
                files_failed: AtomicU64::new(0),
                parser_errors: AtomicU64::new(0),
                parser_panics: AtomicU64::new(0),
            }
        }
    }

    impl ProgressReporter for TestReporter {
        fn on_file_discovered(&self, _path: &Path) {
            self.files_discovered.fetch_add(1, Ordering::Relaxed);
        }
        fn on_file_indexed(&self, _path: &Path) {
            self.files_indexed.fetch_add(1, Ordering::Relaxed);
        }
        fn on_file_skipped(&self, _path: &Path) {
            self.files_skipped.fetch_add(1, Ordering::Relaxed);
        }
        fn on_file_failed(&self, _path: &Path, _reason: &str) {
            self.files_failed.fetch_add(1, Ordering::Relaxed);
        }
        fn on_parser_error(&self, _path: &Path, _parser: &str, _reason: &str) {
            self.parser_errors.fetch_add(1, Ordering::Relaxed);
        }
        fn on_parser_panic(&self, _path: &Path, _parser: &str) {
            self.parser_panics.fetch_add(1, Ordering::Relaxed);
        }
        fn set_total_files(&self, _total: u64) {}
        fn on_operation_started(&self, _operation: &str) {}
        fn on_operation_completed(&self) {}
    }

    #[test]
    fn test_reporter_counts() {
        let reporter = TestReporter::new();
        let path = Path::new("/tmp/test.txt");

        reporter.on_file_discovered(path);
        reporter.on_file_indexed(path);
        reporter.on_file_discovered(path);
        reporter.on_file_skipped(path);
        reporter.on_file_failed(path, "error");
        reporter.on_parser_error(path, "PDF", "corrupted");
        reporter.on_parser_panic(path, "DOCX");

        assert_eq!(reporter.files_discovered.load(Ordering::Relaxed), 2);
        assert_eq!(reporter.files_indexed.load(Ordering::Relaxed), 1);
        assert_eq!(reporter.files_skipped.load(Ordering::Relaxed), 1);
        assert_eq!(reporter.files_failed.load(Ordering::Relaxed), 1);
        assert_eq!(reporter.parser_errors.load(Ordering::Relaxed), 1);
        assert_eq!(reporter.parser_panics.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn noop_reporter_compiles() {
        let reporter = NoopReporter;
        let path = Path::new("/tmp/test.txt");
        reporter.on_file_discovered(path);
        reporter.on_file_indexed(path);
        reporter.on_file_skipped(path);
        reporter.on_file_failed(path, "error");
        reporter.on_parser_error(path, "PDF", "corrupted");
        reporter.on_parser_panic(path, "DOCX");
        reporter.set_total_files(100);
        reporter.on_operation_started("Indexing");
        reporter.on_operation_completed();
    }

    #[test]
    fn reporter_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<NoopReporter>();
        assert_send_sync::<TestReporter>();
    }
}
