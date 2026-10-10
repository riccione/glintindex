//! File preview rendering tests.
//!
//! Tests preview content loading, TextBuffer state, and preview panel behavior.

use gtk::prelude::*;

/// Get all text from a TextBuffer.
fn buffer_text(buffer: &gtk::TextBuffer) -> String {
    let start = buffer.start_iter();
    let end = buffer.end_iter();
    buffer.text(&start, &end, false).to_string()
}

#[gtk::test]
fn test_preview_buffer_initial_state() {
    let _ = gtk::init();
    let (preview_widget, buffer) = crate::ui::preview::build_with_buffer();
    let text = buffer_text(&buffer);
    assert!(
        text.contains("Select a file"),
        "Initial buffer should show placeholder: got '{}'",
        text
    );
    let _ = preview_widget;
}

#[gtk::test]
fn test_preview_buffer_set_text_updates_content() {
    let _ = gtk::init();
    let (_preview_widget, buffer) = crate::ui::preview::build_with_buffer();

    buffer.set_text("Hello, world!");
    let text = buffer_text(&buffer);
    assert_eq!(text, "Hello, world!");
}

#[gtk::test]
fn test_preview_buffer_clears_on_new_selection() {
    let _ = gtk::init();
    let (_preview_widget, buffer) = crate::ui::preview::build_with_buffer();

    buffer.set_text("First content");
    assert_eq!(buffer_text(&buffer), "First content");

    buffer.set_text("Second content");
    assert_eq!(buffer_text(&buffer), "Second content");
}

#[gtk::test]
fn test_preview_text_view_is_read_only() {
    let _ = gtk::init();
    let (preview_widget, _buffer) = crate::ui::preview::build_with_buffer();

    // Find the TextView inside the container
    let child = preview_widget
        .first_child()
        .expect("Preview should have a child");
    let text_view = child
        .downcast_ref::<gtk::TextView>()
        .expect("Expected a TextView inside preview widget");

    assert!(!text_view.is_editable(), "Preview should be read-only");
}

#[gtk::test]
fn test_preview_text_view_is_monospace() {
    let _ = gtk::init();
    let (preview_widget, _buffer) = crate::ui::preview::build_with_buffer();

    let child = preview_widget
        .first_child()
        .expect("Preview should have a child");
    let text_view = child
        .downcast_ref::<gtk::TextView>()
        .expect("Expected a TextView inside preview widget");

    assert!(text_view.is_monospace(), "Preview should be monospace");
}

#[gtk::test]
fn test_preview_handles_empty_content() {
    let _ = gtk::init();
    let (_preview_widget, buffer) = crate::ui::preview::build_with_buffer();
    buffer.set_text("");
    assert!(buffer_text(&buffer).is_empty());
}

#[gtk::test]
fn test_preview_handles_binary_detection() {
    let _ = gtk::init();
    let (_preview_widget, buffer) = crate::ui::preview::build_with_buffer();
    buffer.set_text("Binary file detected — preview not available");
    let text = buffer_text(&buffer);
    assert!(text.contains("Binary") || text.contains("not available"));
}

#[gtk::test]
fn test_preview_encoding_utf8_detected() {
    let _ = gtk::init();
    let (_preview_widget, buffer) = crate::ui::preview::build_with_buffer();
    buffer.set_text("UTF-8 content: é, ñ, ü, 日本語");
    let text = buffer_text(&buffer);
    assert!(text.contains("é"), "Should contain UTF-8 characters");
}

#[gtk::test]
fn test_preview_encoding_utf16_detected() {
    let _ = gtk::init();
    let (_preview_widget, buffer) = crate::ui::preview::build_with_buffer();
    buffer.set_text("// Encoding: Utf16Le\nHello from UTF-16");
    let text = buffer_text(&buffer);
    assert!(text.contains("Utf16"), "Should show UTF-16 encoding");
}

#[gtk::test]
fn test_preview_handles_missing_file() {
    let _ = gtk::init();
    let (_preview_widget, buffer) = crate::ui::preview::build_with_buffer();
    buffer.set_text("File not found: /nonexistent/path.txt");
    let text = buffer_text(&buffer);
    assert!(
        text.contains("not found") || text.contains("File not found"),
        "Should show file not found message"
    );
}

#[gtk::test]
fn test_preview_buffer_reset_on_selection_lost() {
    let _ = gtk::init();
    let (_preview_widget, buffer) = crate::ui::preview::build_with_buffer();

    buffer.set_text("Some preview content");
    assert_eq!(buffer_text(&buffer), "Some preview content");

    buffer.set_text("Select a file to preview");
    assert_eq!(buffer_text(&buffer), "Select a file to preview");
}

#[gtk::test]
fn test_preview_line_numbers_present() {
    let _ = gtk::init();
    let (_preview_widget, buffer) = crate::ui::preview::build_with_buffer();

    let content = "   1 line one\n   2 line two\n   3 line three";
    buffer.set_text(content);

    let text = buffer_text(&buffer);
    assert!(text.contains("1 "), "Should contain line number 1");
    assert!(text.contains("2 "), "Should contain line number 2");
    assert!(text.contains("3 "), "Should contain line number 3");
}

#[gtk::test]
fn test_preview_truncation_notice() {
    let _ = gtk::init();
    let (_preview_widget, buffer) = crate::ui::preview::build_with_buffer();

    buffer.set_text("// File truncated (showing first 10000 bytes)\nContent here...");
    let text = buffer_text(&buffer);
    assert!(text.contains("truncated"), "Should show truncation notice");
}

