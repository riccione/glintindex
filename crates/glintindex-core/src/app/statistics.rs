/// Application-level statistics combining index and folder information.
///
/// Provides a high-level view of the application state without exposing
/// internal subsystem details. Designed so future fields can be added
/// without breaking the public API.
///
/// # Examples
///
/// ```
/// use glintindex_core::app::ApplicationStatistics;
///
/// let stats = ApplicationStatistics::new(42, 3);
/// assert_eq!(stats.indexed_documents, 42);
/// assert_eq!(stats.indexed_folders, 3);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationStatistics {
    /// Total number of documents currently in the search index.
    pub indexed_documents: u64,
    /// Number of folders configured for indexing.
    pub indexed_folders: u64,
    /// Result of the most recent indexing operation, if any.
    pub last_indexing_result: Option<IndexingResult>,
}

/// Summary of a single indexing operation.
///
/// Captures the outcome of scanning one or more folders, including
/// counts of discovered, indexed, skipped, and failed files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexingResult {
    /// Number of directories traversed during the scan.
    pub directories_scanned: u64,
    /// Total number of files encountered.
    pub files_discovered: u64,
    /// Number of files successfully indexed.
    pub files_indexed: u64,
    /// Number of files skipped (unsupported type, ignored, etc.).
    pub files_skipped: u64,
    /// Number of files that failed to index.
    pub files_failed: u64,
    /// Number of files skipped due to parser errors (corrupted, invalid format, etc.).
    pub parser_errors: u64,
    /// Number of files skipped due to parser panics (caught via catch_unwind).
    pub parser_panics: u64,
}

impl ApplicationStatistics {
    /// Creates new application statistics with the given counts.
    pub fn new(indexed_documents: u64, indexed_folders: u64) -> Self {
        Self {
            indexed_documents,
            indexed_folders,
            last_indexing_result: None,
        }
    }

    /// Sets the result of the most recent indexing operation.
    pub fn with_last_indexing_result(mut self, result: IndexingResult) -> Self {
        self.last_indexing_result = Some(result);
        self
    }
}

impl IndexingResult {
    /// Creates a new indexing result from scanner statistics.
    pub fn new(
        directories_scanned: u64,
        files_discovered: u64,
        files_indexed: u64,
        files_skipped: u64,
        files_failed: u64,
        parser_errors: u64,
        parser_panics: u64,
    ) -> Self {
        Self {
            directories_scanned,
            files_discovered,
            files_indexed,
            files_skipped,
            files_failed,
            parser_errors,
            parser_panics,
        }
    }
}

impl From<crate::scanner::ScannerStatistics> for IndexingResult {
    fn from(stats: crate::scanner::ScannerStatistics) -> Self {
        Self {
            directories_scanned: stats.directories_scanned,
            files_discovered: stats.files_discovered,
            files_indexed: stats.files_indexed,
            files_skipped: stats.files_skipped,
            files_failed: stats.files_failed,
            parser_errors: stats.parser_errors,
            parser_panics: stats.parser_panics,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_statistics_are_zero() {
        let stats = ApplicationStatistics::new(0, 0);
        assert_eq!(stats.indexed_documents, 0);
        assert_eq!(stats.indexed_folders, 0);
        assert!(stats.last_indexing_result.is_none());
    }

    #[test]
    fn statistics_with_values() {
        let stats = ApplicationStatistics::new(42, 3);
        assert_eq!(stats.indexed_documents, 42);
        assert_eq!(stats.indexed_folders, 3);
    }

    #[test]
    fn statistics_with_last_indexing_result() {
        let result = IndexingResult::new(10, 50, 40, 5, 5, 2, 1);
        let stats = ApplicationStatistics::new(40, 2).with_last_indexing_result(result);
        let last = stats.last_indexing_result.unwrap();
        assert_eq!(last.files_indexed, 40);
        assert_eq!(last.files_skipped, 5);
        assert_eq!(last.files_failed, 5);
        assert_eq!(last.parser_errors, 2);
        assert_eq!(last.parser_panics, 1);
    }

    #[test]
    fn indexing_result_from_scanner_statistics() {
        let mut scanner_stats = crate::scanner::ScannerStatistics::new();
        scanner_stats.directories_scanned = 5;
        scanner_stats.files_discovered = 20;
        scanner_stats.files_indexed = 15;
        scanner_stats.files_skipped = 3;
        scanner_stats.files_failed = 2;
        scanner_stats.parser_errors = 1;
        scanner_stats.parser_panics = 1;

        let result = IndexingResult::from(scanner_stats);
        assert_eq!(result.directories_scanned, 5);
        assert_eq!(result.files_discovered, 20);
        assert_eq!(result.files_indexed, 15);
        assert_eq!(result.files_skipped, 3);
        assert_eq!(result.files_failed, 2);
        assert_eq!(result.parser_errors, 1);
        assert_eq!(result.parser_panics, 1);
    }

    #[test]
    fn indexing_result_new() {
        let result = IndexingResult::new(1, 2, 3, 4, 5, 6, 7);
        assert_eq!(result.directories_scanned, 1);
        assert_eq!(result.files_discovered, 2);
        assert_eq!(result.files_indexed, 3);
        assert_eq!(result.files_skipped, 4);
        assert_eq!(result.files_failed, 5);
        assert_eq!(result.parser_errors, 6);
        assert_eq!(result.parser_panics, 7);
    }

    #[test]
    fn statistics_clone() {
        let stats = ApplicationStatistics::new(10, 2);
        let cloned = stats.clone();
        assert_eq!(stats, cloned);
    }
}
