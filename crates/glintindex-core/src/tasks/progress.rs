//! Progress reporting for background tasks.
//!
//! Provides a snapshot of the current state of a background operation,
//! including the number of files processed, total files, and any errors
//! encountered during parsing.

use crate::scanner::ScannerStatistics;

/// Progress information for a background indexing or rebuild operation.
///
/// Captures real-time information about the current operation so the GUI
/// can display meaningful status to the user. The progress model is
/// designed to be cheaply cloneable and extendable without breaking
/// the public API.
///
/// # Examples
///
/// ```
/// use glintindex_core::tasks::Progress;
///
/// let progress = Progress::new("Indexing");
/// assert_eq!(progress.status_message, "Indexing");
/// assert_eq!(progress.files_processed, 0);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Progress {
    /// Human-readable status message (e.g., "Indexing", "Rebuilding").
    pub status_message: String,
    /// Number of files processed so far.
    pub files_processed: u64,
    /// Total number of files to process, if known.
    ///
    /// `None` when the total is not yet determined (e.g., during directory
    /// traversal before all files are counted).
    pub total_files: Option<u64>,
    /// Number of files successfully indexed.
    pub files_indexed: u64,
    /// Number of files skipped (unsupported, binary, etc.).
    pub files_skipped: u64,
    /// Number of files that failed to parse.
    pub files_failed: u64,
    /// Number of parser errors (corrupted files, invalid format).
    pub parser_errors: u64,
    /// Number of parser panics caught during extraction.
    pub parser_panics: u64,
    /// The path of the file currently being processed, if any.
    pub current_file: Option<String>,
}

impl Progress {
    /// Creates a new `Progress` with the given status message and all
    /// counters at zero.
    pub fn new(status_message: impl Into<String>) -> Self {
        Self {
            status_message: status_message.into(),
            ..Default::default()
        }
    }

    /// Sets the current file being processed.
    pub fn with_current_file(mut self, path: impl Into<String>) -> Self {
        self.current_file = Some(path.into());
        self
    }

    /// Sets the total number of files to process.
    pub fn with_total_files(mut self, total: u64) -> Self {
        self.total_files = Some(total);
        self
    }

    /// Returns `true` if a total file count is known.
    pub fn has_total(&self) -> bool {
        self.total_files.is_some()
    }

    /// Returns the completion percentage (0.0 to 100.0), or `None` if
    /// the total is not known.
    pub fn percentage(&self) -> Option<f64> {
        self.total_files.map(|total| {
            if total == 0 {
                100.0
            } else {
                (self.files_processed as f64 / total as f64) * 100.0
            }
        })
    }

    /// Creates a `Progress` from a `ScannerStatistics` snapshot.
    pub fn from_statistics(stats: &ScannerStatistics, status_message: impl Into<String>) -> Self {
        Self {
            status_message: status_message.into(),
            files_processed: stats.files_discovered,
            total_files: None,
            files_indexed: stats.files_indexed,
            files_skipped: stats.files_skipped,
            files_failed: stats.files_failed,
            parser_errors: stats.parser_errors,
            parser_panics: stats.parser_panics,
            current_file: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_progress_defaults() {
        let progress = Progress::new("Indexing");
        assert_eq!(progress.status_message, "Indexing");
        assert_eq!(progress.files_processed, 0);
        assert!(progress.total_files.is_none());
        assert_eq!(progress.files_indexed, 0);
        assert_eq!(progress.files_skipped, 0);
        assert_eq!(progress.files_failed, 0);
        assert_eq!(progress.parser_errors, 0);
        assert_eq!(progress.parser_panics, 0);
        assert!(progress.current_file.is_none());
    }

    #[test]
    fn progress_with_current_file() {
        let progress = Progress::new("Indexing").with_current_file("/path/to/file.txt");
        assert_eq!(progress.current_file.as_deref(), Some("/path/to/file.txt"));
    }

    #[test]
    fn progress_with_total_files() {
        let progress = Progress::new("Indexing").with_total_files(100);
        assert_eq!(progress.total_files, Some(100));
        assert!(progress.has_total());
    }

    #[test]
    fn progress_percentage_with_total() {
        let progress = Progress::new("Indexing")
            .with_total_files(200)
            .with_current_file("file.txt");
        // Manually set files_processed for testing
        let mut progress = progress;
        progress.files_processed = 50;
        assert!((progress.percentage().unwrap() - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn progress_percentage_zero_total() {
        let mut progress = Progress::new("Indexing").with_total_files(0);
        progress.files_processed = 0;
        assert!((progress.percentage().unwrap() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn progress_percentage_no_total() {
        let progress = Progress::new("Indexing");
        assert!(progress.percentage().is_none());
    }

    #[test]
    fn progress_from_statistics() {
        let mut stats = ScannerStatistics::new();
        stats.files_discovered = 50;
        stats.files_indexed = 40;
        stats.files_skipped = 5;
        stats.files_failed = 5;
        stats.parser_errors = 3;
        stats.parser_panics = 1;

        let progress = Progress::from_statistics(&stats, "Indexing complete");
        assert_eq!(progress.status_message, "Indexing complete");
        assert_eq!(progress.files_processed, 50);
        assert_eq!(progress.files_indexed, 40);
        assert_eq!(progress.files_skipped, 5);
        assert_eq!(progress.files_failed, 5);
        assert_eq!(progress.parser_errors, 3);
        assert_eq!(progress.parser_panics, 1);
    }

    #[test]
    fn progress_clone() {
        let progress = Progress::new("Indexing")
            .with_total_files(100)
            .with_current_file("test.txt");
        let cloned = progress.clone();
        assert_eq!(progress, cloned);
    }
}
