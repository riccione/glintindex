//! GlintIndex GUI — GTK4-based desktop search interface.
//!
//! This is the entry point for the graphical user interface.
//! It uses GTK4 to create a native desktop application with
//! live search capabilities.

mod application;
mod file_actions;
mod theme;
mod ui;
mod window;

use gtk::prelude::*;

fn main() {
    env_logger::init();

    let app = application::build_application();
    app.run();
}
