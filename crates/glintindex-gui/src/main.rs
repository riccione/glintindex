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
    // Set up global panic handler to capture crash information
    std::panic::set_hook(Box::new(|panic_info| {
        tracing::error!(target: "glintindex::panic", "PANIC OCCURRED");
        tracing::error!(target: "glintindex::panic", "{}", panic_info);

        if let Some(location) = panic_info.location() {
            tracing::error!(
                target: "glintindex::panic",
                file = location.file(),
                line = location.line(),
                column = location.column(),
                "Panic location"
            );
        }

        let backtrace = std::backtrace::Backtrace::force_capture();
        tracing::error!(
            target: "glintindex::panic",
            backtrace = %backtrace,
            "Backtrace"
        );
    }));

    // Initialize structured logging with file output
    // The GUI always logs to file; stderr is enabled for development
    let log_to_stderr = std::env::var("RUST_LOG").is_ok();
    init_logging(LoggingConfig {
        default_level: "debug".to_string(),
        log_to_stderr,
        log_to_file: true,
    });

    // Bridge log crate to tracing for any remaining log:: calls
    tracing_log::LogTracer::init().ok();

    // Log environment information
    tracing::info!(target: "glintindex::startup", "GlintIndex GUI starting");
    tracing::info!(
        target: "glintindex::startup",
        os = std::env::consts::OS,
        arch = std::env::consts::ARCH,
        family = std::env::consts::FAMILY,
        "Operating system information"
    );
    tracing::info!(
        target: "glintindex::startup",
        gtk_major = gtk::major_version(),
        gtk_minor = gtk::minor_version(),
        gtk_micro = gtk::micro_version(),
        "GTK version"
    );
    tracing::info!(
        target: "glintindex::startup",
        version = env!("CARGO_PKG_VERSION"),
        "Application version"
    );
    tracing::info!(
        target: "glintindex::startup",
        rust_version = env!("CARGO_PKG_RUST_VERSION"),
        "Rust version"
    );

    // Log the log file location for diagnostic purposes
    if let Some(log_dir) = glintindex_core::logging::log_dir() {
        tracing::info!(
            target: "glintindex::startup",
            log_directory = %log_dir.display(),
            log_file = %log_dir.join("glintindex.log").display(),
            "Log file location"
        );
    }

    let app = application::build_application();
    tracing::info!(target: "glintindex::startup", "GTK Application created, entering main loop");
    app.run();
}
