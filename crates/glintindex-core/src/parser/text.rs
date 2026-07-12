//! Plain text file parser.
//!
//! Handles UTF-8 text files and common source code formats.
//! This is the fallback parser for any text-based file.

use std::path::Path;

use crate::error::{GlintIndexError, Result};

use super::trait_impl::{DocumentMetadata, DocumentParser, ParseResult};

/// Parser for plain text files.
///
/// Reads file bytes and decodes them as UTF-8 (lossy).
/// This parser serves as the fallback for any unsupported text format.
pub struct PlainTextParser;

impl PlainTextParser {
    /// Creates a new `PlainTextParser`.
    pub fn new() -> Self {
        Self
    }
}

impl Default for PlainTextParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentParser for PlainTextParser {
    fn supported_extensions(&self) -> &[&str] {
        &[
            "txt",
            "md",
            "log",
            "json",
            "yaml",
            "yml",
            "toml",
            "xml",
            "csv",
            "rs",
            "c",
            "cpp",
            "h",
            "hpp",
            "py",
            "go",
            "java",
            "kt",
            "js",
            "jsx",
            "ts",
            "tsx",
            "html",
            "css",
            "scss",
            "sql",
            "sh",
            "bash",
            "zsh",
            "fish",
            "ps1",
            "bat",
            "cmd",
            "psm1",
            "psd1",
            "rb",
            "php",
            "pl",
            "pm",
            "r",
            "scala",
            "swift",
            "m",
            "mm",
            "cs",
            "fs",
            "fsx",
            "vb",
            "lua",
            "ex",
            "exs",
            "erl",
            "hs",
            "lhs",
            "clj",
            "cljs",
            "ml",
            "mli",
            "zig",
            "nim",
            "cr",
            "d",
            "rs",
            "toml",
            "ini",
            "cfg",
            "conf",
            "env",
            "gitignore",
            "dockerignore",
            "editorconfig",
            "makefile",
            "cmake",
            "gradle",
            "sbt",
            "cabal",
            "gemspec",
            "podspec",
            "csproj",
            "vbproj",
            "fsproj",
            "vcxproj",
            "sln",
        ]
    }

    fn parse(&self, bytes: &[u8], path: &Path) -> Result<ParseResult> {
        let content = String::from_utf8_lossy(bytes).into_owned();

        if content.is_empty() {
            return Err(GlintIndexError::Other(format!(
                "empty file: {}",
                path.display()
            )));
        }

        Ok(ParseResult {
            content,
            metadata: DocumentMetadata::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_utf8_text() {
        let parser = PlainTextParser::new();
        let bytes = b"Hello, world!\nThis is a test.";
        let result = parser.parse(bytes, Path::new("test.txt")).unwrap();
        assert_eq!(result.content, "Hello, world!\nThis is a test.");
    }

    #[test]
    fn parse_empty_file() {
        let parser = PlainTextParser::new();
        let result = parser.parse(b"", Path::new("empty.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn parse_lossy_utf8() {
        let parser = PlainTextParser::new();
        let bytes = &[0xFF, 0xFE, b'h', b'i'];
        let result = parser.parse(bytes, Path::new("bad.txt")).unwrap();
        assert!(result.content.contains("hi"));
    }

    #[test]
    fn supported_extensions_include_common_types() {
        let parser = PlainTextParser::new();
        let exts = parser.supported_extensions();
        assert!(exts.contains(&"txt"));
        assert!(exts.contains(&"md"));
        assert!(exts.contains(&"rs"));
        assert!(exts.contains(&"py"));
        assert!(exts.contains(&"js"));
    }
}
