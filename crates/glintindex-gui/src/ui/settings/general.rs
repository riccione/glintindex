//! General settings page.
//!
//! Displays read-only application information: name, version,
//! config directory, and index directory.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Box as GtkBox, Label, Orientation, Separator};

use crate::window::WindowState;

/// Application version from Cargo.toml at compile time.
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application name from Cargo.toml at compile time.
const APP_NAME: &str = env!("CARGO_PKG_NAME");

/// Builds the General settings page.
pub fn build(state: &Rc<RefCell<WindowState>>) -> GtkBox {
    let content = GtkBox::new(Orientation::Vertical, 12);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);

    let title = Label::builder()
        .label("General")
        .halign(gtk::Align::Start)
        .css_classes(["title-1"])
        .build();
    content.append(&title);
    content.append(&Separator::new(gtk::Orientation::Horizontal));

    let st = state.borrow();
    let config_display = st
        .service
        .config()
        .index_directory
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(not available)".to_string());

    let index_display = st.service.index_path().display().to_string();

    content.append(&info_row("Application", APP_NAME));
    content.append(&info_row("Version", APP_VERSION));
    content.append(&info_row("Config Directory", &config_display));
    content.append(&info_row("Index Directory", &index_display));

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
