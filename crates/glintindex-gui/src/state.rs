//! Application state for the GlintIndex GUI.
//!
//! Holds both the business logic service and the UI-specific state.
//! The GUI never accesses core internals directly — all interaction
//! goes through `ApplicationService`.

use glintindex_core::{ApplicationService, ApplicationStatistics, IndexedFolder, SearchResult};

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
            settings_open: false,
            settings_page: SettingsPage::General,
            indexed_folders,
            ignored_folders,
            statistics,
            settings_status: String::new(),
            operation_in_progress: false,
        }
    }

    /// Returns the currently selected search result, if any.
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

    /// Returns the enabled folder count from the cached snapshot.
    pub fn enabled_folder_count(&self) -> usize {
        self.indexed_folders.iter().filter(|f| f.enabled).count()
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
