//! PPTX presentation parser.
//!
//! Extracts text content from Microsoft PowerPoint PPTX files.
//! Parses slide XML to extract text nodes from text boxes and shapes.

use std::io::Read;
use std::path::Path;

use zip::ZipArchive;

use crate::error::{GlintIndexError, Result};

use super::trait_impl::{DocumentMetadata, DocumentParser, ParseResult};

/// Parser for Microsoft PowerPoint PPTX documents.
///
/// Opens the PPTX file as a ZIP archive and parses slide XML files
/// to extract text content from text boxes and shapes.
pub struct PptxParser;

impl PptxParser {
    /// Creates a new `PptxParser`.
    pub fn new() -> Self {
        Self
    }
}

impl Default for PptxParser {
    fn default() -> Self {
        Self::new()
    }
}

impl PptxParser {
    /// Extracts text from a single slide XML string.
    fn extract_text_from_slide_xml(xml: &str) -> String {
        let mut text = String::new();
        let mut in_text = false;

        // Simple state machine to extract text from <a:t> tags
        let mut chars = xml.char_indices().peekable();
        while let Some((i, c)) = chars.next() {
            match c {
                '<' => {
                    // Check for <a:t> or <a:t ...>
                    if xml[i..].starts_with("<a:t>") {
                        in_text = true;
                        // Skip past the opening tag
                        for _ in 0.."<a:t>".len() - 1 {
                            chars.next();
                        }
                    } else if xml[i..].starts_with("</a:t>") {
                        in_text = false;
                        for _ in 0.."</a:t>".len() - 1 {
                            chars.next();
                        }
                    }
                }
                _ if in_text => {
                    text.push(c);
                }
                _ => {}
            }
        }

        text
    }
}

impl DocumentParser for PptxParser {
    fn supported_extensions(&self) -> &[&str] {
        &["pptx", "pptm"]
    }

    fn parse(&self, bytes: &[u8], path: &Path) -> Result<ParseResult> {
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor).map_err(|e| {
            GlintIndexError::Other(format!(
                "PPTX ZIP extraction failed for {}: {}",
                path.display(),
                e
            ))
        })?;

        let mut content = String::new();

        // Find all slide XML files
        let slide_files: Vec<String> = (0..archive.len())
            .filter_map(|i| {
                archive.by_index(i).ok().and_then(|f| {
                    let name = f.name().to_string();
                    if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
                        Some(name)
                    } else {
                        None
                    }
                })
            })
            .collect();

        // Sort slides by number
        let mut slide_files = slide_files;
        slide_files.sort_by(|a, b| {
            let num_a = a
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse::<u32>()
                .unwrap_or(0);
            let num_b = b
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse::<u32>()
                .unwrap_or(0);
            num_a.cmp(&num_b)
        });

        for slide_file in &slide_files {
            let mut file = archive.by_name(slide_file).map_err(|e| {
                GlintIndexError::Other(format!(
                    "Failed to read slide {} from {}: {}",
                    slide_file,
                    path.display(),
                    e
                ))
            })?;

            let mut xml = String::new();
            file.read_to_string(&mut xml).map_err(|e| {
                GlintIndexError::Other(format!("Failed to read slide XML {}: {}", slide_file, e))
            })?;

            let slide_text = Self::extract_text_from_slide_xml(&xml);
            if !slide_text.trim().is_empty() {
                content.push_str(&slide_text);
                content.push('\n');
            }
        }

        if content.trim().is_empty() {
            return Err(GlintIndexError::Other(format!(
                "PPTX contains no extractable text: {}",
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
        let parser = PptxParser::new();
        assert!(parser.supported_extensions().contains(&"pptx"));
        assert!(parser.supported_extensions().contains(&"pptm"));
    }

    #[test]
    fn can_parse_pptx() {
        let parser = PptxParser::new();
        assert!(parser.can_parse(Path::new("presentation.pptx")));
        assert!(!parser.can_parse(Path::new("document.pdf")));
    }

    #[test]
    fn extract_text_from_slide() {
        let xml = r#"<p:sld><p:cSld><p:sp><p:txBody><a:p><a:r><a:t>Hello World</a:t></a:r></a:p></p:txBody></p:sp></p:cSld></p:sld>"#;
        let text = PptxParser::extract_text_from_slide_xml(xml);
        assert_eq!(text, "Hello World");
    }
}
