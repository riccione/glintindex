//! Preview pane UI component.
//!
//! Displays file content in a read-only text view with search-match
//! highlights rendered via TextBuffer tags.
//! GTK4's TextView natively supports text selection and copying.

use gtk::prelude::*;
use gtk::{Box as GtkBox, Label, Orientation, TextBuffer, TextView};

/// Tag name for search-match highlight styling.
pub const MATCH_TAG: &str = "match-highlight";
/// Tag name for dimmed metadata/gap lines (headers, `…` separators).
pub const GAP_TAG: &str = "gap";
/// Placeholder text shown when no file is previewed.
pub const PLACEHOLDER_TEXT: &str = "Select a file to preview";

/// Builds the preview pane widget with a placeholder.
#[allow(dead_code)]
pub fn build() -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 0);

    let placeholder = Label::builder()
        .label("Select a file to preview")
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .vexpand(true)
        .css_classes(["dim-label"])
        .build();

    container.append(&placeholder);
    container
}

/// Builds the preview pane with a text buffer reference.
///
/// Returns the container widget and the text buffer for updating content.
/// The buffer has `match-highlight` and `gap` text tags registered.
pub fn build_with_buffer() -> (GtkBox, TextBuffer) {
    let container = GtkBox::new(Orientation::Vertical, 0);

    let text_view = TextView::builder()
        .editable(false)
        .monospace(true)
        .left_margin(4)
        .top_margin(4)
        .bottom_margin(4)
        .wrap_mode(gtk::WrapMode::None)
        .vexpand(true)
        .hexpand(true)
        .build();

    let buffer = text_view.buffer();
    create_preview_tags(&buffer);
    buffer.set_text(PLACEHOLDER_TEXT);

    container.append(&text_view);
    (container, buffer)
}

/// Resets the buffer to the startup placeholder text.
///
/// Used when the selection is lost (search input cleared, query
/// changed, results refreshed) so the preview pane never keeps
/// showing a previously selected document.
pub fn reset_to_placeholder(buffer: &TextBuffer) {
    buffer.set_text(PLACEHOLDER_TEXT);
}

/// Creates the highlight and gap text tags on a buffer's tag table.
fn create_preview_tags(buffer: &TextBuffer) {
    let tag_table = buffer.tag_table();

    if tag_table.lookup(MATCH_TAG).is_none() {
        // Warm yellow with alpha — readable on both light and dark themes.
        let background = gtk::gdk::RGBA::new(1.0, 0.84, 0.2, 0.55);
        // PANGO_WEIGHT_BOLD
        let weight = 700i32;
        buffer.create_tag(
            Some(MATCH_TAG),
            &[("weight", &weight), ("background-rgba", &background)],
        );
    }

    if tag_table.lookup(GAP_TAG).is_none() {
        let foreground = gtk::gdk::RGBA::new(0.5, 0.5, 0.5, 1.0);
        buffer.create_tag(
            Some(GAP_TAG),
            &[
                ("style", &gtk::pango::Style::Italic),
                ("foreground-rgba", &foreground),
            ],
        );
    }
}

/// Populates the text buffer from preview output.
///
/// Builds the toolkit-agnostic [`crate::preview_model`] then renders it
/// into the buffer: dim headers/gaps via the `gap` tag, match highlights
/// via the `match-highlight` tag (byte ranges converted to character
/// offsets). Returns the plain-text rendering of the buffer content.
pub fn populate_preview_buffer(
    buffer: &TextBuffer,
    output: &glintindex_core::PreviewOutput,
) -> String {
    let model = crate::preview_model::build_preview_model(output);
    populate_preview_buffer_from_model(buffer, &model)
}

/// Renders a [`crate::preview_model::PreviewModel`] into the text buffer.
fn populate_preview_buffer_from_model(
    buffer: &TextBuffer,
    model: &crate::preview_model::PreviewModel,
) -> String {
    create_preview_tags(buffer);
    buffer.set_text("");

    let mut plain = String::new();
    let mut offset: i32 = 0;

    for line in &model.lines {
        match line.style {
            crate::preview_model::LineStyle::Dim => {
                let text = format!("{}{}\n", line.number_prefix, line.text);
                offset = insert_styled(buffer, offset, &text, GAP_TAG);
                plain.push_str(&text);
            }
            crate::preview_model::LineStyle::Normal => {
                offset = insert_styled(buffer, offset, &line.number_prefix, "");
                plain.push_str(&line.number_prefix);

                let mut last_byte = 0;
                for range in &line.match_ranges {
                    let start = clamp_char_boundary(&line.text, range.start).max(last_byte);
                    let end = clamp_char_boundary(&line.text, range.end).max(start);
                    if start < last_byte {
                        continue;
                    }
                    let unstyled = &line.text[last_byte..start];
                    offset = insert_styled(buffer, offset, unstyled, "");
                    plain.push_str(unstyled);

                    let matched = &line.text[start..end];
                    offset = insert_styled(buffer, offset, matched, MATCH_TAG);
                    plain.push_str(matched);
                    last_byte = end;
                }
                let tail = &line.text[last_byte..];
                offset = insert_styled(buffer, offset, tail, "");
                offset = insert_styled(buffer, offset, "\n", "");
                plain.push_str(tail);
                plain.push('\n');
            }
        }
    }

    plain
}

/// Inserts text into the buffer at the given character offset, optionally
/// applying a named tag to the inserted range. Returns the new offset.
fn insert_styled(buffer: &TextBuffer, offset: i32, text: &str, tag: &str) -> i32 {
    let mut iter = buffer.iter_at_offset(offset);
    buffer.insert(&mut iter, text);
    let end_offset = offset + text.chars().count() as i32;
    if !tag.is_empty() {
        let start = buffer.iter_at_offset(offset);
        let end = buffer.iter_at_offset(end_offset);
        buffer.apply_tag_by_name(tag, &start, &end);
    }
    end_offset
}

/// Clamps a byte offset to the nearest valid char boundary at or below it.
fn clamp_char_boundary(text: &str, byte: usize) -> usize {
    let mut b = byte.min(text.len());
    while b > 0 && !text.is_char_boundary(b) {
        b -= 1;
    }
    b
}
