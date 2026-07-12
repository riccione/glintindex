//! RTF document parser.
//!
//! Extracts text content from Rich Text Format (RTF) files using the `rtf-parser` crate.
//! Handles RTF 1.9 specification with UTF-16 unicode support.

use std::path::Path;

use crate::error::{GlintIndexError, Result};

use super::trait_impl::{DocumentMetadata, DocumentParser, ParseResult};

/// Parser for Rich Text Format (RTF) documents.
///
/// Uses `rtf-parser` to parse RTF files and extract plain text content.
/// Handles unicode characters and special formatting.
pub struct RtfParser;

impl RtfParser {
    /// Creates a new `RtfParser`.
    pub fn new() -> Self {
        Self
    }
}

impl Default for RtfParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentParser for RtfParser {
    fn supported_extensions(&self) -> &[&str] {
        &["rtf"]
    }

    fn parse(&self, bytes: &[u8], path: &Path) -> Result<ParseResult> {
        let rtf_str = std::str::from_utf8(bytes).map_err(|e| {
            GlintIndexError::Other(format!("RTF encoding error for {}: {}", path.display(), e))
        })?;

        let tokens = rtf_parser::Lexer::scan(rtf_str).map_err(|e| {
            GlintIndexError::Other(format!("RTF lexing failed for {}: {}", path.display(), e))
        })?;

        let mut parser = rtf_parser::Parser::new(tokens);
        let document = parser.parse().map_err(|e| {
            GlintIndexError::Other(format!("RTF parsing failed for {}: {}", path.display(), e))
        })?;

        let content = document.get_text();

        if content.trim().is_empty() {
            return Err(GlintIndexError::Other(format!(
                "RTF contains no extractable text: {}",
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
    fn supported_extensions() {
        let parser = RtfParser::new();
        assert_eq!(parser.supported_extensions(), &["rtf"]);
    }

    #[test]
    fn can_parse_rtf() {
        let parser = RtfParser::new();
        assert!(parser.can_parse(Path::new("document.rtf")));
        assert!(!parser.can_parse(Path::new("document.txt")));
    }

    #[test]
    fn parse_simple_rtf() {
        let parser = RtfParser::new();
        let rtf = r#"{\rtf1\ansi{\fonttbl\f0\fswiss Helvetica;}\f0\pard Hello {\b World}.\par}"#;
        let result = parser.parse(rtf.as_bytes(), Path::new("test.rtf")).unwrap();
        assert!(result.content.contains("Hello"));
        assert!(result.content.contains("World"));
    }
}
