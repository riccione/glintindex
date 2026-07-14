//! General settings page.
//!
//! Displays read-only application information: name, version,
//! config directory, and index directory.
//! Also provides a font size setting with immediate application.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Box as GtkBox, Label, Orientation, Separator, SpinButton};

use crate::window::WindowState;

/// Application version from Cargo.toml at compile time.
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application name from Cargo.toml at compile time.
const APP_NAME: &str = env!("CARGO_PKG_NAME");

/// Minimum allowed font size.
const FONT_SIZE_MIN: f64 = 8.0;

/// Maximum allowed font size.
const FONT_SIZE_MAX: f64 = 32.0;

/// Font size step increment.
const FONT_SIZE_STEP: f64 = 1.0;

/// Builds the General settings page.
pub fn build(state: &Rc<RefCell<WindowState>>, window: &gtk::Window) -> GtkBox {
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

    // Font size setting
    content.append(&Separator::new(gtk::Orientation::Horizontal));

    let font_size_label = Label::builder()
        .label("Font Size")
        .halign(gtk::Align::Start)
        .css_classes(["title-2"])
        .build();
    content.append(&font_size_label);

    let current_font_size = st.service.config().clamped_font_size() as f64;

    let font_size_spin = SpinButton::builder()
        .adjustment(&gtk::Adjustment::new(
            current_font_size,
            FONT_SIZE_MIN,
            FONT_SIZE_MAX,
            FONT_SIZE_STEP,
            FONT_SIZE_STEP * 5.0,
            0.0,
        ))
        .halign(gtk::Align::Start)
        .build();

    // Apply font size immediately on change
    {
        let state_clone = state.clone();
        let window_clone = window.clone();
        font_size_spin.connect_value_changed(move |spin_button| {
            let new_size = spin_button.value() as u32;
            let mut st = state_clone.borrow_mut();
            let _ = st.service.set_font_size(new_size);
            drop(st);
            apply_font_size(&window_clone, new_size);
        });
    }

    let font_size_row = GtkBox::new(Orientation::Horizontal, 8);
    font_size_row.append(
        &Label::builder()
            .label("Application font size (8–32)")
            .hexpand(true)
            .halign(gtk::Align::Start)
            .css_classes(["dim-label"])
            .build(),
    );
    font_size_row.append(&font_size_spin);

    content.append(&font_size_row);

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

/// Applies the given font size to the application window using GTK4 CSS.
pub fn apply_font_size(window: &gtk::Window, font_size: u32) {
    let css = format!("* {{ font-size: {}pt; }}", font_size);

    let provider = gtk::CssProvider::new();
    provider.load_from_data(&css);

    gtk::style_context_add_provider_for_display(
        &gtk::prelude::WidgetExt::display(window),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
