//! Preview service providing the public API for file previews.
//!
//! Coordinates file loading, syntax highlighting, encoding detection,
//! and search match highlighting. The GUI should communicate only
//! through this service.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::preview::encoding::Encoding;
use crate::preview::highlight::{self, HighlightedMatch};
use crate::preview::loader;
use crate::preview::loader::LoadConfig;
use crate::preview::syntax::{Style, SyntaxHighlighter};

/// Configuration for the preview service.
#[derive(Debug, Clone)]
pub struct PreviewConfig {
    /// Maximum file size for preview in bytes.
    pub max_file_size: u64,
}

impl Default for PreviewConfig {
    fn default() -> Self {
        Self {
            max_file_size: 1024 * 1024, // 1 MB
        }
    }
}

/// Complete preview output for a file.
#[derive(Debug, Clone)]
pub struct PreviewOutput {
    /// The file path being previewed.
    pub path: PathBuf,
    /// The highlighted lines with syntax information.
    pub lines: Vec<PreviewLine>,
    /// Whether the file was truncated.
    pub truncated: bool,
    /// The detected encoding.
    pub encoding: Encoding,
    /// Whether the file is binary.
    pub is_binary: bool,
    /// Error message if loading failed.
    pub error: Option<String>,
    /// The original file size.
    pub original_size: u64,
}

/// A single line in the preview output.
#[derive(Debug, Clone)]
pub struct PreviewLine {
    /// The line number (1-indexed).
    pub line_number: usize,
    /// The plain text content of the line.
    pub text: String,
    /// Syntax-highlighted segments.
    pub syntax_spans: Vec<(usize, usize, Style)>,
    /// Search match highlights (byte offsets).
    pub match_highlights: Vec<HighlightedMatch>,
}

/// High-level preview service that coordinates all preview operations.
///
/// The service is thread-safe and can be shared across components.
/// It handles file loading, syntax detection, encoding detection,
/// and search match highlighting.
///
/// Implements `Clone` cheaply via `Arc` — all internal state is shared.
pub struct PreviewService {
    config: PreviewConfig,
    syntax_highlighter: Arc<Mutex<SyntaxHighlighter>>,
    /// Cache for the last loaded file to avoid redundant reloads.
    cache: Arc<Mutex<Option<(PathBuf, PreviewOutput)>>>,
}

impl Clone for PreviewService {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            syntax_highlighter: Arc::clone(&self.syntax_highlighter),
            cache: Arc::clone(&self.cache),
        }
    }
}

impl PreviewService {
    /// Creates a new preview service with the given configuration.
    pub fn new(config: PreviewConfig) -> Self {
        Self {
            config,
            syntax_highlighter: Arc::new(Mutex::new(SyntaxHighlighter::new())),
            cache: Arc::new(Mutex::new(None)),
        }
    }

    /// Creates a new preview service with default configuration.
    pub fn with_default_config() -> Self {
        Self::new(PreviewConfig::default())
    }

    /// Generates a preview from pre-extracted text.
    ///
    /// Performs syntax highlighting and search match highlighting on the
    /// provided text. Does not access the filesystem, detect encoding,
    /// or check for binary content — the caller is responsible for
    /// providing the text to display.
    ///
    /// This is the core rendering pipeline. Use [`load_preview`] when you
    /// need to read a file from disk first, or call this directly when
    /// you already have the text (e.g., from a search result).
    pub fn generate_preview(&self, text: &str, path: &Path, query: &str) -> PreviewOutput {
        let lines = self.highlight_content(text, path);
        let mut output = PreviewOutput {
            path: path.to_path_buf(),
            lines,
            truncated: false,
            encoding: Encoding::Utf8,
            is_binary: false,
            error: None,
            original_size: text.len() as u64,
        };
        self.apply_search_highlights(&mut output, query);
        output
    }

