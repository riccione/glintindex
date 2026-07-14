use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use walkdir::WalkDir;

use crate::error::Result;
use crate::index::IndexService;
use crate::model::Document;
use crate::parser::ParserRegistry;
use crate::parser::trait_impl::DocumentParser;
use crate::traits::DocumentIndexer;

use super::ignore::IgnoreRules;
use super::parser;
use super::progress::{NoopReporter, ProgressReporter};
use super::statistics::ScannerStatistics;

/// High-level filesystem scanner that discovers, parses, and indexes files.
///
/// `FilesystemScanner` coordinates the entire scan pipeline: walking
/// directories, applying ignore rules, filtering by file type, reading
/// content, and sending results to the [`IndexService`]. It hides all
/// `walkdir` types from the public API.
///
/// # Progress Reporting
///
/// The scanner accepts an optional [`ProgressReporter`] via
/// [`with_progress`](Self::with_progress). When provided, the scanner
/// calls the reporter during file processing, allowing frontends to
/// display real-time progress without duplicating scan logic.
///
/// # Examples
///
/// ```no_run
/// use glintindex_core::scanner::FilesystemScanner;
/// use glintindex_core::index::IndexService;
/// use std::path::Path;
///
/// let index_service = IndexService::open_or_create(Path::new("/tmp/index")).unwrap();
/// let scanner = FilesystemScanner::new(&index_service);
/// let stats = scanner.scan_directory(Path::new("/home/user/docs")).unwrap();
/// println!("Indexed {} files", stats.files_indexed);
/// ```
pub struct FilesystemScanner<'a> {
    index_service: &'a IndexService,
    ignore_rules: IgnoreRules,
    parser_registry: ParserRegistry,
    reporter: &'a dyn ProgressReporter,
}

impl<'a> FilesystemScanner<'a> {
    /// Creates a new scanner with default ignore rules and no progress reporting.
    pub fn new(index_service: &'a IndexService) -> Self {
        Self {
            index_service,
            ignore_rules: IgnoreRules::new(),
            parser_registry: ParserRegistry::new(),
            reporter: &NoopReporter,
        }
    }

    /// Creates a new scanner with custom ignore patterns merged into the defaults.
    pub fn with_custom_ignores(index_service: &'a IndexService, custom: &[String]) -> Self {
        Self {
            index_service,
            ignore_rules: IgnoreRules::with_custom(custom),
            parser_registry: ParserRegistry::new(),
            reporter: &NoopReporter,
        }
    }

    /// Sets a progress reporter for scanning operations.
    ///
    /// The reporter is called during file discovery, indexing, and
    /// error handling to provide real-time progress feedback.
    pub fn with_progress(mut self, reporter: &'a dyn ProgressReporter) -> Self {
        self.reporter = reporter;
        self
    }

