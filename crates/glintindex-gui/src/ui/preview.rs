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
/// Renders line numbers and text, applies the `match-highlight` tag over
/// each search match (byte offsets converted to character offsets for
/// `TextBuffer`), inserts dim `…` gap markers between non-contiguous
/// line blocks, and dim header lines for truncation/encoding notices.
/// Returns the plain-text rendering of the buffer content.
pub fn populate_preview_buffer(
    buffer: &TextBuffer,
    output: &glintindex_core::PreviewOutput,
) -> String {
    create_preview_tags(buffer);
    buffer.set_text("");

    let mut plain = String::new();
    let mut offset: i32 = 0;

    if output.truncated {
        let header = format!(
            "// Preview truncated (original size: {} bytes)\n",
            output.original_size
        );
        offset = insert_styled(buffer, offset, &header, GAP_TAG);
        plain.push_str(&header);
    }

    if output.encoding != glintindex_core::Encoding::Utf8 {
        let header = format!("// Encoding: {:?}\n", output.encoding);
        offset = insert_styled(buffer, offset, &header, GAP_TAG);
        plain.push_str(&header);
    }

    let mut previous_line: Option<usize> = None;
    for line in &output.lines {
        if let Some(prev) = previous_line {
            if line.line_number > prev + 1 {
                let gap = "…\n".to_string();
                offset = insert_styled(buffer, offset, &gap, GAP_TAG);
                plain.push_str(&gap);
            }
        }
        previous_line = Some(line.line_number);

        let prefix = format!("{:>4} ", line.line_number);
        offset = insert_styled(buffer, offset, &prefix, "");
        plain.push_str(&prefix);

        let text_start = offset;
        offset = insert_styled(buffer, offset, &line.text, "");
        offset = insert_styled(buffer, offset, "\n", "");
        plain.push_str(&line.text);
        plain.push('\n');

        for m in &line.match_highlights {
            let char_start = byte_to_char_offset(&line.text, m.start);
            let char_end = byte_to_char_offset(&line.text, m.end);
            let start = buffer.iter_at_offset(text_start + char_start as i32);
            let end = buffer.iter_at_offset(text_start + char_end as i32);
            buffer.apply_tag_by_name(MATCH_TAG, &start, &end);
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

/// Converts a byte offset within `text` to a character offset,
/// clamping to the nearest char boundary if the offset is invalid.
fn byte_to_char_offset(text: &str, byte: usize) -> usize {
    let mut b = byte.min(text.len());
    while b > 0 && !text.is_char_boundary(b) {
        b -= 1;
    }
    text[..b].chars().count()
}
