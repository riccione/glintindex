//! Preview module for file content display.
//!
//! This module provides a complete file preview system with:
//! - Syntax highlighting using `syntect`
//! - Encoding detection (UTF-8, UTF-16, binary)
//! - Search match highlighting
//! - Large-file handling with configurable limits
//! - Line numbers
//!
//! The GUI should communicate only through [`PreviewService`].
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │                  GUI                            │
//! │         (preview widget)                        │
//! └─────────────────────┬───────────────────────────┘
//!                       │
//! ┌─────────────────────▼───────────────────────────┐
//! │              PreviewService                     │
//! │  - load_preview()                               │
//! │  - clear_cache()                                │
//! └─────────────────────┬───────────────────────────┘
//!                       │
//!     ┌─────────────────┼─────────────────┐
//!     │                 │                 │
//! ┌───▼───┐        ┌────▼────┐       ┌────▼────┐
//! │loader │        │ syntax  │       │encoding │
//! │       │        │         │       │         │
//! └───────┘        └─────────┘       └─────────┘
//! ```
//!
//! # Usage
//!
//! ```no_run
//! use std::path::Path;
//! use glintindex_core::preview::{PreviewService, PreviewConfig};
//!
//! let service = PreviewService::new(PreviewConfig::default());
//! let output = service.load_preview(Path::new("main.rs"), "fn");
//!
//! for line in &output.lines {
//!     println!("{}: {}", line.line_number, line.text);
//! }
//! ```

pub mod encoding;
pub mod highlight;
pub mod loader;
pub mod service;
pub mod syntax;

pub use encoding::{Encoding, EncodingResult};
pub use highlight::HighlightedMatch;
pub use loader::{LoadConfig, LoadResult};
pub use service::{PreviewConfig, PreviewLine, PreviewOutput, PreviewService};
pub use syntax::{HighlightedLine, Style, SyntaxHighlighter};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn preview_module_integration() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.rs");
        std::fs::write(&path, "fn main() {\n    println!(\"Hello\");\n}").unwrap();

        let service = PreviewService::new(PreviewConfig::default());
        let output = service.load_preview(&path, "Hello");

        assert!(output.error.is_none());
        assert!(!output.lines.is_empty());
        assert_eq!(output.lines.len(), 3);
        assert_eq!(output.lines[0].line_number, 1);
    }

    #[test]
    fn preview_encoding_detection() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "Hello, world!").unwrap();

        let result = encoding::detect_from_file(&path).unwrap();
        assert_eq!(result.encoding, Encoding::Utf8);
        assert!(!result.is_binary);
    }

    #[test]
    fn preview_syntax_detection() {
        let highlighter = SyntaxHighlighter::new();
        let syntax = highlighter.detect_syntax(&PathBuf::from("test.py"));
        assert!(syntax.is_some());
        assert!(!syntax.unwrap().name.is_empty());
    }

    #[test]
    fn preview_match_finding() {
        let matches = highlight::find_matches("hello world hello", "hello");
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn preview_line_numbering() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "a\nb\nc").unwrap();

        let service = PreviewService::new(PreviewConfig::default());
        let output = service.load_preview(&path, "");

        assert_eq!(output.lines[0].line_number, 1);
        assert_eq!(output.lines[1].line_number, 2);
        assert_eq!(output.lines[2].line_number, 3);
    }
}
