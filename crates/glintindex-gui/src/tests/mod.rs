//! Common test helpers for headless egui tests.
//!
//! [`run_ui`] drives [`GlintApp::ui`] through real `egui::Context`
//! frames with synthetic input — no window system or display needed.

use eframe::egui;

use crate::app::GlintApp;

mod preview_rendering;
mod search_flow;
mod settings_persistence;
mod theme_switching;

/// Creates a temporary ApplicationService with isolated state.
pub fn setup_test_service() -> (glintindex_core::ApplicationService, tempfile::TempDir) {
    let tmp = tempfile::TempDir::new().unwrap();
    let index_dir = tmp.path().join("index");
    std::fs::create_dir_all(&index_dir).unwrap();
    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"indexed_folders = []
ignored_folders = []
index_directory = "{}"
theme = "system"
max_preview_size = 200
commit_interval = 500
"#,
            index_dir.display()
        ),
    )
    .unwrap();
    let service = glintindex_core::ApplicationService::with_config_path(&config_path).unwrap();
    (service, tmp)
}

/// Creates a GlintApp backed by an isolated temp-dir service.
///
/// Returns the app and the TempDir guard (keeps config/index alive).
pub fn setup_test_app() -> (GlintApp, tempfile::TempDir) {
    let (service, tmp) = setup_test_service();
    (GlintApp::with_service(service), tmp)
}

/// Runs `n` headless frames of the app UI on a fresh egui context.
pub fn run_ui(app: &mut GlintApp, n: usize) {
    let ctx = egui::Context::default();
    for _ in 0..n {
        let out = ctx.run_ui(egui::RawInput::default(), |ui| app.ui(ui));
        out.drop_without_applying_deltas();
    }
}

/// Builds a standalone egui context for rendering-only checks.
pub fn test_context() -> egui::Context {
    egui::Context::default()
}
