//! Common test helpers for GTK4 integration tests.
//!
//! All tests in this module use `#[gtk::test]` which handles
//! GTK initialization and main-thread execution automatically.

use gtk::glib;

mod preview_rendering;
mod search_flow;
mod settings_persistence;
mod theme_switching;

/// Initialize GTK and drain the GLib main context queue.
///
/// Call at the start of every test. Use `--test-threads=1` when
/// running GTK tests (GTK is single-threaded).
#[allow(dead_code)]
pub fn setup_gtk() {
    let _ = gtk::init();
    process_events();
}

/// Drain the GLib main context queue until empty.
pub fn process_events() {
    let ctx = glib::MainContext::default();
    while ctx.pending() {
        ctx.iteration(false);
    }
}

/// Create a temporary ApplicationService and WindowState with isolated state.
///
/// Returns the WindowState and a TempDir guard (keeps config alive).
pub fn setup_test_state() -> (crate::window::WindowState, tempfile::TempDir) {
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
    let service =
        glintindex_core::ApplicationService::with_config_path(&config_path).unwrap();

    let theme = service.config().theme;
    let font_size = service.config().clamped_font_size();

    let state = crate::window::WindowState {
        service,
        preview_service: glintindex_core::PreviewService::with_default_config(),
        query: String::new(),
        results: Vec::new(),
        selected_index: None,
        status: String::new(),
        preview_text: String::new(),
        progress_active: false,
        progress_message: String::new(),
        statistics: None,
        settings_window: None,
        preview_buffer: None,
        last_job_progress: None,
        theme_manager: crate::theme::ThemeManager::new(theme, font_size),
    };

    (state, tmp)
}

/// Create a temporary ApplicationService with isolated state.
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
    let service =
        glintindex_core::ApplicationService::with_config_path(&config_path).unwrap();
    (service, tmp)
}
