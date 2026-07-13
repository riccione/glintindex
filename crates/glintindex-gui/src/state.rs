//! Application state for the GlintIndex GUI.
//!
//! Holds both the business logic service and the UI-specific state.
//! The GUI never accesses core internals directly — all interaction
//! goes through `ApplicationService`.
//!
//! # Preview Content
//!
//! The preview panel uses `text_editor::Content` (stored as
//! `preview_content`) instead of `text()` widgets because Iced's
//! `text()` does not support text selection. Only `text_editor()`
//! provides native text selection and OS-native copy shortcuts.
//! The `Content` must be stored in application state because
//! `text_editor()` borrows it by reference.

use iced::widget::text_editor;

use glintindex_core::{
    ApplicationService, ApplicationStatistics, IndexedFolder, PreviewOutput, Progress, SearchResult,
};

use crate::message::SettingsPage;

/// Snapshot of ignored folder names for display in the settings UI.
///
/// This is a cheap clone of the configuration's ignored_folders list,
/// kept in sync by refreshing after each mutation.
pub type IgnoredFolders = Vec<String>;

/// The complete application state for the GUI.
///
/// Holds both the business logic service and the UI-specific state.
/// The source of truth for configuration remains `ApplicationService`.
pub struct AppState {
    // ── Core ────────────────────────────────────────────────────
    /// The core application service providing search and indexing.
    pub service: ApplicationService,

    // ── Search ──────────────────────────────────────────────────
    /// The current search query text.
    pub query: String,
    /// The most recent search results.
    pub results: Vec<SearchResult>,
    /// Index of the currently selected result, if any.
    pub selected_index: Option<usize>,
    /// Status message displayed in the status bar.
    pub status: String,
    /// Whether a debounced search is pending.
    pub search_pending: bool,
    /// The query for which a debounced search is pending.
    pub pending_query: String,

    // ── Recent Searches ─────────────────────────────────────────
    /// Whether the recent searches dropdown is visible.
    pub recent_searches_open: bool,

    // ── Preview ─────────────────────────────────────────────────
    /// The current preview output, if any.
    pub current_preview: Option<PreviewOutput>,
    /// Whether a preview is currently loading.
    pub preview_loading: bool,
    /// Error message from the last preview operation, if any.
    pub preview_error: Option<String>,
    /// The search query currently highlighted in the preview.
    pub preview_search_query: String,

    // ── Settings ────────────────────────────────────────────────
    /// Whether the settings window is currently visible.
    pub settings_open: bool,
    /// The currently active settings page.
    pub settings_page: SettingsPage,
    /// Cached snapshot of configured indexed folders.
    pub indexed_folders: Vec<IndexedFolder>,
    /// Cached snapshot of ignored folder names.
    pub ignored_folders: IgnoredFolders,
    /// Cached application statistics.
    pub statistics: Option<ApplicationStatistics>,
    /// Status message for settings operations.
    pub settings_status: String,
    /// Whether a long-running operation is in progress.
    pub operation_in_progress: bool,
    /// Current progress information during indexing/rebuild operations.
    pub current_progress: Option<Progress>,
    /// Text editor content for the preview pane.
    ///
    /// Stored here because `text_editor()` borrows `Content` by
    /// reference, so it must outlive the view function. Updated
    /// whenever a new preview is loaded.
    pub preview_content: text_editor::Content,
}

impl AppState {
    /// Creates a new `AppState` with the given application service.
    ///
    /// Loads the initial configuration snapshot from the service.
    pub fn new(service: ApplicationService) -> Self {
        let status = Self::compute_status(&service);
        let indexed_folders = service.indexed_folders().into_iter().cloned().collect();
        let ignored_folders = service.ignored_folders().to_vec();
        let statistics = service.statistics().ok();

        Self {
            service,
            query: String::new(),
            results: Vec::new(),
            selected_index: None,
            status,
            search_pending: false,
            pending_query: String::new(),
            recent_searches_open: false,
            current_preview: None,
            preview_loading: false,
            preview_error: None,
            preview_search_query: String::new(),
            settings_open: false,
            settings_page: SettingsPage::General,
            indexed_folders,
            ignored_folders,
            statistics,
            settings_status: String::new(),
            operation_in_progress: false,
            current_progress: None,
            preview_content: text_editor::Content::default(),
        }
    }

    /// Returns the currently selected search result, if any.
    #[allow(dead_code)]
    pub fn selected_result(&self) -> Option<&SearchResult> {
        self.selected_index.and_then(|i| self.results.get(i))
    }

    /// Refreshes the cached configuration snapshot from the service.
    ///
    /// Call this after any configuration mutation to keep the UI in sync.
    pub fn refresh_config_snapshot(&mut self) {
        self.indexed_folders = self
            .service
            .indexed_folders()
            .into_iter()
            .cloned()
            .collect();
        self.ignored_folders = self.service.ignored_folders().to_vec();
    }

    /// Refreshes the cached statistics from the service.
    pub fn refresh_statistics(&mut self) {
        self.statistics = self.service.statistics().ok();
    }

