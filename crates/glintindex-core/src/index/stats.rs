//! Public indexing result structure.
//!
//! Provides a clean, user-facing summary of what happened during
//! an indexing operation. This is distinct from the internal
//! [`ScannerStatistics`](crate::scanner::ScannerStatistics) type.

use crate::scanner::ScannerStatistics;

/// Summary of an indexing operation, presented to users.
///
/// This struct is the public-facing representation of indexing
/// outcomes. It is constructed from the internal `ScannerStatistics`
/// and provides a simplified view focused on user-relevant metrics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexingStats {
    /// Number of folders processed during indexing.
    pub folders_processed: usize,
    /// Number of newly indexed files (no prior metadata).
    pub files_indexed: u64,
    /// Number of files re-indexed because content changed.
    pub files_reindexed: u64,
    /// Number of files skipped because unchanged.
    pub files_skipped: u64,
    /// Number of parser errors (corrupted files, etc.).
    pub parser_errors: u64,
}

impl IndexingStats {
    /// Creates an `IndexingStats` from scanner statistics.
    ///
    /// The `folders_processed` count must be provided separately
    /// because the scanner does not track folder counts.
    pub fn from_scanner_stats(stats: &ScannerStatistics, folders_processed: usize) -> Self {
        Self {
            folders_processed,
            files_indexed: stats.files_indexed,
            files_reindexed: stats.files_reindexed,
            files_skipped: stats.files_unchanged,
            parser_errors: stats.parser_errors,
        }
    }
}

impl std::fmt::Display for IndexingStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Indexing completed\n")?;
        writeln!(f, "Folders:            {}", self.folders_processed)?;
        writeln!(f, "Files indexed:      {}", self.files_indexed)?;
        writeln!(f, "Files re-indexed:   {}", self.files_reindexed)?;
        writeln!(f, "Files skipped:      {}", self.files_skipped)?;
        writeln!(f, "Parser errors:      {}", self.parser_errors)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_scanner_stats() {
        let mut stats = ScannerStatistics::new();
        stats.files_indexed = 10;
        stats.files_reindexed = 5;
        stats.files_unchanged = 3;
        stats.parser_errors = 2;

        let result = IndexingStats::from_scanner_stats(&stats, 2);
        assert_eq!(result.folders_processed, 2);
        assert_eq!(result.files_indexed, 10);
        assert_eq!(result.files_reindexed, 5);
        assert_eq!(result.files_skipped, 3);
        assert_eq!(result.parser_errors, 2);
    }

    #[test]
    fn display_format() {
        let stats = IndexingStats {
            folders_processed: 1,
            files_indexed: 116,
            files_reindexed: 8,
            files_skipped: 8,
            parser_errors: 4,
        };
        let output = stats.to_string();
        assert!(output.contains("Folders:            1"));
        assert!(output.contains("Files indexed:      116"));
        assert!(output.contains("Files re-indexed:   8"));
        assert!(output.contains("Files skipped:      8"));
        assert!(output.contains("Parser errors:      4"));
    }
}
