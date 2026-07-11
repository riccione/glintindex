use glintindex_core::{ApplicationService, SearchResult};

/// The complete application state for the GUI.
///
/// Holds both the business logic service and the UI-specific state.
/// The GUI never accesses core internals directly — all interaction
/// goes through `ApplicationService`.
pub struct AppState {
    /// The core application service providing search and indexing.
    pub service: ApplicationService,

    /// The current search query text.
    pub query: String,

    /// The most recent search results.
    pub results: Vec<SearchResult>,

    /// Index of the currently selected result, if any.
    pub selected_index: Option<usize>,

    /// Status message displayed in the status bar.
    pub status: String,
}

impl AppState {
    /// Creates a new `AppState` with the given application service.
    pub fn new(service: ApplicationService) -> Self {
        let status = Self::compute_status(service.config());
        Self {
            service,
            query: String::new(),
            results: Vec::new(),
            selected_index: None,
            status,
        }
    }

    /// Returns the currently selected search result, if any.
    pub fn selected_result(&self) -> Option<&SearchResult> {
        self.selected_index.and_then(|i| self.results.get(i))
    }

    /// Returns a status message reflecting the current index state.
    fn compute_status(config: &glintindex_core::AppConfig) -> String {
        let folder_count = config.indexed_folders.len();
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