    /// Updates the preview content for text selection support.
    ///
    /// Converts a [`PreviewOutput`] into a `text_editor::Content`
    /// that supports native text selection and OS-native copy.
    /// Includes line numbers and metadata notices (truncation,
    /// encoding) as comments at the top of the content.
    pub fn update_preview_content(&mut self, preview: &PreviewOutput) {
        let mut content = String::new();

        if preview.truncated {
            content.push_str(&format!(
                "// File truncated (showing first {} bytes)\n",
                preview.lines.len() * 50
            ));
        }

        if preview.encoding != glintindex_core::Encoding::Utf8 {
            content.push_str(&format!("// Encoding: {:?}\n", preview.encoding));
        }

        for line in &preview.lines {
            content.push_str(&format!("{:>4} {}\n", line.line_number, line.text));
        }

        self.preview_content = text_editor::Content::with_text(&content);
    }

    /// Returns the enabled folder count from the cached snapshot.
    pub fn enabled_folder_count(&self) -> usize {
        self.indexed_folders.iter().filter(|f| f.enabled).count()
    }

    /// Returns the recent searches from the service.
    pub fn recent_searches(&self) -> &[String] {
        self.service.recent_searches()
    }

    /// Computes a status message reflecting the current index state.
    fn compute_status(service: &ApplicationService) -> String {
        let folder_count = service.indexed_folders().len();
        if folder_count == 0 {
            "No folders configured".to_string()
        } else {
            format!(
                "Ready — {} folder{} configured",
                folder_count,
                if folder_count == 1 { "" } else { "s" }
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::application;
    use crate::message::Message;
    use glintindex_core::ApplicationService;
    use std::time::UNIX_EPOCH;

    fn create_test_state() -> AppState {
        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join("config.toml");
        let config = glintindex_core::AppConfig {
            index_directory: tmp.path().join("index"),
            ..Default::default()
        };
        glintindex_core::config::loader::save(&config_path, &config).unwrap();
        let service = ApplicationService::with_config_path(&config_path).unwrap();
        AppState::new(service)
    }

    fn create_test_result(filename: &str) -> glintindex_core::SearchResult {
        glintindex_core::SearchResult::new(
            glintindex_core::Document::new(
                std::path::PathBuf::from(filename),
                100,
                UNIX_EPOCH,
                "content".to_string(),
            ),
            1.0,
            "snippet".to_string(),
        )
    }

    #[test]
    fn navigate_up_empty_results() {
        let mut state = create_test_state();
        state.results = Vec::new();
        state.selected_index = None;

        let message = Message::NavigateUp;
        let _ = application::update(&mut state, message);
        assert!(state.selected_index.is_none());
    }

    #[test]
    fn navigate_down_empty_results() {
        let mut state = create_test_state();
        state.results = Vec::new();
        state.selected_index = None;

        let message = Message::NavigateDown;
        let _ = application::update(&mut state, message);
        assert!(state.selected_index.is_none());
    }

    #[test]
    fn navigate_up_no_selection() {
        let mut state = create_test_state();
        state.results = vec![create_test_result("file1.txt")];
        state.selected_index = None;

        let message = Message::NavigateUp;
        let _ = application::update(&mut state, message);
        assert_eq!(state.selected_index, Some(0));
    }

    #[test]
    fn navigate_down_no_selection() {
        let mut state = create_test_state();
        state.results = vec![create_test_result("file1.txt")];
        state.selected_index = None;

        let message = Message::NavigateDown;
        let _ = application::update(&mut state, message);
        assert_eq!(state.selected_index, Some(0));
    }

    #[test]
    fn navigate_up_at_top() {
        let mut state = create_test_state();
        state.results = vec![create_test_result("file1.txt")];
        state.selected_index = Some(0);

        let message = Message::NavigateUp;
        let _ = application::update(&mut state, message);
        assert_eq!(state.selected_index, Some(0));
    }

    #[test]
    fn navigate_down_at_bottom() {
        let mut state = create_test_state();
        state.results = vec![create_test_result("file1.txt")];
        state.selected_index = Some(0);

        let message = Message::NavigateDown;
        let _ = application::update(&mut state, message);
        assert_eq!(state.selected_index, Some(0));
    }

    #[test]
    fn navigate_up_middle() {
        let mut state = create_test_state();
        state.results = vec![
            create_test_result("file1.txt"),
            create_test_result("file2.txt"),
            create_test_result("file3.txt"),
        ];
        state.selected_index = Some(2);

        let message = Message::NavigateUp;
        let _ = application::update(&mut state, message);
        assert_eq!(state.selected_index, Some(1));
    }

    #[test]
    fn navigate_down_middle() {
        let mut state = create_test_state();
        state.results = vec![
            create_test_result("file1.txt"),
            create_test_result("file2.txt"),
            create_test_result("file3.txt"),
        ];
        state.selected_index = Some(0);

        let message = Message::NavigateDown;
        let _ = application::update(&mut state, message);
        assert_eq!(state.selected_index, Some(1));
    }

    #[test]
    fn recent_searches_initially_empty() {
        let state = create_test_state();
        assert!(state.recent_searches().is_empty());
    }
}
