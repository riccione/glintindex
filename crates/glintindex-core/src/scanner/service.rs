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
    commit_interval: usize,
    max_file_size_bytes: u64,
    parser_timeout_secs: u64,
}

impl<'a> FilesystemScanner<'a> {
    /// Creates a new scanner with default ignore rules and no progress reporting.
    pub fn new(index_service: &'a IndexService) -> Self {
        Self {
            index_service,
            ignore_rules: IgnoreRules::new(),
            parser_registry: ParserRegistry::new(),
            reporter: &NoopReporter,
            commit_interval: 0,
            max_file_size_bytes: 50 * 1024 * 1024,
            parser_timeout_secs: 10,
        }
    }

    /// Creates a new scanner with custom ignore patterns merged into the defaults.
    pub fn with_custom_ignores(index_service: &'a IndexService, custom: &[String]) -> Self {
        Self {
            index_service,
            ignore_rules: IgnoreRules::with_custom(custom),
            parser_registry: ParserRegistry::new(),
            reporter: &NoopReporter,
            commit_interval: 0,
            max_file_size_bytes: 50 * 1024 * 1024,
            parser_timeout_secs: 10,
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

    /// Sets the commit interval for incremental commits.
    ///
    /// When set to a value greater than 0, the scanner will commit the
    /// index every N successfully indexed documents. Set to 0 to disable
    /// incremental commits (single commit at end of scan).
    pub fn with_commit_interval(mut self, interval: usize) -> Self {
        self.commit_interval = interval;
        self
    }

    /// Sets the maximum file size in bytes.
    ///
    /// Files larger than this are indexed by metadata only (path/filename)
    /// without reading the full content into memory.
    pub fn with_max_file_size(mut self, bytes: u64) -> Self {
        self.max_file_size_bytes = bytes;
        self
    }

    /// Sets the parser timeout in seconds.
    ///
    /// If parsing a file exceeds this duration, the file is indexed
    /// metadata-only and processing continues to the next file.
    pub fn with_parser_timeout(mut self, secs: u64) -> Self {
        self.parser_timeout_secs = secs;
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
        let mut docs_since_commit = 0usize;
        let ignore_rules = self.ignore_rules.clone();

        self.reporter
            .on_operation_started("Scanning directories...");

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
                    tracing::debug!(
                        target: "glintindex::scanner",
                        error = %err,
                        "directory walk error"
                    );
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

            // Check metadata to determine if file needs processing
            let path_str = path.to_string_lossy();
            let file_meta = std::fs::metadata(path);
            let current_size = file_meta.as_ref().map(|m| m.len() as i64).unwrap_or(0);
            let current_modified = file_meta
                .as_ref()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            // Query metadata repository to check if file has changed
            let is_new_file = if let Some(repo) = self.index_service.metadata() {
                if let Ok(guard) = repo.lock() {
                    match guard.get(&path_str) {
                        Ok(Some(existing)) => {
                            // Metadata exists — check if file has changed
                            if existing.size == current_size
                                && existing.modified == current_modified
                            {
                                // File unchanged — skip processing
                                stats.inc_files_unchanged();
                                self.reporter.on_file_skipped(path);
                                continue;
                            }
                            // File changed — will be re-indexed
                            false
                        }
                        Ok(None) => true, // No metadata — new file
                        Err(_) => true,   // Query error — process anyway
                    }
                } else {
                    true // Lock poisoned — process anyway
                }
            } else {
                true // No metadata repository — process anyway
            };

            match self.process_file(path) {
                Ok(doc) => {
                    if let Err(err) = self.index_service.update_document(&doc) {
                        let file_size = doc.size;
                        let extension = doc
                            .path
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("unknown")
                            .to_string();

                        tracing::debug!(
                            target: "glintindex::scanner",
                            operation = "index",
                            path = %doc.path.display(),
                            extension = %extension,
                            size = file_size,
                            error = %err,
                            "failed to update document in index"
                        );
                        stats.inc_files_failed();
                        self.reporter.on_file_failed(path, &err.to_string());
                    } else {
                        tracing::debug!(
                            target: "glintindex::scanner",
                            operation = "index",
                            path = %path.display(),
                            metadata_only = doc.is_metadata_only,
                            "file indexed successfully"
                        );
                        if is_new_file {
                            stats.inc_files_indexed();
                        } else {
                            stats.inc_files_reindexed();
                        }
                        self.reporter.on_file_indexed(path);

                        // Track uncommitted documents for final commit
                        docs_since_commit += 1;

                        // Incremental commit when interval is reached
                        if self.commit_interval > 0 && docs_since_commit >= self.commit_interval {
                            self.index_service.flush_metadata_buffer()?;
                            self.index_service.commit()?;
                            docs_since_commit = 0;
                            stats.inc_commits();
                        }
                    }
                }
                Err(err) => {
                    tracing::debug!(
                        target: "glintindex::scanner",
                        operation = "index",
                        path = %path.display(),
                        error = %err,
                        "failed to process file"
                    );
                    stats.inc_files_failed();
                    self.reporter.on_file_failed(path, &err.to_string());
                }
            }
        }

        // Final flush of remaining buffered metadata and commit
        self.index_service.flush_metadata_buffer()?;
        if docs_since_commit > 0 {
            self.index_service.commit()?;
            stats.inc_commits();
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

    fn process_file(&self, path: &Path) -> Result<Document> {
        // Read file metadata first for size check
        let metadata = std::fs::metadata(path)?;
        let file_size = metadata.len();
        let modified = metadata.modified().unwrap_or(UNIX_EPOCH);

        // File size guard — skip text extraction for oversized files
        if file_size > self.max_file_size_bytes {
            tracing::debug!(
                target: "glintindex::scanner",
                path = %path.display(),
                size = file_size,
                max = self.max_file_size_bytes,
                "file exceeds size limit, indexing metadata only"
            );
            return Ok(Document::metadata_only(
                path.to_path_buf(),
                file_size,
                modified,
            ));
        }

        // Read file content
        let bytes = std::fs::read(path)?;

        // Skip binary files for plain text parsing
        let is_binary_format = self.parser_registry.parser_for(path).supported_extensions()
            != crate::parser::PlainTextParser::new().supported_extensions();

        if !is_binary_format && parser::is_likely_binary(&bytes) {
            return Ok(Document::metadata_only(
                path.to_path_buf(),
                file_size,
                modified,
            ));
        }

        let parser = self.parser_registry.parser_for(path);
        let parser_name = parser_type_name(path);

        // Parse with panic guard — panics are caught and treated as metadata-only.
        // Note: true infinite loops cannot be interrupted from the same thread.
        // The timeout_secs config is reserved for future thread-based enforcement.
        let result = catch_unwind(AssertUnwindSafe(|| parser.parse(&bytes, path)));

        match result {
            Ok(Ok(parse_result)) => Ok(Document::new(
                path.to_path_buf(),
                file_size,
                modified,
                parse_result.content,
            )),
            Ok(Err(err)) => {
                tracing::debug!(
                    target: "glintindex::scanner",
                    parser = %parser_name,
                    path = %path.display(),
                    error = %err,
                    "parser error, indexing metadata only"
                );
                Ok(Document::metadata_only(
                    path.to_path_buf(),
                    file_size,
                    modified,
                ))
            }
            Err(_panic) => {
                tracing::warn!(
                    target: "glintindex::scanner",
                    parser = %parser_name,
                    path = %path.display(),
                    "parser panicked, indexing metadata only"
                );
                Ok(Document::metadata_only(
                    path.to_path_buf(),
                    file_size,
                    modified,
                ))
            }
        }
    }
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
        // Binary file is indexed as metadata-only (not skipped)
        assert_eq!(stats.files_indexed, 2);
        assert_eq!(stats.files_failed, 0);
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

        // Both files indexed — broken.pdf as metadata-only
        assert_eq!(stats.files_indexed, 2);
        assert_eq!(stats.parser_errors, 0);
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

        // Both files indexed — broken.docx as metadata-only
        assert_eq!(stats.files_indexed, 2);
        assert_eq!(stats.parser_errors, 0);
    }

    #[test]
    fn corrupted_xlsx_does_not_stop_scan() {
        let (tmp, root) = setup_test_dir();
        fs::write(root.join("broken.xlsx"), b"not an xlsx").unwrap();
        fs::write(root.join("good.txt"), "hello").unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        let stats = scanner.scan_directory(&root).unwrap();

        // Both files indexed — broken.xlsx as metadata-only
        assert_eq!(stats.files_indexed, 2);
        assert_eq!(stats.parser_errors, 0);
    }

    #[test]
    fn corrupted_pptx_does_not_stop_scan() {
        let (tmp, root) = setup_test_dir();
        fs::write(root.join("broken.pptx"), b"not a pptx").unwrap();
        fs::write(root.join("good.txt"), "hello").unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        let stats = scanner.scan_directory(&root).unwrap();

        // Both files indexed — broken.pptx as metadata-only
        assert_eq!(stats.files_indexed, 2);
        assert_eq!(stats.parser_errors, 0);
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

        // Both files indexed — broken.rtf as metadata-only
        assert_eq!(stats.files_discovered, 2);
        assert_eq!(stats.files_indexed, 2);
    }

    #[test]
    fn corrupted_odt_does_not_stop_scan() {
        let (tmp, root) = setup_test_dir();
        fs::write(root.join("broken.odt"), b"not an odt").unwrap();
        fs::write(root.join("good.txt"), "hello").unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        let stats = scanner.scan_directory(&root).unwrap();

        // Both files indexed — broken.odt as metadata-only
        assert_eq!(stats.files_indexed, 2);
        assert_eq!(stats.parser_errors, 0);
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

        // All 7 files indexed — corrupted ones as metadata-only
        assert_eq!(stats.files_indexed, 7);
        assert_eq!(stats.parser_errors, 0);
        assert_eq!(stats.parser_panics, 0);
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

        // All 15 files indexed — corrupted ones as metadata-only
        assert_eq!(stats.files_indexed, 15);
        assert_eq!(stats.parser_errors, 0);
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
        // good.txt indexed normally, broken.pdf and binary.txt as metadata-only
        assert_eq!(stats.files_indexed, 3);
        assert_eq!(stats.parser_errors, 0);
        assert_eq!(stats.files_failed, 0);
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

    // ── Metadata-based skipping tests ────────────────────────────

    #[test]
    fn first_indexing_all_files_are_indexed() {
        let (tmp, root) = setup_test_dir();
        fs::write(root.join("a.txt"), "hello").unwrap();
        fs::write(root.join("b.txt"), "world").unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        let stats = scanner.scan_directory(&root).unwrap();

        assert_eq!(stats.files_indexed, 2);
        assert_eq!(stats.files_reindexed, 0);
        assert_eq!(stats.files_unchanged, 0);
    }

    #[test]
    fn second_indexing_skips_unchanged_files() {
        let (tmp, root) = setup_test_dir();
        fs::write(root.join("a.txt"), "hello").unwrap();
        fs::write(root.join("b.txt"), "world").unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);

        // First scan — all files are new
        let stats1 = scanner.scan_directory(&root).unwrap();
        assert_eq!(stats1.files_indexed, 2);
        assert_eq!(stats1.files_unchanged, 0);

        // Second scan — files should be skipped as unchanged
        let stats2 = scanner.scan_directory(&root).unwrap();
        assert_eq!(stats2.files_indexed, 0);
        assert_eq!(stats2.files_unchanged, 2);
    }

    #[test]
    fn modified_file_is_reindexed() {
        let (tmp, root) = setup_test_dir();
        fs::write(root.join("a.txt"), "hello").unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);

        // First scan
        let stats1 = scanner.scan_directory(&root).unwrap();
        assert_eq!(stats1.files_indexed, 1);

        // Modify the file
        fs::write(root.join("a.txt"), "hello world!").unwrap();

        // Second scan — file should be re-indexed
        let stats2 = scanner.scan_directory(&root).unwrap();
        assert_eq!(stats2.files_reindexed, 1);
        assert_eq!(stats2.files_unchanged, 0);
    }

    #[test]
    fn new_file_is_indexed_on_second_scan() {
        let (tmp, root) = setup_test_dir();
        fs::write(root.join("a.txt"), "hello").unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);

        // First scan
        let stats1 = scanner.scan_directory(&root).unwrap();
        assert_eq!(stats1.files_indexed, 1);

        // Add a new file
        fs::write(root.join("b.txt"), "world").unwrap();

        // Second scan — new file should be indexed, old one skipped
        let stats2 = scanner.scan_directory(&root).unwrap();
        assert_eq!(stats2.files_indexed, 1); // b.txt is new
        assert_eq!(stats2.files_unchanged, 1); // a.txt is unchanged
    }