    /// Loads a file from disk and generates a preview.
    ///
    /// Reads the file, detects encoding, checks for binary content,
    /// and delegates to [`generate_preview`] for syntax highlighting
    /// and search match highlighting. Results are cached to avoid
    /// redundant file I/O.
    pub fn load_preview(&self, path: &Path, query: &str) -> PreviewOutput {
        // Check cache first
        {
            let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((cached_path, cached_output)) = cache.as_ref() {
                if cached_path == path && cached_output.error.is_none() {
                    let mut output = cached_output.clone();
                    self.apply_search_highlights(&mut output, query);
                    return output;
                }
            }
        }

        let load_config = LoadConfig {
            max_size: self.config.max_file_size,
        };

        let load_result = loader::load_file(path, &load_config);

        if load_result.is_binary {
            return PreviewOutput {
                path: path.to_path_buf(),
                lines: Vec::new(),
                truncated: false,
                encoding: load_result.encoding,
                is_binary: true,
                error: Some("Binary file preview is not available.".to_string()),
                original_size: load_result.original_size,
            };
        }

        if let Some(error) = &load_result.error {
            return PreviewOutput {
                path: path.to_path_buf(),
                lines: Vec::new(),
                truncated: false,
                encoding: load_result.encoding,
                is_binary: false,
                error: Some(error.clone()),
                original_size: load_result.original_size,
            };
        }

        let mut output = self.generate_preview(&load_result.content, path, query);
        output.truncated = load_result.truncated;
        output.encoding = load_result.encoding;
        output.original_size = load_result.original_size;

        // Update cache
        {
            let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            *cache = Some((path.to_path_buf(), output.clone()));
        }

        output
    }

    /// Highlights file content with syntax highlighting.
    fn highlight_content(&self, content: &str, path: &Path) -> Vec<PreviewLine> {
        let highlighter = self
            .syntax_highlighter
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let highlighted_lines = highlighter.highlight_lines(content, path);

        highlighted_lines
            .into_iter()
            .enumerate()
            .map(|(i, line)| PreviewLine {
                line_number: i + 1,
                text: line.text,
                syntax_spans: line.spans,
                match_highlights: Vec::new(),
            })
            .collect()
    }

    /// Applies search match highlights to preview output.
    fn apply_search_highlights(&self, output: &mut PreviewOutput, query: &str) {
        if query.is_empty() {
            return;
        }

        for line in &mut output.lines {
            let matches = highlight::find_matches(&line.text, query);
            line.match_highlights = matches;
        }
    }

    /// Clears the preview cache.
    pub fn clear_cache(&self) {
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        *cache = None;
    }

    /// Returns the current configuration.
    pub fn config(&self) -> &PreviewConfig {
        &self.config
    }
}

impl Default for PreviewService {
    fn default() -> Self {
        Self::with_default_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn create_preview_service() {
        let service = PreviewService::new(PreviewConfig::default());
        assert_eq!(service.config().max_file_size, 1024 * 1024);
    }

    #[test]
    fn preview_text_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "Hello, world!").unwrap();

        let service = PreviewService::new(PreviewConfig::default());
        let output = service.load_preview(&path, "");

