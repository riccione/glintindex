//! Preview rendering tests.
//!
//! Verifies the egui `LayoutJob` rendering of the preview model:
//! headers, gap markers, match highlights, and placeholder behavior.
//! The model itself is covered by `preview_model` unit tests.

use std::path::PathBuf;

use eframe::egui::text::LayoutSection;
use glintindex_core::{Encoding, HighlightedMatch, PreviewLine as CoreLine, PreviewOutput};

use crate::preview_model::{LineStyle, build_preview_model};
use crate::theme::MATCH_BACKGROUND;
use crate::ui::preview::{PLACEHOLDER_TEXT, to_layout_job};

use super::{run_ui, setup_test_app};

/// Extracts a section's slice of the job text (byte offsets via `ByteIndex`).
fn section_text<'a>(job: &'a eframe::egui::text::LayoutJob, section: &LayoutSection) -> &'a str {
    let start = usize::from(section.byte_range.start);
    let end = usize::from(section.byte_range.end);
    &job.text[start..end]
}

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
fn test_layout_job_contains_path_header_and_content() {
    let model = build_preview_model(&output(vec![line(1, "hello world", vec![])]));
    let visuals = eframe::egui::Visuals::light();
    let job = to_layout_job(&model, 14.0, visuals);

    assert!(job.text.contains("// File path: /test/file.txt"));
    assert!(job.text.contains("hello world"));
    assert!(job.text.contains("   1 "));
}

#[test]
fn test_layout_job_truncation_header() {
    let mut out = output(vec![line(1, "content", vec![])]);
    out.truncated = true;
    out.original_size = 4242;
    let model = build_preview_model(&out);
    let job = to_layout_job(&model, 14.0, eframe::egui::Visuals::light());
    assert!(job.text.contains("4242"));
    assert!(job.text.contains("// Preview truncated"));
}

#[test]
fn test_layout_job_match_ranges_get_highlight_background() {
    let model = build_preview_model(&output(vec![line(1, "hello world", vec![(6, 11)])]));
    let job = to_layout_job(&model, 14.0, eframe::egui::Visuals::light());

    let highlighted: Vec<&str> = job
        .sections
        .iter()
        .filter(|s| s.format.background == MATCH_BACKGROUND)
        .map(|s| section_text(&job, s))
        .collect();
    assert_eq!(highlighted, vec!["world"]);
}

#[test]
fn test_layout_job_dim_headers_use_weak_color() {
    let model = build_preview_model(&output(vec![]));
    let visuals = eframe::egui::Visuals::light();
    let job = to_layout_job(&model, 14.0, visuals.clone());

    let first = &job.sections[0];
    assert_eq!(first.format.color, visuals.weak_text_color());
    assert!(first.format.italics);
}

#[test]
fn test_layout_job_gap_marker_dim() {
    let model = build_preview_model(&output(vec![line(2, "a", vec![]), line(10, "b", vec![])]));
    assert!(model.lines.iter().any(|l| l.text == "…"));
    let visuals = eframe::egui::Visuals::light();
    let job = to_layout_job(&model, 14.0, visuals.clone());
    let gap_section = job
        .sections
        .iter()
        .find(|s| section_text(&job, s).trim_end() == "…")
        .expect("gap section present");
    assert_eq!(gap_section.format.color, visuals.weak_text_color());
}

#[test]
fn test_layout_job_multibyte_match_offsets() {
    // "héllo wörld": "wörld" at bytes 7..13.
    let model = build_preview_model(&output(vec![line(1, "héllo wörld", vec![(7, 13)])]));
    let job = to_layout_job(&model, 14.0, eframe::egui::Visuals::light());
    let highlighted: Vec<&str> = job
        .sections
        .iter()
        .filter(|s| s.format.background == MATCH_BACKGROUND)
        .map(|s| section_text(&job, s))
        .collect();
    assert_eq!(highlighted, vec!["wörld"]);
}

#[test]
fn test_no_selection_shows_placeholder_smoke() {
    let (mut app, _tmp) = setup_test_app();
    assert!(app.preview_model.is_none());
    run_ui(&mut app, 1);
    assert_eq!(PLACEHOLDER_TEXT, "Select a file to preview");
}

#[test]
fn test_selection_then_clear_resets_preview() {
    let (mut app, _tmp) = setup_test_app();
    let doc = glintindex_core::Document::new(
        PathBuf::from("/test/file.txt"),
        10,
        std::time::SystemTime::now(),
        "content".into(),
    );
    app.results
        .push(glintindex_core::SearchResult::new(doc, 1.0, "snip".into()));

    app.select_result(0);
    assert!(app.preview_model.is_some());

    app.clear_results();
    assert!(app.preview_model.is_none(), "cleared query resets preview");

    run_ui(&mut app, 1);
}

#[test]
fn test_metadata_only_preview_placeholder_line() {
    let (mut app, _tmp) = setup_test_app();
    let mut doc = glintindex_core::Document::new(
        PathBuf::from("/test/bin.dat"),
        10,
        std::time::SystemTime::now(),
        String::new(),
    );
    doc.is_metadata_only = true;
    app.results
        .push(glintindex_core::SearchResult::new(doc, 1.0, String::new()));

    app.select_result(0);
    let model = app.preview_model.as_ref().unwrap();
    assert!(
        model
            .lines
            .iter()
            .any(|l| l.text.contains("Preview unavailable")),
        "metadata-only files show a placeholder line"
    );
    assert_eq!(
        model.lines[0].style,
        LineStyle::Dim,
        "path header still first"
    );
}
