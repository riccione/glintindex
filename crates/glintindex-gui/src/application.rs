//! GTK Application setup and lifecycle.
//!
//! Creates and configures the GTK4 Application, builds the main window,
//! and manages the application lifecycle.

use gtk::Application;
use gtk::prelude::*;

use crate::window::GlintIndexWindow;

/// Application ID for D-Bus and window matching.
const APP_ID: &str = "com.github.glintindex.gui";

/// Creates and configures the GTK4 Application.
pub fn build_application() -> Application {
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(|app| {
        let window = GlintIndexWindow::new(app);
        window.present();
    });

    app
}
