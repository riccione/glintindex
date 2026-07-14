//! Appearance settings page.
//!
//! Provides visual customization options including theme selection
//! and font size configuration. Settings are applied immediately
//! and persisted to the configuration file.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Box as GtkBox, CheckButton, Label, Orientation, Separator, SpinButton};

use crate::window::WindowState;

/// Minimum allowed font size.
const FONT_SIZE_MIN: f64 = 8.0;

/// Maximum allowed font size.
const FONT_SIZE_MAX: f64 = 32.0;

/// Font size step increment.
const FONT_SIZE_STEP: f64 = 1.0;

/// Builds the Appearance settings page.
pub fn build(state: &Rc<RefCell<WindowState>>, window: &gtk::Window) -> GtkBox {
    let content = GtkBox::new(Orientation::Vertical, 12);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);

    let title = Label::builder()
        .label("Appearance")
        .halign(gtk::Align::Start)
        .css_classes(["title-1"])
        .build();
    content.append(&title);
    content.append(&Separator::new(gtk::Orientation::Horizontal));

    // ── Theme ──────────────────────────────────────────────────
    let theme_label = Label::builder()
        .label("Theme")
        .halign(gtk::Align::Start)
        .css_classes(["title-2"])
        .build();
    content.append(&theme_label);

    let st = state.borrow();
    let current_theme = st.service.config().theme;

    let radio_system = CheckButton::with_label("System");
    let radio_light = CheckButton::with_label("Light");
    let radio_dark = CheckButton::with_label("Dark");

    // Group radio buttons for mutual exclusion
    // radio_system is the group leader (no set_group needed)
    radio_light.set_group(Some(&radio_system));
    radio_dark.set_group(Some(&radio_system));

    // Set initial active state
    match current_theme {
        glintindex_core::Theme::System => radio_system.set_active(true),
        glintindex_core::Theme::Light => radio_light.set_active(true),
        glintindex_core::Theme::Dark => radio_dark.set_active(true),
    }

    // Connect theme changes
    {
        let state_clone = state.clone();
        radio_system.connect_toggled(move |btn| {
            if btn.is_active() {
                let mut st = state_clone.borrow_mut();
                let _ = st.service.set_theme(glintindex_core::Theme::System);
            }
        });
    }
    {
        let state_clone = state.clone();
        radio_light.connect_toggled(move |btn| {
            if btn.is_active() {
                let mut st = state_clone.borrow_mut();
                let _ = st.service.set_theme(glintindex_core::Theme::Light);
            }
        });
    }
    {
        let state_clone = state.clone();
        radio_dark.connect_toggled(move |btn| {
            if btn.is_active() {
                let mut st = state_clone.borrow_mut();
                let _ = st.service.set_theme(glintindex_core::Theme::Dark);
            }
        });
    }

    let theme_row = GtkBox::new(Orientation::Vertical, 4);
    theme_row.append(&radio_system);
    theme_row.append(&radio_light);
    theme_row.append(&radio_dark);
    content.append(&theme_row);

    content.append(&Separator::new(gtk::Orientation::Horizontal));

    // ── Font Size ──────────────────────────────────────────────
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
