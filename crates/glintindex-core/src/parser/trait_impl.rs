//! Core trait and types for document parsers.
//!
//! The [`DocumentParser`] trait defines the interface that all document
//! parsers must implement. Each parser extracts searchable text and
//! optional metadata from a specific document format.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │              ParserRegistry                 │
//! │  (selects parser by file extension)         │
//! └─────────────┬───────────────────────────────┘
//!               │
//!     ┌─────────┼─────────┐
//!     │         │         │
//! ┌───▼──┐ ┌───▼──┐ ┌───▼──┐
//! │ PDF  │ │ DOCX │ │ ...  │
//! └──────┘ └──────┘ └──────┘
//! ```
//!
//! # Adding New Parsers
//!
//! 1. Create a new module in `src/parser/`
//! 2. Implement `DocumentParser` for your struct
//! 3. Register the parser in `ParserRegistry::new()`

use std::path::Path;

use crate::error::Result;

/// Metadata extracted from a document.
///
/// Missing fields are represented as `None` and should not be treated
/// as errors. Not all formats support all metadata fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocumentMetadata {
    /// The document title, if available.
    pub title: Option<String>,
    /// The document author, if available.
    pub author: Option<String>,
    /// The document subject or description, if available.
    pub subject: Option<String>,
}

/// The result of parsing a document.
///
/// Contains the extracted text content and optional metadata.
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// The extracted plain text content, suitable for indexing.
    pub content: String,
    /// Optional metadata extracted from the document.
    pub metadata: DocumentMetadata,
}

/// A trait for extracting text content from documents.
///
/// Each implementation handles a specific document format (PDF, DOCX, etc.).
/// Parsers should be stateless and independent — all data comes through
/// the [`parse`](DocumentParser::parse) method parameters.
///
/// # Error Handling
///
/// Parsers should return meaningful errors for corrupted or unsupported
/// documents. The scanner will catch these errors and continue processing
/// other files.
///
/// # Examples
///
/// ```ignore
/// use glintindex_core::parser::DocumentParser;
/// use std::path::Path;
///
/// struct MyParser;
///
/// impl DocumentParser for MyParser {
///     fn supported_extensions(&self) -> &[&str] {
///         &["myformat"]
///     }
///
///     fn can_parse(&self, path: &Path) -> bool {
///         path.extension()
///             .and_then(|e| e.to_str())
///             .map(|ext| ext == "myformat")
///             .unwrap_or(false)
///     }
///
///     fn parse(&self, bytes: &[u8], _path: &Path) -> glintindex_core::Result<ParseResult> {
///         let content = String::from_utf8_lossy(bytes).into_owned();
///         Ok(ParseResult {
///             content,
///             metadata: DocumentMetadata::default(),
///         })
///     }
/// }
/// ```
pub trait DocumentParser: Send + Sync {
    /// Returns the file extensions this parser supports (without the dot).
    ///
    /// Examples: `&["pdf"]`, `&["docx", "docm"]`
    fn supported_extensions(&self) -> &[&str];

    /// Returns `true` if this parser can handle the file at the given path.
    ///
    /// The default implementation checks the file extension against
    /// [`supported_extensions`](DocumentParser::supported_extensions).
    /// Override this for more sophisticated detection.
    fn can_parse(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|ext| self.supported_extensions().contains(&ext))
            .unwrap_or(false)
    }

    /// Extracts text content and metadata from the given bytes.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The raw file contents
    /// * `path` - The file path (for context in error messages)
    ///
    /// # Errors
    ///
    /// Returns an error if the document is corrupted or cannot be parsed.
    /// The error should be descriptive enough for logging.
    fn parse(&self, bytes: &[u8], path: &Path) -> Result<ParseResult>;
}
