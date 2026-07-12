//! PDF document parser.
//!
//! Extracts text content from PDF files using the `pdf-extract` crate.
//! This parser handles standard PDF text extraction but does not
//! support OCR or image-based PDFs.

use std::path::Path;

use crate::error::{GlintIndexError, Result};

use super::trait_impl::{DocumentMetadata, DocumentParser, ParseResult};

/// Parser for PDF documents.
///
/// Uses `pdf-extract` to extract text content from PDF files.
/// Metadata extraction is limited to what `pdf-extract` provides.
pub struct PdfParser;

impl PdfParser {
    /// Creates a new `PdfParser`.
    pub fn new() -> Self {
        Self
    }
}

impl Default for PdfParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentParser for PdfParser {
    fn supported_extensions(&self) -> &[&str] {
        &["pdf"]
    }

    fn parse(&self, bytes: &[u8], path: &Path) -> Result<ParseResult> {
        let content = pdf_extract::extract_text_from_mem(bytes).map_err(|e| {
            GlintIndexError::Other(format!(
                "PDF extraction failed for {}: {}",
                path.display(),
                e
            ))
        })?;

        if content.trim().is_empty() {
            return Err(GlintIndexError::Other(format!(
                "PDF contains no extractable text: {}",
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
        let parser = PdfParser::new();
        assert_eq!(parser.supported_extensions(), &["pdf"]);
    }

    #[test]
    fn can_parse_pdf() {
        let parser = PdfParser::new();
        assert!(parser.can_parse(Path::new("document.pdf")));
        assert!(!parser.can_parse(Path::new("document.txt")));
    }
}
