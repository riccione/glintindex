//! XLSX spreadsheet parser.
//!
//! Extracts text content from Excel XLSX files using the `calamine` crate.
//! Iterates through all sheets and rows, joining cell values into
//! searchable text.

use std::path::Path;

use calamine::{Reader, Xlsx, open_workbook_from_rs};
use std::io::Cursor;

use crate::error::{GlintIndexError, Result};

use super::trait_impl::{DocumentMetadata, DocumentParser, ParseResult};

/// Parser for Microsoft Excel XLSX documents.
///
/// Uses `calamine` to read spreadsheet data and extract cell values
/// as searchable text. Iterates through all sheets and rows.
pub struct XlsxParser;

impl XlsxParser {
    /// Creates a new `XlsxParser`.
    pub fn new() -> Self {
        Self
    }
}

impl Default for XlsxParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentParser for XlsxParser {
    fn supported_extensions(&self) -> &[&str] {
        &["xlsx", "xlsm", "xlsb", "xls"]
    }

    fn parse(&self, bytes: &[u8], path: &Path) -> Result<ParseResult> {
        let cursor = Cursor::new(bytes);
        let mut workbook: Xlsx<_> = open_workbook_from_rs(cursor).map_err(|e| {
            GlintIndexError::Other(format!(
                "XLSX extraction failed for {}: {}",
                path.display(),
                e
            ))
        })?;

        let sheet_names = workbook.sheet_names().to_vec();
        let mut content = String::new();

        for name in &sheet_names {
            if let Ok(range) = workbook.worksheet_range(name) {
                for row in range.rows() {
                    let row_text: Vec<String> = row
                        .iter()
                        .map(|cell| match cell {
                            calamine::Data::Empty => String::new(),
                            calamine::Data::String(s) => s.clone(),
                            calamine::Data::Float(f) => format!("{}", f),
                            calamine::Data::Int(i) => format!("{}", i),
                            calamine::Data::Bool(b) => format!("{}", b),
                            calamine::Data::Error(e) => format!("{:?}", e),
                            calamine::Data::DateTime(dt) => format!("{}", dt),
                            calamine::Data::DateTimeIso(s) => s.clone(),
                            calamine::Data::DurationIso(s) => s.clone(),
                        })
                        .filter(|s| !s.is_empty())
                        .collect();

                    if !row_text.is_empty() {
                        content.push_str(&row_text.join("\t"));
                        content.push('\n');
                    }
                }
            }
        }

        if content.trim().is_empty() {
            return Err(GlintIndexError::Other(format!(
                "XLSX contains no extractable text: {}",
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
        let parser = XlsxParser::new();
        assert!(parser.supported_extensions().contains(&"xlsx"));
        assert!(parser.supported_extensions().contains(&"xlsm"));
    }

    #[test]
    fn can_parse_xlsx() {
        let parser = XlsxParser::new();
        assert!(parser.can_parse(Path::new("spreadsheet.xlsx")));
        assert!(!parser.can_parse(Path::new("document.pdf")));
    }
}