        assert!(output.error.is_none());
        assert!(!output.is_binary);
        assert!(!output.lines.is_empty());
        assert_eq!(output.lines[0].text, "Hello, world!");
        assert_eq!(output.lines[0].line_number, 1);
    }

    #[test]
    fn preview_with_search_query() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "Hello world!").unwrap();

        let service = PreviewService::new(PreviewConfig::default());
        let output = service.load_preview(&path, "world");

        assert!(!output.lines[0].match_highlights.is_empty());
        assert_eq!(output.lines[0].match_highlights[0].text, "world");
    }

    #[test]
    fn preview_binary_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.bin");
        std::fs::write(&path, vec![0x00, 0x01, 0x02]).unwrap();

        let service = PreviewService::new(PreviewConfig::default());
        let output = service.load_preview(&path, "");

        assert!(output.is_binary);
        assert!(output.error.is_some());
    }

    #[test]
    fn preview_nonexistent_file() {
        let path = PathBuf::from("/nonexistent/file.txt");
        let service = PreviewService::new(PreviewConfig::default());
        let output = service.load_preview(&path, "");

        assert!(output.error.is_some());
    }

    #[test]
    fn preview_multiline() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "line 1\nline 2\nline 3").unwrap();

        let service = PreviewService::new(PreviewConfig::default());
        let output = service.load_preview(&path, "");

        assert_eq!(output.lines.len(), 3);
        assert_eq!(output.lines[0].line_number, 1);
        assert_eq!(output.lines[1].line_number, 2);
        assert_eq!(output.lines[2].line_number, 3);
    }

    #[test]
    fn preview_cache_works() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "Hello").unwrap();

        let service = PreviewService::new(PreviewConfig::default());
        let output1 = service.load_preview(&path, "");
        let output2 = service.load_preview(&path, "");

        // Should return cached result (same content)
        assert_eq!(output1.lines.len(), output2.lines.len());
    }

    #[test]
    fn clear_cache() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "Hello").unwrap();

        let service = PreviewService::new(PreviewConfig::default());
        let _ = service.load_preview(&path, "");
        service.clear_cache();

        // Cache should be empty now
        let cache = service.cache.lock().unwrap();
        assert!(cache.is_none());
    }

    #[test]
    fn preview_empty_query() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "Hello").unwrap();

        let service = PreviewService::new(PreviewConfig::default());
        let output = service.load_preview(&path, "");

        // Empty query should not add highlights
        for line in &output.lines {
            assert!(line.match_highlights.is_empty());
        }
    }

    #[test]
    fn preview_truncated_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("large.txt");
        let content = "x".repeat(2000);
        std::fs::write(&path, &content).unwrap();

        let config = PreviewConfig {
            max_file_size: 1024,
        };
        let service = PreviewService::new(config);
        let output = service.load_preview(&path, "");

        assert!(output.truncated);
    }

    #[test]
    fn preview_default_config() {
        let service = PreviewService::with_default_config();
        assert_eq!(service.config().max_file_size, 1024 * 1024);
    }

    #[test]
    fn preview_case_insensitive_search() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "Hello World").unwrap();

        let service = PreviewService::new(PreviewConfig::default());
        let output = service.load_preview(&path, "hello");

        assert!(!output.lines[0].match_highlights.is_empty());
    }

    #[test]
    fn generate_preview_basic() {
        let service = PreviewService::with_default_config();
        let output = service.generate_preview("Hello, world!", Path::new("test.txt"), "");

        assert!(output.error.is_none());
        assert!(!output.is_binary);
        assert!(!output.lines.is_empty());
        assert_eq!(output.lines[0].text, "Hello, world!");
        assert_eq!(output.lines[0].line_number, 1);
    }

    #[test]
    fn generate_preview_with_query() {
        let service = PreviewService::with_default_config();
        let output = service.generate_preview("Hello world", Path::new("test.txt"), "world");

        assert!(!output.lines[0].match_highlights.is_empty());
        assert_eq!(output.lines[0].match_highlights[0].text, "world");
    }

    #[test]
    fn generate_preview_multiline() {
        let service = PreviewService::with_default_config();
        let output = service.generate_preview("line 1\nline 2\nline 3", Path::new("test.txt"), "");

        assert_eq!(output.lines.len(), 3);
        assert_eq!(output.lines[0].line_number, 1);
        assert_eq!(output.lines[1].line_number, 2);
        assert_eq!(output.lines[2].line_number, 3);
    }

    #[test]
    fn generate_preview_empty_query() {
        let service = PreviewService::with_default_config();
        let output = service.generate_preview("Hello", Path::new("test.txt"), "");

        for line in &output.lines {
            assert!(line.match_highlights.is_empty());
        }
    }

    #[test]
    fn generate_preview_case_insensitive_search() {
        let service = PreviewService::with_default_config();
        let output = service.generate_preview("Hello World", Path::new("test.txt"), "hello");

        assert!(!output.lines[0].match_highlights.is_empty());
    }
}
