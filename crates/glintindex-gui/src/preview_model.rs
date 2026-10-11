//! Toolkit-agnostic preview document model.
//!
//! [`build_preview_model`] turns a [`PreviewOutput`] into a flat list of
//! styled lines (path/truncation/encoding headers, `…` gap markers, and
//! content lines with line-number prefixes and byte-offset match ranges).
//! UI toolkits render the model; tests exercise it directly with no
//! widget dependencies.

use std::ops::Range;

/// Content kind for a rendered preview line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineStyle {
    /// File content line with a line-number prefix.
    Normal,
    /// Dimmed metadata: `//` headers and `…` gap markers.
    Dim,
}

/// One rendered line of the preview pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewLine {
    /// Line-number prefix (e.g. `"   2 "`); empty for dim header lines.
    pub number_prefix: String,
    /// Line content without the prefix or trailing newline.
    pub text: String,
    pub style: LineStyle,
    /// Search-match ranges as **byte offsets** into `text` (the same
    /// convention as [`glintindex_core::HighlightedMatch`]). Renderers
    /// convert to their own coordinate space.
    pub match_ranges: Vec<Range<usize>>,
}

/// Fully rendered preview document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreviewModel {
    pub lines: Vec<PreviewLine>,
}

impl PreviewModel {
    /// Joins all lines (prefix + text) with newlines — the plain-text
    /// form of the preview pane.
    #[cfg(test)]
    pub fn to_plain_text(&self) -> String {
        let mut plain = String::new();
        for line in &self.lines {
            plain.push_str(&line.number_prefix);
            plain.push_str(&line.text);
            plain.push('\n');
        }
        plain
    }

    fn dim(text: impl Into<String>) -> PreviewLine {
        PreviewLine {
            number_prefix: String::new(),
            text: text.into(),
            style: LineStyle::Dim,
            match_ranges: Vec::new(),
        }
    }
}

/// Builds the preview document model from preview output.
///
/// Emits, in order: dim `// File path: …` header, optional dim
/// truncation/encoding notices, then content lines with right-aligned
/// 4-wide line-number prefixes and `…` dim gap markers between
/// non-contiguous line blocks.
pub fn build_preview_model(output: &glintindex_core::PreviewOutput) -> PreviewModel {
    let mut lines = Vec::new();

    // Full file path header — the results list shows only the file
    // name, so the path lives here as a dimmed `//` comment.
    lines.push(PreviewModel::dim(format!(
        "// File path: {}",
        output.path.display()
    )));

    if output.truncated {
        lines.push(PreviewModel::dim(format!(
            "// Preview truncated (original size: {} bytes)",
            output.original_size
        )));
    }

    if output.encoding != glintindex_core::Encoding::Utf8 {
        lines.push(PreviewModel::dim(format!(
            "// Encoding: {:?}",
            output.encoding
        )));
    }

    let mut previous_line: Option<usize> = None;
    for line in &output.lines {
        if let Some(prev) = previous_line {
            if line.line_number > prev + 1 {
                lines.push(PreviewModel::dim("…"));
            }
        }
        previous_line = Some(line.line_number);

        lines.push(PreviewLine {
            number_prefix: format!("{:>4} ", line.line_number),
            text: line.text.clone(),
            style: LineStyle::Normal,
            match_ranges: line
                .match_highlights
                .iter()
                .map(|m| m.start..m.end)
                .collect(),
        });
    }

    PreviewModel { lines }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;

    use glintindex_core::{Encoding, HighlightedMatch, PreviewLine as CoreLine, PreviewOutput};

    fn output(lines: Vec<CoreLine>) -> PreviewOutput {
        PreviewOutput {
            path: PathBuf::from("/test/file.txt"),
            lines,
            truncated: false,
            encoding: Encoding::Utf8,
            is_binary: false,
            error: None,
            original_size: 100,
        }
    }

    fn line(number: usize, text: &str, matches: Vec<(usize, usize)>) -> CoreLine {
        CoreLine {
            line_number: number,
            text: text.to_string(),
            syntax_spans: vec![],
            match_highlights: matches
                .into_iter()
                .map(|(start, end)| HighlightedMatch {
                    start,
                    end,
                    text: text[start..end].to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn path_header_is_first_line_and_dim() {
        let model = build_preview_model(&output(vec![]));
        assert_eq!(model.lines.len(), 1);
        assert_eq!(model.lines[0].text, "// File path: /test/file.txt");
        assert_eq!(model.lines[0].style, LineStyle::Dim);
        assert!(model.lines[0].number_prefix.is_empty());
    }

    #[test]
    fn truncation_and_encoding_headers_follow_path_header() {
        let mut out = output(vec![]);
        out.truncated = true;
        out.original_size = 12345;
        out.encoding = Encoding::Utf16Le;
        let model = build_preview_model(&out);

        assert_eq!(model.lines[0].text, "// File path: /test/file.txt");
        assert!(
            model.lines[1].text.contains("12345"),
            "truncation notice second"
        );
        assert!(
            model.lines[2].text.starts_with("// Encoding:"),
            "encoding notice third"
        );
        assert!(model.lines[1..].iter().all(|l| l.style == LineStyle::Dim));
    }

    #[test]
    fn content_lines_have_number_prefixes_and_match_ranges() {
        let model = build_preview_model(&output(vec![
            line(2, "alpha", vec![]),
            line(3, "hello world", vec![(6, 11)]),
        ]));

        assert_eq!(model.lines[1].number_prefix, "   2 ");
        assert_eq!(model.lines[1].text, "alpha");

        let l3 = &model.lines[2];
        assert_eq!(l3.number_prefix, "   3 ");
        assert_eq!(l3.style, LineStyle::Normal);
        assert_eq!(l3.match_ranges, vec![6..11]);
    }

    #[test]
    fn gap_marker_inserted_between_non_contiguous_blocks() {
        let model = build_preview_model(&output(vec![
            line(2, "alpha", vec![]),
            line(3, "beta", vec![]),
            line(10, "gamma", vec![]),
        ]));

        let texts: Vec<&str> = model.lines.iter().map(|l| l.text.as_str()).collect();
        // path header, 2, 3, … gap, 10
        assert_eq!(
            texts,
            vec![
                "// File path: /test/file.txt",
                "alpha",
                "beta",
                "…",
                "gamma"
            ]
        );
        let gap_at = texts.iter().position(|t| *t == "…").unwrap();
        assert_eq!(model.lines[gap_at].style, LineStyle::Dim);
    }

    #[test]
    fn no_gap_between_contiguous_lines() {
        let model = build_preview_model(&output(vec![line(1, "a", vec![]), line(2, "b", vec![])]));
        assert!(model.lines.iter().all(|l| l.text != "…"));
    }

    #[test]
    fn to_plain_text_joins_prefixes_and_lines() {
        let model = build_preview_model(&output(vec![line(7, "content", vec![])]));
        assert_eq!(
            model.to_plain_text(),
            "// File path: /test/file.txt\n   7 content\n"
        );
    }

    #[test]
    fn multibyte_match_ranges_are_preserved_as_byte_offsets() {
        // "héllo wörld": "wörld" starts at byte 7.
        let model = build_preview_model(&output(vec![line(1, "héllo wörld", vec![(7, 13)])]));
        assert_eq!(model.lines[1].match_ranges, vec![7..13]);
    }
}
