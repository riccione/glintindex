//! Parser registry for selecting the appropriate document parser.
//!
//! The [`ParserRegistry`] maintains a collection of parsers and selects
//! the correct one based on file extension. The scanner never needs to
//! know about concrete parser implementations — it only interacts
//! with the registry.

use std::path::Path;

use super::DocumentParser;
use super::text::PlainTextParser;

/// Registry that maps file extensions to parser implementations.
///
/// The registry is created once and shared across the scanning pipeline.
/// It holds parser instances and selects the appropriate one for each file.
///
/// # Examples
///
/// ```ignore
/// use glintindex_core::parser::ParserRegistry;
/// use std::path::Path;
///
/// let registry = ParserRegistry::new();
/// let parser = registry.parser_for(Path::new("document.pdf"));
/// ```
pub struct ParserRegistry {
    parsers: Vec<Box<dyn DocumentParser>>,
    plain_text: PlainTextParser,
}

impl ParserRegistry {
    /// Creates a new registry with all built-in parsers registered.
    ///
    /// The registry includes parsers for:
    /// - Plain text (fallback)
    /// - PDF
    /// - DOCX
    /// - XLSX
    /// - PPTX
    /// - RTF
    /// - ODT
    pub fn new() -> Self {
        let mut registry = Self {
            parsers: Vec::new(),
            plain_text: PlainTextParser::new(),
        };

        // Register format-specific parsers
        // Order matters: first match wins
        #[cfg(feature = "parser-pdf")]
        registry
            .parsers
            .push(Box::new(super::pdf::PdfParser::new()));
        #[cfg(feature = "parser-docx")]
        registry
            .parsers
            .push(Box::new(super::docx::DocxParser::new()));
        #[cfg(feature = "parser-xlsx")]
        registry
            .parsers
            .push(Box::new(super::xlsx::XlsxParser::new()));
        #[cfg(feature = "parser-pptx")]
        registry
            .parsers
            .push(Box::new(super::pptx::PptxParser::new()));
        #[cfg(feature = "parser-rtf")]
        registry
            .parsers
            .push(Box::new(super::rtf::RtfParser::new()));
        #[cfg(feature = "parser-odt")]
        registry
            .parsers
            .push(Box::new(super::odt::OdtParser::new()));

        registry
    }

    /// Returns the appropriate parser for the given file path.
    ///
    /// Checks each registered parser's [`can_parse`](DocumentParser::can_parse)
    /// method in order. Falls back to the plain text parser if no
    /// format-specific parser matches.
    pub fn parser_for(&self, path: &Path) -> &dyn DocumentParser {
        for parser in &self.parsers {
            if parser.can_parse(path) {
                return parser.as_ref();
            }
        }
        &self.plain_text
    }

    /// Returns all file extensions supported by any registered parser.
    ///
    /// Includes both format-specific and plain text extensions.
    pub fn supported_extensions(&self) -> Vec<&str> {
        let mut exts: Vec<&str> = self
            .parsers
            .iter()
            .flat_map(|p| p.supported_extensions())
            .copied()
            .collect();
        exts.extend(self.plain_text.supported_extensions());
        exts.sort();
        exts.dedup();
        exts
    }

    /// Returns `true` if the file extension is supported by any parser.
    pub fn is_supported(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|ext| {
                self.parsers
                    .iter()
                    .any(|p| p.supported_extensions().contains(&ext))
                    || self.plain_text.supported_extensions().contains(&ext)
            })
            .unwrap_or(false)
    }
}

impl Default for ParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_selects_plain_text() {
        let registry = ParserRegistry::new();
        let parser = registry.parser_for(Path::new("test.txt"));
        assert!(parser.can_parse(Path::new("test.txt")));
    }

    #[test]
    fn registry_supported_extensions_not_empty() {
        let registry = ParserRegistry::new();
        let exts = registry.supported_extensions();
        assert!(!exts.is_empty());
        assert!(exts.contains(&"txt"));
    }

    #[test]
    fn registry_is_supported() {
        let registry = ParserRegistry::new();
        assert!(registry.is_supported(Path::new("file.txt")));
        assert!(registry.is_supported(Path::new("file.rs")));
    }
}
