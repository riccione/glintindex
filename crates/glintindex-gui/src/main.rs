#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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

use glintindex_core::logging::{LoggingConfig, init as init_logging};
use gtk::prelude::*;

fn main() {
    // Initialize structured logging with file output
    // The GUI always logs to file; stderr is enabled for development
    let log_to_stderr = std::env::var("RUST_LOG").is_ok();
    init_logging(LoggingConfig {
        default_level: "info".to_string(),
        log_to_stderr,
        log_to_file: true,
    });

    // Bridge log crate to tracing for any remaining log:: calls
    tracing_log::LogTracer::init().ok();

    let app = application::build_application();
    app.run();
}
