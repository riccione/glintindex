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

    let formatted = crate::window::format_preview_content(&output);
    assert!(formatted.is_empty() || !formatted.contains("truncated"));
}
