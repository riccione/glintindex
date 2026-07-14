//! About settings page.
//!
//! Displays application metadata: name, version, description,
//! Rust version, and license.

use gtk::prelude::*;
use gtk::{Box as GtkBox, Label, Orientation, Separator};

/// Application version from Cargo.toml at compile time.
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application name from Cargo.toml at compile time.
const APP_NAME: &str = env!("CARGO_PKG_NAME");

/// Application description from Cargo.toml at compile time.
const APP_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

/// Rust compiler version at compile time.
const RUST_VERSION: &str = env!("CARGO_PKG_RUST_VERSION");

/// Application license from Cargo.toml at compile time.
const APP_LICENSE: &str = env!("CARGO_PKG_LICENSE");

/// Builds the About settings page.
pub fn build() -> GtkBox {
    let content = GtkBox::new(Orientation::Vertical, 12);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);

    let title = Label::builder()
        .label("About")
        .halign(gtk::Align::Start)
        .css_classes(["title-1"])
        .build();
    content.append(&title);
    content.append(&Separator::new(gtk::Orientation::Horizontal));

    content.append(&info_row("Application", APP_NAME));
    content.append(&info_row("Version", APP_VERSION));
    content.append(&info_row("Description", APP_DESCRIPTION));
    content.append(&info_row("Rust Version", RUST_VERSION));
    content.append(&info_row("License", APP_LICENSE));

    content
}

/// Creates a label-value row for displaying information.
fn info_row(label: &str, value: &str) -> GtkBox {
    let row = GtkBox::new(Orientation::Vertical, 2);

    let label_widget = Label::builder()
        .label(label)
        .halign(gtk::Align::Start)
        .css_classes(["dim-label", "caption"])
        .build();

    let value_widget = Label::builder()
        .label(value)
        .halign(gtk::Align::Start)
        .wrap(true)
        .selectable(true)
        .build();

    row.append(&label_widget);
    row.append(&value_widget);
    row
}
