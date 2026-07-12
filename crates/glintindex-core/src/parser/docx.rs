//! DOCX document parser.
//!
//! Extracts text content from Microsoft Word DOCX files using the `docx-lite` crate.
//! Handles paragraphs, tables, and basic document structure.

use std::path::Path;

use crate::error::{GlintIndexError, Result};

use super::trait_impl::{DocumentMetadata, DocumentParser, ParseResult};

/// Parser for Microsoft Word DOCX documents.
///
/// Uses `docx-lite` to extract text content from DOCX files.
/// Extracts paragraphs, tables, and basic document structure.
pub struct DocxParser;

impl DocxParser {
    /// Creates a new `DocxParser`.
    pub fn new() -> Self {
        Self
    }
}

impl Default for DocxParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentParser for DocxParser {
    fn supported_extensions(&self) -> &[&str] {
        &["docx", "docm"]
    }

    fn parse(&self, bytes: &[u8], path: &Path) -> Result<ParseResult> {
        let content = docx_lite::extract_text_from_bytes(bytes).map_err(|e| {
            GlintIndexError::Other(format!(
                "DOCX extraction failed for {}: {}",
                path.display(),
                e
            ))
        })?;

        if content.trim().is_empty() {
            return Err(GlintIndexError::Other(format!(
                "DOCX contains no extractable text: {}",
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
        let parser = DocxParser::new();
        assert!(parser.supported_extensions().contains(&"docx"));
        assert!(parser.supported_extensions().contains(&"docm"));
    }

    #[test]
    fn can_parse_docx() {
        let parser = DocxParser::new();
        assert!(parser.can_parse(Path::new("document.docx")));
        assert!(parser.can_parse(Path::new("document.docm")));
        assert!(!parser.can_parse(Path::new("document.pdf")));
    }
}
