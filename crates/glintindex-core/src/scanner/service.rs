use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use walkdir::WalkDir;

use crate::error::{GlintIndexError, Result};
use crate::index::IndexService;
use crate::model::Document;
use crate::parser::ParserRegistry;
use crate::parser::trait_impl::DocumentParser;
use crate::traits::DocumentIndexer;

use super::ignore::IgnoreRules;
use super::parser;
use super::statistics::ScannerStatistics;

/// High-level filesystem scanner that discovers, parses, and indexes files.
///
/// `FilesystemScanner` coordinates the entire scan pipeline: walking
/// directories, applying ignore rules, filtering by file type, reading
/// content, and sending results to the [`IndexService`]. It hides all
/// `walkdir` types from the public API.
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
}

impl<'a> FilesystemScanner<'a> {
    /// Creates a new scanner with default ignore rules.
    pub fn new(index_service: &'a IndexService) -> Self {
        Self {
            index_service,
            ignore_rules: IgnoreRules::new(),
            parser_registry: ParserRegistry::new(),
        }
    }

    /// Creates a new scanner with custom ignore patterns merged into the defaults.
    pub fn with_custom_ignores(index_service: &'a IndexService, custom: &[String]) -> Self {
        Self {
            index_service,
            ignore_rules: IgnoreRules::with_custom(custom),
            parser_registry: ParserRegistry::new(),
        }
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

            if !IgnoreRules::is_supported_file(path) {
                stats.inc_files_skipped();
                continue;
            }

            match self.process_file(path) {
                Ok(doc) => {
                    if let Err(err) = self.index_service.add_document(&doc) {
                        tracing::warn!("failed to index {}: {err}", path.display());
                        stats.inc_files_failed();
                    } else {
                        stats.inc_files_indexed();
                    }
                }
                Err(err) => {
                    tracing::warn!("failed to read {}: {err}", path.display());
                    stats.inc_files_failed();
                }
            }
        }

        Ok(stats)
    }

    /// Scans multiple directories and returns combined statistics.
    pub fn scan_directories(&self, directories: &[PathBuf]) -> Result<ScannerStatistics> {
        let mut combined = ScannerStatistics::new();
        for dir in directories {
            let stats = self.scan_directory(dir)?;
            combined.directories_scanned += stats.directories_scanned;
            combined.files_discovered += stats.files_discovered;
            combined.files_indexed += stats.files_indexed;
            combined.files_skipped += stats.files_skipped;
            combined.files_failed += stats.files_failed;
        }
        Ok(combined)
    }

    fn process_file(&self, path: &Path) -> Result<Document> {
        let bytes = std::fs::read(path)?;

        // Skip binary files for plain text parsing
        // Document parsers handle their own binary formats
        let is_binary_format = self.parser_registry.parser_for(path).supported_extensions()
            != crate::parser::PlainTextParser::new().supported_extensions();

        if !is_binary_format && parser::is_likely_binary(&bytes) {
            return Err(GlintIndexError::Other("binary file detected".into()));
        }

        let parser = self.parser_registry.parser_for(path);
        let parse_result = parser.parse(&bytes, path)?;

        let metadata = std::fs::metadata(path)?;
        let size = metadata.len();
        let modified = metadata.modified().unwrap_or(UNIX_EPOCH);

        Ok(Document::new(
            path.to_path_buf(),
            size,
            modified,
            parse_result.content,
        ))
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
}