    #[test]
    fn parser_error_stores_metadata_only() {
        let (tmp, root) = setup_test_dir();
        fs::write(root.join("broken.pdf"), b"not a pdf").unwrap();

        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);

        // First scan — broken.pdf indexed as metadata-only
        let stats1 = scanner.scan_directory(&root).unwrap();
        assert_eq!(stats1.files_indexed, 1);
        assert_eq!(stats1.parser_errors, 0);

        // Second scan — broken.pdf skipped because metadata unchanged
        let stats2 = scanner.scan_directory(&root).unwrap();
        assert_eq!(stats2.files_indexed, 0);
        assert_eq!(stats2.files_unchanged, 1);
    }

    #[test]
    fn commit_interval_1_commits_every_document() {
        let (tmp, root) = setup_test_dir();
        for i in 0..5 {
            fs::write(root.join(format!("file_{i}.txt")), "content").unwrap();
        }
        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service).with_commit_interval(1);
        let stats = scanner.scan_directory(&root).unwrap();
        assert_eq!(stats.files_indexed, 5);
        assert_eq!(stats.commits, 5);
    }

    #[test]
    fn commit_interval_3_commits_in_batches() {
        let (tmp, root) = setup_test_dir();
        for i in 0..10 {
            fs::write(root.join(format!("file_{i}.txt")), "content").unwrap();
        }
        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service).with_commit_interval(3);
        let stats = scanner.scan_directory(&root).unwrap();
        // commits at 3, 6, 9, then final commit for 1 remaining = 4
        assert_eq!(stats.commits, 4);
    }

    #[test]
    fn commit_interval_0_single_commit() {
        let (tmp, root) = setup_test_dir();
        for i in 0..10 {
            fs::write(root.join(format!("file_{i}.txt")), "content").unwrap();
        }
        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service).with_commit_interval(0);
        let stats = scanner.scan_directory(&root).unwrap();
        // interval=0 → no incremental commits, only final commit = 1
        assert_eq!(stats.commits, 1);
    }

    #[test]
    fn commit_interval_exceeds_file_count() {
        let (tmp, root) = setup_test_dir();
        for i in 0..5 {
            fs::write(root.join(format!("file_{i}.txt")), "content").unwrap();
        }
        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service).with_commit_interval(100);
        let stats = scanner.scan_directory(&root).unwrap();
        // 5 files < 100 interval → no incremental commit, only final = 1
        assert_eq!(stats.commits, 1);
    }

    #[test]
    fn commit_interval_exact_boundary() {
        let (tmp, root) = setup_test_dir();
        for i in 0..10 {
            fs::write(root.join(format!("file_{i}.txt")), "content").unwrap();
        }
        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service).with_commit_interval(5);
        let stats = scanner.scan_directory(&root).unwrap();
        // commits at 5, 10 → 2 commits, no final needed (remainder is 0)
        assert_eq!(stats.commits, 2);
    }

    #[test]
    fn unchanged_files_no_commit() {
        let (tmp, root) = setup_test_dir();
        for i in 0..10 {
            fs::write(root.join(format!("file_{i}.txt")), "content").unwrap();
        }
        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service).with_commit_interval(3);

        // First scan — commits happen
        let stats1 = scanner.scan_directory(&root).unwrap();
        assert!(stats1.commits > 0);

        // Second scan — all files unchanged, no indexing, no commits
        let stats2 = scanner.scan_directory(&root).unwrap();
        assert_eq!(stats2.commits, 0);
    }

    #[test]
    fn incremental_commit_makes_documents_searchable() {
        use crate::model::SearchQuery;
        use crate::traits::SearchEngine;

        let (tmp, root) = setup_test_dir();
        let service = create_index_service(&tmp);

        // First batch — 2 files, commit_interval=2 → commit after both
        fs::write(root.join("a.txt"), "apple banana").unwrap();
        fs::write(root.join("b.txt"), "blueberry").unwrap();
        let scanner = FilesystemScanner::new(&service).with_commit_interval(2);
        let stats1 = scanner.scan_directory(&root).unwrap();
        assert_eq!(stats1.commits, 1);
        assert_eq!(stats1.files_indexed, 2);

        // Search for "apple" — should be found after commit
        let response = service.search(&SearchQuery::new("apple")).unwrap();
        assert!(
            !response.results.is_empty(),
            "document 'a.txt' should be searchable after commit"
        );

        // Second batch — 2 more files, commit_interval=2 → commit after both
        fs::write(root.join("c.txt"), "cherry date").unwrap();
        fs::write(root.join("d.txt"), "elderberry").unwrap();
        let scanner2 = FilesystemScanner::new(&service).with_commit_interval(2);
        let stats2 = scanner2.scan_directory(&root).unwrap();
        assert_eq!(stats2.commits, 1);
        assert_eq!(stats2.files_indexed, 2);

        // Search for "cherry" — should be found after second commit
        let response = service.search(&SearchQuery::new("cherry")).unwrap();
        assert!(
            !response.results.is_empty(),
            "document 'c.txt' should be searchable after second commit"
        );
    }

    #[test]
    fn commit_interval_default_is_0() {
        let (tmp, _root) = setup_test_dir();
        let service = create_index_service(&tmp);
        let scanner = FilesystemScanner::new(&service);
        // Default commit_interval is 0 (no incremental commits)
        assert_eq!(scanner.commit_interval, 0);
    }
}
