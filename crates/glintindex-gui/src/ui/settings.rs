//! Settings UI component.
//!
//! Provides a settings dialog for managing indexed folders,
//! ignored folders, and application configuration.

use gtk::Window;
use gtk::prelude::*;

/// Shows the settings window.
#[allow(dead_code)]
pub fn show_settings(parent: &Window) {
    let settings_window = Window::builder()
        .title("Settings")
        .default_width(500)
        .default_height(400)
        .modal(true)
        .transient_for(parent)
        .build();

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);

    let label = gtk::Label::builder()
        .label("Settings coming soon...")
        .build();

    content.append(&label);
    settings_window.set_child(Some(&content));

    settings_window.present();
}
