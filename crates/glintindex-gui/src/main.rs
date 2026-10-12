#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! GlintIndex GUI — eframe/egui desktop search interface.
//!
//! This is the entry point for the graphical user interface.
//! It uses eframe (egui) to create a native desktop application with
//! live search capabilities.

mod app;
mod file_actions;
mod preview_job;
mod preview_model;
mod search;
mod theme;
mod ui;

#[cfg(test)]
mod tests;

use eframe::egui;
use glintindex_core::config::loader;
use glintindex_core::logging::{LoggingConfig, init as init_logging};

fn main() -> eframe::Result {
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

    // Bridge log crate to tracing for any remaining log::calls
    tracing_log::LogTracer::init().ok();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("GlintIndex")
            .with_inner_size([1000.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native(
        "GlintIndex",
        options,
        Box::new(|cc| Ok(Box::new(app::GlintApp::new(cc)))),
    )
}
