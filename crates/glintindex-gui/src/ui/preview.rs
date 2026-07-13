//! Preview pane UI component.
//!
//! Displays file content in a read-only text view.
//! GTK4's TextView natively supports text selection and copying.

use gtk::prelude::*;
use gtk::{Box as GtkBox, Label, Orientation, TextBuffer, TextView};

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
    buffer.set_text("Select a file to preview");

    container.append(&text_view);
    (container, buffer)
}