    /// Scans a single directory recursively and indexes all supported files.
    ///
    /// Returns [`ScannerStatistics`] summarizing what was found and processed.
    /// Errors on individual files are recovered from — one bad file does not
    /// stop the scan.
    ///
    /// # Errors
    ///
    /// Returns an error only if the root directory cannot be read.
    pub fn scan_directory(&self, directory: &Path) -> Result<ScannerStatistics> {
        let mut stats = ScannerStatistics::new();
        let ignore_rules = self.ignore_rules.clone();

        self.reporter
            .on_operation_started("Scanning directories...");

        // Pre-count supported files for accurate progress reporting
        let total = self.count_supported_files(directory);
        self.reporter.set_total_files(total);

        let walker = WalkDir::new(directory)
            .follow_links(true)
            .into_iter()
            .filter_entry(move |entry| {
                if entry.file_type().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        return !ignore_rules.should_ignore_dir(name);
                    }
                }
                true
            });

        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(err) => {
                    tracing::warn!("walkdir error: {err}");
                    continue;
                }
            };

            if entry.file_type().is_dir() {
                stats.inc_directories_scanned();
                continue;
            }

            // File entry
            let path = entry.path();
            stats.inc_files_discovered();
            self.reporter.on_file_discovered(path);

            if !IgnoreRules::is_supported_file(path) {
                stats.inc_files_skipped();
                self.reporter.on_file_skipped(path);
                continue;
            }

            match self.process_file(path) {
                Ok(doc) => {
                    if let Err(err) = self.index_service.update_document(&doc) {
                        tracing::warn!("failed to index {}: {err}", path.display());
                        stats.inc_files_failed();
                        self.reporter.on_file_failed(path, &err.to_string());
                    } else {
                        tracing::info!("Indexed: {}", path.display());
                        stats.inc_files_indexed();
                        self.reporter.on_file_indexed(path);
                    }
                }
                Err(FileParseOutcome::ReadError(err)) => {
                    tracing::warn!(
                        "Skipping unreadable file: {}\n  Reason: {err}",
                        path.display()
                    );
                    stats.inc_files_failed();
                    self.reporter.on_file_failed(path, &err);
                }
                Err(FileParseOutcome::ParserError(parser_name, err)) => {
                    tracing::warn!(
                        "Skipping corrupted {parser_name}: {}\n  Reason: {err}",
                        path.display()
                    );
                    stats.inc_parser_errors();
                    stats.inc_files_skipped();
                    self.reporter.on_parser_error(path, &parser_name, &err);
                }
                Err(FileParseOutcome::ParserPanic(parser_name)) => {
                    tracing::error!(
                        "{parser_name} parser panicked: {}\n  Parser panic recovered.",
                        path.display()
                    );
                    stats.inc_parser_panics();
                    stats.inc_files_skipped();
                    self.reporter.on_parser_panic(path, &parser_name);
                }
            }
        }

        self.reporter.on_operation_completed();
        Ok(stats)
    }

    /// Scans multiple directories and returns combined statistics.
    pub fn scan_directories(&self, directories: &[PathBuf]) -> Result<ScannerStatistics> {
        let mut combined = ScannerStatistics::new();
        for dir in directories {
            let stats = self.scan_directory(dir)?;
            combined.merge(&stats);
        }
        Ok(combined)
    }

    fn process_file(&self, path: &Path) -> std::result::Result<Document, FileParseOutcome> {
        let bytes = std::fs::read(path)
            .map_err(|e| FileParseOutcome::ReadError(format!("I/O error: {e}")))?;

        // Skip binary files for plain text parsing
        // Document parsers handle their own binary formats
        let is_binary_format = self.parser_registry.parser_for(path).supported_extensions()
            != crate::parser::PlainTextParser::new().supported_extensions();

        if !is_binary_format && parser::is_likely_binary(&bytes) {
            return Err(FileParseOutcome::ReadError("binary file detected".into()));
        }

        let parser = self.parser_registry.parser_for(path);
        let parser_name = parser_type_name(path);

        let result = catch_unwind(AssertUnwindSafe(|| parser.parse(&bytes, path)));

        match result {
            Ok(Ok(parse_result)) => {
                let metadata = std::fs::metadata(path).map_err(|e| {
                    FileParseOutcome::ReadError(format!("metadata read error: {e}"))
                })?;
                let size = metadata.len();
                let modified = metadata.modified().unwrap_or(UNIX_EPOCH);

                Ok(Document::new(
                    path.to_path_buf(),
                    size,
                    modified,
                    parse_result.content,
                ))
            }
            Ok(Err(err)) => Err(FileParseOutcome::ParserError(
                parser_name.to_string(),
                format!("{err}"),
            )),
            Err(_panic) => Err(FileParseOutcome::ParserPanic(parser_name.to_string())),
        }
    }

    /// Counts supported files in a directory for progress reporting.
    ///
    /// Performs a lightweight WalkDir traversal counting only files
    /// with supported extensions. Directories and unsupported files
    /// are skipped. This is used to set `total_files` on the progress
    /// reporter before the main processing loop begins.
    fn count_supported_files(&self, directory: &Path) -> u64 {
        let ignore_rules = self.ignore_rules.clone();

        WalkDir::new(directory)
            .follow_links(true)
            .into_iter()
            .filter_entry(move |entry| {
                if entry.file_type().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        return !ignore_rules.should_ignore_dir(name);
                    }
                }
                true
            })
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .filter(|entry| IgnoreRules::is_supported_file(entry.path()))
            .count() as u64
    }
}

