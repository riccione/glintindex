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
    /// Number of files skipped due to parser errors (corrupted, invalid format, etc.).
    pub parser_errors: u64,
    /// Number of files skipped due to parser panics (caught via catch_unwind).
    pub parser_panics: u64,
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
            parser_errors: 0,
            parser_panics: 0,
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

    /// Increments the parser errors counter.
    pub fn inc_parser_errors(&mut self) {
        self.parser_errors += 1;
    }

    /// Increments the parser panics counter.
    pub fn inc_parser_panics(&mut self) {
        self.parser_panics += 1;
    }

    /// Merges another `ScannerStatistics` into this one by adding all counters.
    pub fn merge(&mut self, other: &ScannerStatistics) {
        self.directories_scanned += other.directories_scanned;
        self.files_discovered += other.files_discovered;
        self.files_indexed += other.files_indexed;
        self.files_skipped += other.files_skipped;
        self.files_failed += other.files_failed;
        self.parser_errors += other.parser_errors;
        self.parser_panics += other.parser_panics;
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
        assert_eq!(stats.parser_errors, 0);
        assert_eq!(stats.parser_panics, 0);
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
        stats.inc_parser_errors();
        stats.inc_parser_errors();
        stats.inc_parser_errors();
        stats.inc_parser_panics();

        assert_eq!(stats.directories_scanned, 2);
        assert_eq!(stats.files_discovered, 1);
        assert_eq!(stats.files_indexed, 1);
        assert_eq!(stats.files_skipped, 1);
        assert_eq!(stats.files_failed, 2);
        assert_eq!(stats.parser_errors, 3);
        assert_eq!(stats.parser_panics, 1);
    }

    #[test]
    fn merge_statistics() {
        let mut a = ScannerStatistics::new();
        a.directories_scanned = 5;
        a.files_discovered = 20;
        a.files_indexed = 15;
        a.files_skipped = 3;
        a.files_failed = 2;
        a.parser_errors = 1;
        a.parser_panics = 1;

        let mut b = ScannerStatistics::new();
        b.directories_scanned = 3;
        b.files_discovered = 10;
        b.files_indexed = 8;
        b.files_skipped = 1;
        b.files_failed = 1;
        b.parser_errors = 2;
        b.parser_panics = 0;

        a.merge(&b);

        assert_eq!(a.directories_scanned, 8);
        assert_eq!(a.files_discovered, 30);
        assert_eq!(a.files_indexed, 23);
        assert_eq!(a.files_skipped, 4);
        assert_eq!(a.files_failed, 3);
        assert_eq!(a.parser_errors, 3);
        assert_eq!(a.parser_panics, 1);
    }
}
