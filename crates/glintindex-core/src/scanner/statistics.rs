/// Statistics collected during a filesystem scan.
///
/// Provides an overview of what the scanner found and processed.
/// Designed for extensibility — additional metrics can be added
/// without breaking the public API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannerStatistics {
    /// Number of directories traversed during the scan.
    pub directories_scanned: u64,
    /// Total number of files discovered (regardless of type).
    pub files_discovered: u64,
    /// Number of files successfully indexed.
    pub files_indexed: u64,
    /// Number of files skipped (unsupported extension, binary, ignored).
    pub files_skipped: u64,
    /// Number of files that failed to parse or read.
    pub files_failed: u64,
}

impl ScannerStatistics {
    /// Creates a new `ScannerStatistics` with all counters at zero.
    pub fn new() -> Self {
        Self {
            directories_scanned: 0,
            files_discovered: 0,
            files_indexed: 0,
            files_skipped: 0,
            files_failed: 0,
        }
    }

    /// Increments the directories scanned counter.
    pub fn inc_directories_scanned(&mut self) {
        self.directories_scanned += 1;
    }

    /// Increments the files discovered counter.
    pub fn inc_files_discovered(&mut self) {
        self.files_discovered += 1;
    }

    /// Increments the files indexed counter.
    pub fn inc_files_indexed(&mut self) {
        self.files_indexed += 1;
    }

    /// Increments the files skipped counter.
    pub fn inc_files_skipped(&mut self) {
        self.files_skipped += 1;
    }

    /// Increments the files failed counter.
    pub fn inc_files_failed(&mut self) {
        self.files_failed += 1;
    }
}

impl Default for ScannerStatistics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_statistics_are_zero() {
        let stats = ScannerStatistics::new();
        assert_eq!(stats.directories_scanned, 0);
        assert_eq!(stats.files_discovered, 0);
        assert_eq!(stats.files_indexed, 0);
        assert_eq!(stats.files_skipped, 0);
        assert_eq!(stats.files_failed, 0);
    }

    #[test]
    fn increment_counters() {
        let mut stats = ScannerStatistics::new();
        stats.inc_directories_scanned();
        stats.inc_directories_scanned();
        stats.inc_files_discovered();
        stats.inc_files_indexed();
        stats.inc_files_skipped();
        stats.inc_files_failed();
        stats.inc_files_failed();

        assert_eq!(stats.directories_scanned, 2);
        assert_eq!(stats.files_discovered, 1);
        assert_eq!(stats.files_indexed, 1);
        assert_eq!(stats.files_skipped, 1);
        assert_eq!(stats.files_failed, 2);
    }
}
