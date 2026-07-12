//! Document parser framework for rich document support.
//!
//! This module provides a trait-based parser architecture for extracting
//! searchable text from various document formats. The scanner uses the
//! [`ParserRegistry`] to automatically select the correct parser based
//! on file extension.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                    Scanner                          │
//! │   (discovers files, reads bytes, indexes content)   │
//! └───────────────────────┬─────────────────────────────┘
//!                         │
//! ┌───────────────────────▼─────────────────────────────┐
//! │                ParserRegistry                       │
//! │   (selects parser by extension, fallback to text)   │
//! └───────┬───────────┬───────────┬───────────┬─────────┘
//!         │           │           │           │
//!     ┌───▼──┐    ┌───▼──┐    ┌───▼──┐    ┌───▼──┐
//!     │ PDF  │    │ DOCX │    │ XLSX │    │ ...  │
//!     └──────┘    └──────┘    └──────┘    └──────┘
//! ```
//!
//! # Supported Formats
//!
//! | Format | Extension | Parser |
//! |--------|-----------|--------|
//! | Plain text | txt, md, rs, py, ... | [`PlainTextParser`] |
//! | PDF | pdf | [`PdfParser`] |
//! | Word | docx | [`DocxParser`] |
//! | Excel | xlsx | [`XlsxParser`] |
//! | PowerPoint | pptx | [`PptxParser`] |
//! | RTF | rtf | [`RtfParser`] |
//! | OpenDocument Text | odt | [`OdtParser`] |
//!
//! # Adding New Parsers
//!
//! 1. Create a new module (e.g., `src/parser/myformat.rs`)
//! 2. Implement the [`DocumentParser`] trait
//! 3. Register the parser in [`ParserRegistry::new()`]
//! 4. Add the feature flag to `Cargo.toml`

pub mod registry;
pub mod text;
pub mod trait_impl;

#[cfg(feature = "parser-docx")]
pub mod docx;
#[cfg(feature = "parser-odt")]
pub mod odt;
#[cfg(feature = "parser-pdf")]
pub mod pdf;
#[cfg(feature = "parser-pptx")]
pub mod pptx;
#[cfg(feature = "parser-rtf")]
pub mod rtf;
#[cfg(feature = "parser-xlsx")]
pub mod xlsx;

pub use registry::ParserRegistry;
pub use text::PlainTextParser;
pub use trait_impl::{DocumentMetadata, DocumentParser, ParseResult};
