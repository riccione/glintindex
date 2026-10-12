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
    /// Number of context lines shown above and below each search match.
    pub context_lines: usize,
    /// Number of leading lines shown when the query is empty or matches
    /// no literal substring in the content.
    pub fallback_lines: usize,
    /// Hard cap on the total number of lines emitted for one preview.
    pub max_total_lines: usize,
    /// Maximum rendered characters per line. Longer lines are truncated
    /// with an ellipsis so renderers bound their layout work — a single
    /// minified line can otherwise be megabytes wide.
    pub max_line_chars: usize,
}

impl Default for PreviewConfig {
    fn default() -> Self {
        Self {
            max_file_size: 1024 * 1024, // 1 MB
            context_lines: 3,
            fallback_lines: 50,
            max_total_lines: 500,
            max_line_chars: 500,
        }
    }
}

/// Returns the longest prefix of `text` that is at most `max` bytes,
/// cut at a UTF-8 char boundary.
fn truncate_at_char_boundary(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let mut cut = max;
    while cut > 0 && !text.is_char_boundary(cut) {
        cut -= 1;
    }
    &text[..cut]
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
    ///
    /// Input beyond [`PreviewConfig::max_file_size`] is cut at a char
    /// boundary before highlighting and reported via `truncated`.
    pub fn generate_preview(&self, text: &str, path: &Path, query: &str) -> PreviewOutput {
        // Search results carry the full file content (up to
        // `indexing.max_file_size_mb`). Bound it to the same cap the disk
        // loader enforces so syntax highlighting and match scanning stay
        // proportional to the preview size, not the file size.
        let original_size = text.len() as u64;
        let cap = usize::try_from(self.config.max_file_size).unwrap_or(usize::MAX);
        let bounded = truncate_at_char_boundary(text, cap);
        let lines = self.highlight_content(bounded, path);
        let mut output = PreviewOutput {
            path: path.to_path_buf(),
            lines,
            truncated: bounded.len() < text.len(),
            encoding: Encoding::Utf8,
            is_binary: false,
            error: None,
            original_size,
        };
        self.finalize_preview(&mut output, query);
        output
    }

    /// Loads a file from disk and generates a preview.
    ///
    /// Reads the file, detects encoding, checks for binary content,
    /// and delegates to [`generate_preview`] for syntax highlighting
    /// and search match highlighting. Results are cached to avoid
    /// redundant file I/O.
    pub fn load_preview(&self, path: &Path, query: &str) -> PreviewOutput {
        // Check cache first. The cache stores the full, highlight-free
        // output so each query gets fresh highlights and context filtering.
        {
            let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((cached_path, cached_output)) = cache.as_ref() {
                if cached_path == path && cached_output.error.is_none() {
                    let mut output = cached_output.clone();
                    self.finalize_preview(&mut output, query);
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

        // Build the full, highlight-free output and cache it.
        let mut output = PreviewOutput {
            path: path.to_path_buf(),
            lines: self.highlight_content(&load_result.content, path),
            truncated: load_result.truncated,
            encoding: load_result.encoding,
            is_binary: false,
            error: None,
            original_size: load_result.original_size,
        };

        {
            let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            *cache = Some((path.to_path_buf(), output.clone()));
        }

        self.finalize_preview(&mut output, query);
        output
    }

    /// Applies search highlights and filters lines to the context window.
    ///
    /// This is the per-query pass over a full (unhighlighted) line set.
    fn finalize_preview(&self, output: &mut PreviewOutput, query: &str) {
        self.apply_search_highlights(output, query);
        self.filter_to_context(output, query);
        self.cap_line_lengths(output);
    }

    /// Truncates overlong lines so renderers lay out a bounded amount of
    /// text per frame.
    fn cap_line_lengths(&self, output: &mut PreviewOutput) {
        let max = self.config.max_line_chars;
        if max == 0 {
            return;
        }
        for line in &mut output.lines {
            if line.text.len() <= max {
                continue;
            }
            let cut = truncate_at_char_boundary(&line.text, max).len();
            // Clamp offsets while the byte prefix is still intact, then
            // cut the text and append an ellipsis.
            line.syntax_spans.retain_mut(|span| {
                if span.0 >= cut {
                    return false;
                }
                span.1 = span.1.min(cut);
                true
            });
            line.match_highlights.retain_mut(|m| {
                if m.start >= cut {
                    return false;
                }
                m.end = m.end.min(cut);
                true
            });
            line.text.truncate(cut);
            line.text.push('…');
            for m in &mut line.match_highlights {
                if let Some(slice) = line.text.get(m.start..m.end) {
                    m.text = slice.to_string();
                }
            }
        }
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

    /// Filters preview lines to a context window around search matches.
    ///
    /// With a non-empty query that matches at least one line, keeps only
    /// matched lines plus [`PreviewConfig::context_lines`] above and below,
    /// merging overlapping windows and preserving original line numbers.
    /// When the query is empty or matches nothing literally (e.g. prefix
    /// or fuzzy index hits), falls back to the first
    /// [`PreviewConfig::fallback_lines`] lines. The total number of lines
    /// is capped at [`PreviewConfig::max_total_lines`]. Sets `truncated`
    /// when lines are dropped.
    fn filter_to_context(&self, output: &mut PreviewOutput, query: &str) {
        let total = output.lines.len();
        if total == 0 {
            return;
        }

        let matched: Vec<usize> = output
            .lines
            .iter()
            .enumerate()
            .filter(|(_, line)| !line.match_highlights.is_empty())
            .map(|(i, _)| i)
            .collect();

        let selected: Vec<usize> = if !query.is_empty() && !matched.is_empty() {
            let ctx = self.config.context_lines;
            let mut keep = vec![false; total];
            for &i in &matched {
                let start = i.saturating_sub(ctx);
                let end = (i + ctx).min(total - 1);
                keep[start..=end].fill(true);
            }
            keep.iter()
                .copied()
                .enumerate()
                .filter(|&(_, keep)| keep)
                .map(|(i, _)| i)
                .collect()
        } else {
            // Empty query or no literal match: leading lines only.
            let limit = self.config.fallback_lines.min(total);
            (0..limit).collect()
        };

        let cap = self.config.max_total_lines;
        let truncated = selected.len() < total || selected.len() > cap;
        if !truncated {
            // Already contiguous from line 1 — nothing to filter.
            return;
        }

        let mut keep = vec![false; total];
        for &i in selected.iter().take(cap) {
            keep[i] = true;
        }
        let all = std::mem::take(&mut output.lines);
        output.lines = all
            .into_iter()
            .enumerate()
            .filter(|(i, _)| keep[*i])
            .map(|(_, line)| line)
            .collect();
        output.truncated = output.truncated || truncated;
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
            ..Default::default()
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

    fn multiline_content(n: usize, needle_line: usize, needle: &str) -> String {
        (1..=n)
            .map(|i| {
                if i == needle_line {
                    format!("line {i} has {needle} inside")
                } else {
                    format!("line {i} has nothing")
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn preview_context_window_around_matches() {
        let content = multiline_content(20, 10, "target");
        let service = PreviewService::with_default_config();
        let output = service.generate_preview(&content, Path::new("test.txt"), "target");

        // Match on line 10 ± 3 context → lines 7..=13
        let numbers: Vec<usize> = output.lines.iter().map(|l| l.line_number).collect();
        assert_eq!(numbers, (7..=13).collect::<Vec<_>>());
        assert!(output.truncated);
        // The matched line keeps its highlight
        let matched_line = output.lines.iter().find(|l| l.line_number == 10).unwrap();
        assert!(!matched_line.match_highlights.is_empty());
        // Context lines have no highlights
        let context_line = output.lines.iter().find(|l| l.line_number == 7).unwrap();
        assert!(context_line.match_highlights.is_empty());
    }

    #[test]
    fn preview_context_preserves_original_line_numbers() {
        let content = multiline_content(30, 20, "needle");
        let service = PreviewService::with_default_config();
        let output = service.generate_preview(&content, Path::new("test.txt"), "needle");

        assert_eq!(output.lines.first().unwrap().line_number, 17);
        assert_eq!(output.lines.last().unwrap().line_number, 23);
        assert_eq!(output.lines.len(), 7);
    }

    #[test]
    fn preview_merges_overlapping_windows() {
        // Same query matches on lines 5 and 9; context 3 gives windows
        // 2..=7 and 6..=12 which overlap and merge to a contiguous block.
        let mut lines: Vec<String> = (1..=20).map(|i| format!("line {i}")).collect();
        lines[4] = "line 5 has alpha here".to_string();
        lines[8] = "line 9 has alpha here".to_string();
        let service = PreviewService::with_default_config();
        let output = service.generate_preview(&lines.join("\n"), Path::new("test.txt"), "alpha");

        let numbers: Vec<usize> = output.lines.iter().map(|l| l.line_number).collect();
        assert_eq!(numbers, (2..=12).collect::<Vec<_>>());
    }

    #[test]
    fn preview_empty_query_falls_back_to_leading_lines() {
        let content = multiline_content(80, 1, "unused");
        let service = PreviewService::with_default_config();
        let output = service.generate_preview(&content, Path::new("test.txt"), "");

        assert_eq!(output.lines.len(), 50);
        assert_eq!(output.lines[0].line_number, 1);
        assert_eq!(output.lines[49].line_number, 50);
        assert!(output.truncated);
    }

    #[test]
    fn preview_no_literal_match_falls_back_to_leading_lines() {
        let content = multiline_content(80, 40, "present");
        let service = PreviewService::with_default_config();
        // Query is not a literal substring of any line
        let output = service.generate_preview(&content, Path::new("test.txt"), "absent-query");

        assert_eq!(output.lines.len(), 50);
        assert!(output.truncated);
        for line in &output.lines {
            assert!(line.match_highlights.is_empty());
        }
    }

    #[test]
    fn preview_max_total_lines_cap() {
        let content = multiline_content(100, 50, "everywhere");
        let config = PreviewConfig {
            context_lines: 3,
            max_total_lines: 5,
            ..Default::default()
        };
        let service = PreviewService::new(config);
        let output = service.generate_preview(&content, Path::new("test.txt"), "everywhere");

        assert_eq!(output.lines.len(), 5);
        assert!(output.truncated);
    }

    #[test]
    fn preview_small_file_unfiltered_with_empty_query() {
        let service = PreviewService::with_default_config();
        let output = service.generate_preview("a\nb\nc", Path::new("test.txt"), "");

        assert_eq!(output.lines.len(), 3);
        assert!(!output.truncated);
    }

    #[test]
    fn cache_reuse_clears_stale_highlights_for_empty_query() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "Hello world, brave world").unwrap();

        let service = PreviewService::with_default_config();
        let first = service.load_preview(&path, "world");
        assert!(!first.lines[0].match_highlights.is_empty());

        // Second call hits the cache with an empty query — highlights
        // from the cached first query must not leak through.
        let second = service.load_preview(&path, "");
        for line in &second.lines {
            assert!(line.match_highlights.is_empty());
        }
    }

    #[test]
    fn cache_reuse_recomputes_context_per_query() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        // Matches on line 3 and line 18 of the same cached file.
        let mut lines: Vec<String> = (1..=20).map(|i| format!("line {i}")).collect();
        lines[2] = "line 3 has alpha".to_string();
        lines[17] = "line 18 has omega".to_string();
        std::fs::write(&path, lines.join("\n")).unwrap();

        let service = PreviewService::with_default_config();
        let first = service.load_preview(&path, "alpha");
        let first_numbers: Vec<usize> = first.lines.iter().map(|l| l.line_number).collect();
        assert_eq!(first_numbers, (1..=6).collect::<Vec<_>>());

        // Same cached file, query matching a different line — context moves.
        let second = service.load_preview(&path, "omega");
        let second_numbers: Vec<usize> = second.lines.iter().map(|l| l.line_number).collect();
        assert_eq!(second_numbers, (15..=20).collect::<Vec<_>>());
    }

    #[test]
    fn generate_preview_caps_input_to_max_file_size() {
        let config = PreviewConfig {
            max_file_size: 64,
            ..Default::default()
        };
        let service = PreviewService::new(config);
        let content = "line one\nline two\n".repeat(10); // 180 bytes
        let output = service.generate_preview(&content, Path::new("test.txt"), "");

        assert!(output.truncated);
        assert_eq!(output.original_size, 180);
        // Highlighting and matching only saw the bounded slice; line
        // texts (without newlines) therefore fit well within the cap.
        let total: usize = output.lines.iter().map(|l| l.text.len()).sum();
        assert!(total < 64, "highlighted content exceeded cap: {total}");
        assert!(!output.lines.is_empty());
    }

    #[test]
    fn generate_preview_truncates_overlong_lines() {
        let service = PreviewService::with_default_config(); // max_line_chars = 500
        let long = "a".repeat(5_000);
        let content = format!("{long}\nshort needle line");
        let output = service.generate_preview(&content, Path::new("test.rs"), "aaaaa");

        let capped = output
            .lines
            .iter()
            .find(|l| l.text.starts_with('a'))
            .expect("long line present");
        assert_eq!(capped.text.chars().count(), 501); // 500 chars + ellipsis
        assert!(capped.text.ends_with('…'));

        // Offsets and spans stay within the cut so renderers never walk
        // past the truncated text.
        for span in &capped.syntax_spans {
            assert!(span.0 < 500 && span.1 <= 500, "span out of range: {span:?}");
        }
        for m in &capped.match_highlights {
            assert!(m.start < m.end && m.end <= 500, "match out of range: {m:?}");
            assert_eq!(m.text, &capped.text[m.start..m.end]);
        }
        assert!(!capped.match_highlights.is_empty());
    }

    #[test]
    fn generate_preview_short_lines_untouched_by_line_cap() {
        let service = PreviewService::with_default_config();
        let output = service.generate_preview("hello world", Path::new("test.txt"), "world");

        assert_eq!(output.lines[0].text, "hello world");
        assert!(!output.truncated);
    }
}
