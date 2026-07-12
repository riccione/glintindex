//! ODT document parser.
//!
//! Extracts text content from OpenDocument Text (ODT) files.
//! Parses the content.xml file within the ODT ZIP archive to extract
//! text paragraphs and tables.

use std::io::Read;
use std::path::Path;

use quick_xml::Reader;
use quick_xml::events::Event;
use zip::ZipArchive;

use crate::error::{GlintIndexError, Result};

use super::trait_impl::{DocumentMetadata, DocumentParser, ParseResult};

/// Parser for OpenDocument Text (ODT) documents.
///
/// Opens the ODT file as a ZIP archive and parses `content.xml`
/// to extract text content from paragraphs and tables.
pub struct OdtParser;

impl OdtParser {
    /// Creates a new `OdtParser`.
    pub fn new() -> Self {
        Self
    }
}

impl Default for OdtParser {
    fn default() -> Self {
        Self::new()
    }
}

impl OdtParser {
    /// Extracts text from ODT content.xml string.
    fn extract_text_from_content_xml(xml: &str) -> String {
        let mut text = String::new();
        let mut reader = Reader::from_str(xml);
        let mut buf = Vec::new();
        let mut in_text_content = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let name = e.name();
                    let tag = String::from_utf8_lossy(name.as_ref());
                    // Text content elements
                    if tag == "text:p"
                        || tag == "text:h"
                        || tag == "text:span"
                        || tag == "text:a"
                        || tag == "text:s"
                    {
                        in_text_content = true;
                    }
                    // Table cells - add separator
                    if tag == "table:table-cell" || tag == "table:covered-table-cell" {
                        text.push('\t');
                    }
                    // Table rows - add newline
                    if tag == "table:table-row" {
                        text.push('\n');
                    }
                }
                Ok(Event::Text(ref e)) if in_text_content => {
                    if let Ok(content) = e.xml10_content() {
                        text.push_str(&content);
                    }
                }
                Ok(Event::End(ref e)) => {
                    let name = e.name();
                    let tag = String::from_utf8_lossy(name.as_ref());
                    if tag == "text:p"
                        || tag == "text:h"
                        || tag == "text:span"
                        || tag == "text:a"
                        || tag == "text:s"
                    {
                        in_text_content = false;
                        text.push('\n');
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    tracing::warn!("ODT XML parsing error: {}", e);
                    break;
                }
                _ => {}
            }
            buf.clear();
        }

        text
    }
}

impl DocumentParser for OdtParser {
    fn supported_extensions(&self) -> &[&str] {
        &["odt"]
    }

    fn parse(&self, bytes: &[u8], path: &Path) -> Result<ParseResult> {
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor).map_err(|e| {
            GlintIndexError::Other(format!(
                "ODT ZIP extraction failed for {}: {}",
                path.display(),
                e
            ))
        })?;

        let mut content_xml = String::new();
        archive
            .by_name("content.xml")
            .map_err(|e| {
                GlintIndexError::Other(format!(
                    "ODT missing content.xml for {}: {}",
                    path.display(),
                    e
                ))
            })?
            .read_to_string(&mut content_xml)
            .map_err(|e| {
                GlintIndexError::Other(format!(
                    "Failed to read ODT content.xml for {}: {}",
                    path.display(),
                    e
                ))
            })?;

        let content = Self::extract_text_from_content_xml(&content_xml);

        if content.trim().is_empty() {
            return Err(GlintIndexError::Other(format!(
                "ODT contains no extractable text: {}",
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
        let parser = OdtParser::new();
        assert_eq!(parser.supported_extensions(), &["odt"]);
    }

    #[test]
    fn can_parse_odt() {
        let parser = OdtParser::new();
        assert!(parser.can_parse(Path::new("document.odt")));
        assert!(!parser.can_parse(Path::new("document.docx")));
    }

    #[test]
    fn extract_text_from_paragraph() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content>
  <office:body>
    <office:text>
      <text:p>Hello World</text:p>
      <text:p>Second paragraph</text:p>
    </office:text>
  </office:body>
</office:document-content>"#;
        let text = OdtParser::extract_text_from_content_xml(xml);
        assert!(text.contains("Hello World"));
        assert!(text.contains("Second paragraph"));
    }
}