/// Outcome of parsing a single file, distinguishing error types for statistics.
#[derive(Debug)]
enum FileParseOutcome {
    /// File could not be read (I/O error, binary detection, etc.).
    ReadError(String),
    /// Parser returned an error (corrupted file, unsupported format, etc.).
    ParserError(String, String),
    /// Parser panicked (caught via catch_unwind).
    ParserPanic(String),
}

/// Returns a human-readable parser name for logging based on file extension.
fn parser_type_name(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "pdf" => "PDF",
        "docx" | "docm" => "DOCX",
        "xlsx" | "xlsm" | "xlsb" | "xls" => "XLSX",
        "pptx" | "pptm" => "PPTX",
        "rtf" => "RTF",
        "odt" => "ODT",
        _ => "text",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::IndexService;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_dir() -> (TempDir, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("scan");
        fs::create_dir(&root).unwrap();
        (tmp, root)
    }

    fn create_index_service(tmp: &TempDir) -> IndexService {
        let index_path = tmp.path().join("index");
        IndexService::open_or_create(&index_path).unwrap()
    }

    #[test]
    fn scan_empty_directory() {
        let (tmp, root) = setup_test_dir();
        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        let stats = scanner.scan_directory(&root).unwrap();
        assert_eq!(stats.files_indexed, 0);
        assert_eq!(stats.files_discovered, 0);
    }

    #[test]
    fn scan_txt_file() {
        let (tmp, root) = setup_test_dir();
        fs::write(root.join("hello.txt"), "hello world").unwrap();
        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        let stats = scanner.scan_directory(&root).unwrap();
        assert_eq!(stats.files_indexed, 1);
        assert_eq!(stats.files_discovered, 1);
    }

    #[test]
    fn scan_nested_directories() {
        let (tmp, root) = setup_test_dir();
        fs::create_dir_all(root.join("sub/nested")).unwrap();
        fs::write(root.join("a.txt"), "file a").unwrap();
        fs::write(root.join("sub/b.txt"), "file b").unwrap();
        fs::write(root.join("sub/nested/c.txt"), "file c").unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        let stats = scanner.scan_directory(&root).unwrap();
        assert_eq!(stats.files_indexed, 3);
        assert!(stats.directories_scanned >= 2);
    }

    #[test]
    fn skip_ignored_directories() {
        let (tmp, root) = setup_test_dir();
        fs::create_dir_all(root.join(".git/objects")).unwrap();
        fs::write(root.join("good.txt"), "content").unwrap();
        fs::write(root.join(".git/objects/abc"), "git object").unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        let stats = scanner.scan_directory(&root).unwrap();
        assert_eq!(stats.files_indexed, 1);
    }

    #[test]
    fn skip_target_directory() {
        let (tmp, root) = setup_test_dir();
        fs::create_dir_all(root.join("target/debug")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(root.join("target/debug/binary"), "binary").unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        let stats = scanner.scan_directory(&root).unwrap();
        assert_eq!(stats.files_indexed, 1);
    }

    #[test]
    fn skip_unsupported_extensions() {
        let (tmp, root) = setup_test_dir();
        fs::write(root.join("image.png"), [0x89, 0x50, 0x4E, 0x47]).unwrap();
        fs::write(root.join("readme.txt"), "hello").unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        let stats = scanner.scan_directory(&root).unwrap();
        assert_eq!(stats.files_indexed, 1);
        assert_eq!(stats.files_skipped, 1);
    }

    #[test]
    fn skip_binary_files() {
        let (tmp, root) = setup_test_dir();
        let binary_content: Vec<u8> = (0..100).map(|i| (i % 32) as u8).collect();
        fs::write(root.join("data.txt"), &binary_content).unwrap();
        fs::write(root.join("text.txt"), "not binary").unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        let stats = scanner.scan_directory(&root).unwrap();
        assert_eq!(stats.files_indexed, 1);
        assert_eq!(stats.files_failed, 1);
    }

    #[test]
    fn custom_ignored_directories() {
        let (tmp, root) = setup_test_dir();
        fs::create_dir_all(root.join("custom_cache")).unwrap();
        fs::write(root.join("custom_cache/data.txt"), "data").unwrap();
        fs::write(root.join("good.txt"), "good").unwrap();

        let service = create_index_service(&tmp);
        let custom = vec!["custom_cache".to_string()];
        let scanner = FilesystemScanner::with_custom_ignores(&service, &custom);
        let stats = scanner.scan_directory(&root).unwrap();
        assert_eq!(stats.files_indexed, 1);
    }

    #[test]
    fn scan_multiple_directories() {
        let (tmp, root) = setup_test_dir();
        let dir_a = root.join("a");
        let dir_b = root.join("b");
        fs::create_dir_all(&dir_a).unwrap();
        fs::create_dir_all(&dir_b).unwrap();
        fs::write(dir_a.join("file1.txt"), "one").unwrap();
        fs::write(dir_b.join("file2.txt"), "two").unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        let stats = scanner.scan_directories(&[dir_a, dir_b]).unwrap();
        assert_eq!(stats.files_indexed, 2);
    }

    #[test]
    fn invalid_utf8_file_is_read_lossy() {
        let (tmp, root) = setup_test_dir();
        fs::write(root.join("bad.txt"), [0xFF, 0xFE, b'h', b'i']).unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        let stats = scanner.scan_directory(&root).unwrap();
        assert_eq!(stats.files_indexed, 1);
    }

    #[test]
    fn unreadable_file_does_not_stop_scan() {
        let (tmp, root) = setup_test_dir();
        fs::write(root.join("good1.txt"), "one").unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/nonexistent/path", root.join("broken.txt")).unwrap();
        }
        fs::write(root.join("good2.txt"), "two").unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        let stats = scanner.scan_directory(&root).unwrap();
        assert!(stats.files_indexed >= 2);
    }

    #[test]
    fn statistics_are_correct() {
        let (tmp, root) = setup_test_dir();
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("a.rs"), "fn main() {}").unwrap();
        fs::write(root.join("sub/b.py"), "print('hello')").unwrap();
        fs::write(root.join("c.png"), [0x89]).unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        let stats = scanner.scan_directory(&root).unwrap();

        assert_eq!(stats.files_discovered, 3);
        assert_eq!(stats.files_indexed, 2);
        assert!(stats.directories_scanned >= 1);
    }

    // --- Fault tolerance tests ---

    #[test]
    fn corrupted_pdf_does_not_stop_scan() {
        let (tmp, root) = setup_test_dir();
        // Not a valid PDF at all
        fs::write(root.join("broken.pdf"), b"not a pdf file").unwrap();
        fs::write(root.join("good.txt"), "hello").unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        let stats = scanner.scan_directory(&root).unwrap();

        assert_eq!(stats.files_indexed, 1);
        assert_eq!(stats.parser_errors, 1);
        assert_eq!(stats.parser_panics, 0);
    }

    #[test]
    fn corrupted_docx_does_not_stop_scan() {
        let (tmp, root) = setup_test_dir();
        fs::write(root.join("broken.docx"), b"not a docx").unwrap();
        fs::write(root.join("good.txt"), "hello").unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        let stats = scanner.scan_directory(&root).unwrap();

        assert_eq!(stats.files_indexed, 1);
        assert_eq!(stats.parser_errors, 1);
    }

    #[test]
    fn corrupted_xlsx_does_not_stop_scan() {
        let (tmp, root) = setup_test_dir();
        fs::write(root.join("broken.xlsx"), b"not an xlsx").unwrap();
        fs::write(root.join("good.txt"), "hello").unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        let stats = scanner.scan_directory(&root).unwrap();

        assert_eq!(stats.files_indexed, 1);
        assert_eq!(stats.parser_errors, 1);
    }

    #[test]
    fn corrupted_pptx_does_not_stop_scan() {
        let (tmp, root) = setup_test_dir();
        fs::write(root.join("broken.pptx"), b"not a pptx").unwrap();
        fs::write(root.join("good.txt"), "hello").unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        let stats = scanner.scan_directory(&root).unwrap();

        assert_eq!(stats.files_indexed, 1);
        assert_eq!(stats.parser_errors, 1);
    }

    #[test]
    fn corrupted_rtf_does_not_stop_scan() {
        let (tmp, root) = setup_test_dir();
        // RTF starts with {\rtf but content is garbage
        fs::write(root.join("broken.rtf"), b"{\\rtf invalid garbage content}").unwrap();
        fs::write(root.join("good.txt"), "hello").unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        let stats = scanner.scan_directory(&root).unwrap();

        // RTF parser may return an error or parse garbage - either way scan continues
        assert_eq!(stats.files_discovered, 2);
        assert!(stats.files_indexed + stats.parser_errors >= 1);
    }

    #[test]
    fn corrupted_odt_does_not_stop_scan() {
        let (tmp, root) = setup_test_dir();
        fs::write(root.join("broken.odt"), b"not an odt").unwrap();
        fs::write(root.join("good.txt"), "hello").unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        let stats = scanner.scan_directory(&root).unwrap();

        assert_eq!(stats.files_indexed, 1);
        assert_eq!(stats.parser_errors, 1);
    }

    #[test]
    fn multiple_failures_in_one_run() {
        let (tmp, root) = setup_test_dir();
        // Mix of valid and corrupted files
        fs::write(root.join("good1.txt"), "hello").unwrap();
        fs::write(root.join("broken.pdf"), b"not a pdf").unwrap();
        fs::write(root.join("broken.docx"), b"not a docx").unwrap();
        fs::write(root.join("broken.xlsx"), b"not an xlsx").unwrap();
        fs::write(root.join("broken.pptx"), b"not a pptx").unwrap();
        fs::write(root.join("broken.odt"), b"not an odt").unwrap();
        fs::write(root.join("good2.txt"), "world").unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        let stats = scanner.scan_directory(&root).unwrap();

        // Good files should still be indexed
        assert_eq!(stats.files_indexed, 2);
        // Corrupted files should be counted as parser errors
        assert_eq!(stats.parser_errors, 5);
        assert_eq!(stats.parser_panics, 0);
        // Total discovered = 7
        assert_eq!(stats.files_discovered, 7);
    }

    #[test]
    fn indexing_continues_after_many_failures() {
        let (tmp, root) = setup_test_dir();
        // Create 10 corrupted PDF files and 5 good text files
        for i in 0..10 {
            fs::write(root.join(format!("bad{i}.pdf")), b"not a pdf").unwrap();
        }
        for i in 0..5 {
            fs::write(root.join(format!("good{i}.txt")), format!("text {i}")).unwrap();
        }

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        let stats = scanner.scan_directory(&root).unwrap();

        assert_eq!(stats.files_indexed, 5);
        assert_eq!(stats.parser_errors, 10);
        assert_eq!(stats.parser_panics, 0);
        assert_eq!(stats.files_discovered, 15);
    }

    #[test]
    fn statistics_updated_correctly_for_mixed_outcomes() {
        let (tmp, root) = setup_test_dir();
        fs::write(root.join("good.txt"), "hello").unwrap();
        fs::write(root.join("broken.pdf"), b"not a pdf").unwrap();
        let binary_content: Vec<u8> = (0..100).map(|i| (i % 32) as u8).collect();
        fs::write(root.join("binary.txt"), &binary_content).unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        let stats = scanner.scan_directory(&root).unwrap();

        assert_eq!(stats.files_discovered, 3);
        assert_eq!(stats.files_indexed, 1);
        assert_eq!(stats.parser_errors, 1);
        assert_eq!(stats.files_failed, 1); // binary detection
        assert_eq!(stats.parser_panics, 0);
    }

    #[test]
    fn parser_name_returns_correct_names() {
        assert_eq!(parser_type_name(Path::new("test.pdf")), "PDF");
        assert_eq!(parser_type_name(Path::new("test.docx")), "DOCX");
        assert_eq!(parser_type_name(Path::new("test.xlsx")), "XLSX");
        assert_eq!(parser_type_name(Path::new("test.pptx")), "PPTX");
        assert_eq!(parser_type_name(Path::new("test.rtf")), "RTF");
        assert_eq!(parser_type_name(Path::new("test.odt")), "ODT");
        assert_eq!(parser_type_name(Path::new("test.txt")), "text");
        assert_eq!(parser_type_name(Path::new("test.rs")), "text");
    }
}
