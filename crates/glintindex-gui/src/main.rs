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

#[cfg(test)]
mod tests;

// Re-export for #[gtk::test] macro support in binary crate
#[cfg(test)]
pub use gtk::test_synced;

use glintindex_core::config::loader;
use glintindex_core::logging::{LoggingConfig, init as init_logging};
use gtk::prelude::*;

fn main() {
    // Load config to get logging settings (fallback to defaults on error)
    let config_path = glintindex_core::AppPaths::new().config_file();
    let config = loader::load(&config_path).unwrap_or_default();

    // Resolution order:
    // 1. RUST_LOG env var (handled by EnvFilter::try_from_default_env)
    // 2. config.toml logging.level
    // 3. hardcoded "error"
    let log_to_stderr = std::env::var("RUST_LOG").is_ok();
    init_logging(LoggingConfig {
        default_level: config.logging.level.clone(),
        log_to_stderr,
        log_to_file: true,
        max_retention_days: config.logging.max_retention_days,
    });

    // Bridge log crate to tracing for any remaining log:: calls
    tracing_log::LogTracer::init().ok();

    let app = application::build_application();
    app.run();
}