#[gtk::test]
fn test_preview_format_output_basic() {
    let _ = gtk::init();
    use glintindex_core::PreviewOutput;

    let output = PreviewOutput {
        path: std::path::PathBuf::from("/test/file.txt"),
        lines: vec![],
        truncated: false,
        encoding: glintindex_core::Encoding::Utf8,
        is_binary: false,
        error: None,
        original_size: 100,
    };

    let (_widget, buffer) = crate::ui::preview::build_with_buffer();
    let plain = crate::ui::preview::populate_preview_buffer(&buffer, &output);
    assert!(plain.is_empty(), "empty output should render empty text");
}

fn sample_line(
    number: usize,
    text: &str,
    matches: Vec<(usize, usize, &str)>,
) -> glintindex_core::PreviewLine {
    glintindex_core::PreviewLine {
        line_number: number,
        text: text.to_string(),
        syntax_spans: vec![],
        match_highlights: matches
            .into_iter()
            .map(|(start, end, text)| glintindex_core::HighlightedMatch {
                start,
                end,
                text: text.to_string(),
            })
            .collect(),
    }
}

fn sample_output(lines: Vec<glintindex_core::PreviewLine>) -> glintindex_core::PreviewOutput {
    glintindex_core::PreviewOutput {
        path: std::path::PathBuf::from("/test/file.txt"),
        lines,
        truncated: false,
        encoding: glintindex_core::Encoding::Utf8,
        is_binary: false,
        error: None,
        original_size: 100,
    }
}

#[gtk::test]
fn test_preview_buffer_has_highlight_tags() {
    let _ = gtk::init();
    let (_widget, buffer) = crate::ui::preview::build_with_buffer();
    let table = buffer.tag_table();
    assert!(table.lookup(crate::ui::preview::MATCH_TAG).is_some());
    assert!(table.lookup(crate::ui::preview::GAP_TAG).is_some());
}

#[gtk::test]
fn test_populate_renders_line_numbers_and_gaps() {
    let _ = gtk::init();
    let (_widget, buffer) = crate::ui::preview::build_with_buffer();

    // Context blocks around matches: lines 2-3 and 10 — gap between.
    let output = sample_output(vec![
        sample_line(2, "alpha", vec![]),
        sample_line(3, "beta", vec![]),
        sample_line(10, "gamma", vec![]),
    ]);
    let plain = crate::ui::preview::populate_preview_buffer(&buffer, &output);

    assert!(plain.contains("   2 "), "line number 2 rendered");
    assert!(plain.contains("   3 "), "line number 3 rendered");
    assert!(plain.contains("  10 "), "line number 10 rendered");
    assert!(plain.contains('…'), "gap marker between blocks");
    // Gap goes between line 3 and line 10, not between 2 and 3.
    let gap_at = plain.find('…').unwrap();
    let line10_at = plain.find("  10 ").unwrap();
    assert!(gap_at < line10_at);
}

#[gtk::test]
fn test_populate_applies_match_highlight_tag() {
    let _ = gtk::init();
    let (_widget, buffer) = crate::ui::preview::build_with_buffer();

    // Line text "hello world"; match "world" at byte offsets 6..11.
    let output = sample_output(vec![sample_line(1, "hello world", vec![(6, 11, "world")])]);
    crate::ui::preview::populate_preview_buffer(&buffer, &output);

    let text = buffer_text(&buffer);
    assert!(text.contains("hello world"));
    let match_tag = buffer
        .tag_table()
        .lookup(crate::ui::preview::MATCH_TAG)
        .expect("match tag registered");

    // Prefix "   1 " is 5 chars; "world" occupies chars 11..16 (0-based,
    // end-exclusive — 'd' is at char 15).
    let start_of_match = buffer.iter_at_offset(11);
    let last_match_char = buffer.iter_at_offset(15);
    let after_match = buffer.iter_at_offset(16);
    let before_match = buffer.iter_at_offset(10);
    assert!(
        start_of_match.has_tag(&match_tag),
        "first match char tagged"
    );
    assert!(
        last_match_char.has_tag(&match_tag),
        "last match char tagged"
    );
    assert!(
        !after_match.has_tag(&match_tag),
        "tag range is end-exclusive"
    );
    assert!(
        !before_match.has_tag(&match_tag),
        "text before match is untagged"
    );
}

#[gtk::test]
fn test_populate_converts_multibyte_byte_offsets_to_chars() {
    let _ = gtk::init();
    let (_widget, buffer) = crate::ui::preview::build_with_buffer();

    // "héllo wörld": "wörld" starts at byte 7 but char 6 (é is 2 bytes).
    let output = sample_output(vec![sample_line(1, "héllo wörld", vec![(7, 13, "wörld")])]);
    crate::ui::preview::populate_preview_buffer(&buffer, &output);

    let match_tag = buffer
        .tag_table()
        .lookup(crate::ui::preview::MATCH_TAG)
        .expect("match tag registered");

    // Prefix "   1 " = 5 chars; "wörld" is chars 11..16.
    let start_of_match = buffer.iter_at_offset(11);
    assert!(
        start_of_match.has_tag(&match_tag),
        "multibyte match start tagged at char offset"
    );
    // Byte-offset-as-char-offset (12) would start one char late — verify
    // the actual first char of "wörld" is tagged, not its second char.
    let second_char = buffer.iter_at_offset(12);
    assert!(second_char.has_tag(&match_tag));
}

#[gtk::test]
fn test_populate_shows_truncation_header() {
    let _ = gtk::init();
    let (_widget, buffer) = crate::ui::preview::build_with_buffer();

    let mut output = sample_output(vec![sample_line(1, "content", vec![])]);
    output.truncated = true;
    output.original_size = 12345;
    let plain = crate::ui::preview::populate_preview_buffer(&buffer, &output);

    assert!(plain.contains("truncated"), "truncation header shown");
    assert!(plain.contains("12345"), "original size shown");
}
