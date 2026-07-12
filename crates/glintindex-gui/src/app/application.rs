//! Iced application entry point.
//!
//! Wires together the state, messages, and view into a running
//! Iced application using the function-based API.

use glintindex_core::PreviewService;
use iced::{Element, Task};
use log::error;

use crate::message::Message;
use crate::pages;
use crate::state::AppState;

/// Debounce delay in milliseconds.
const DEBOUNCE_MS: u64 = 300;

/// Boots the application, returning the initial state.
///
/// Initializes `ApplicationService` using the default platform config
/// path and wraps it in the application state.
pub fn boot() -> (AppState, Task<Message>) {
    let service = glintindex_core::ApplicationService::with_default_config().unwrap_or_else(|e| {
        error!("Failed to initialize ApplicationService: {}", e);
        panic!("Failed to initialize ApplicationService: {}", e);
    });

    let state = AppState::new(service);
    (state, Task::none())
}

/// Handles application state transitions.
///
/// Matches on incoming messages and updates the state accordingly.
/// Search operations are performed synchronously since `search()`
/// is fast for local Tantivy indices.
pub fn update(state: &mut AppState, message: Message) -> Task<Message> {
    match message {
        // ── Search ──────────────────────────────────────────────
        Message::SearchChanged(query) => {
            state.query = query;
            state.recent_searches_open = false;

            if state.query.trim().is_empty() {
                state.results.clear();
                state.selected_index = None;
                state.search_pending = false;
                state.pending_query.clear();
                return Task::none();
            }

            // Start debounce timer
            state.search_pending = true;
            state.pending_query = state.query.clone();

            Task::perform(
                async {
                    tokio::time::sleep(std::time::Duration::from_millis(DEBOUNCE_MS)).await;
                },
                |_| Message::SearchDebounced(String::new()),
            )
        }

        Message::SearchDebounced(_) => {
            if !state.search_pending {
                return Task::none();
            }
            state.search_pending = false;

            let query = state.pending_query.clone();
            state.pending_query.clear();

            if query.trim().is_empty() {
                return Task::none();
            }

            let query_obj = glintindex_core::SearchQuery::new(&query);

            match state.service.search(&query_obj) {
                Ok(results) => {
                    let count = results.len();
                    state.results = results;
                    state.selected_index = None;
                    state.status = format!(
                        "Found {} result{}",
                        count,
                        if count == 1 { "" } else { "s" }
                    );
                }
                Err(e) => {
                    state.results.clear();
                    state.selected_index = None;
                    state.status = format!("Search error: {}", e);
                    error!("Search failed: {}", e);
                }
            }

            Task::none()
        }

        Message::SearchCompleted(results) => {
            let count = results.len();
            state.results = results;
            state.selected_index = None;
            state.status = format!(
                "Found {} result{}",
                count,
                if count == 1 { "" } else { "s" }
            );
            Task::none()
        }

        Message::ResultSelected(index) => {
            if index < state.results.len() {
                state.selected_index = Some(index);
                let path = &state.results[index].document.path;
                state.status = path
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());

                // Trigger preview loading
                state.preview_loading = true;
                state.preview_error = None;
                state.preview_search_query.clear();

                let preview_service = PreviewService::with_default_config();
                let search_query = state.query.clone();
                let path_clone = path.clone();

                return Task::perform(
                    async move { preview_service.load_preview(&path_clone, &search_query) },
                    Message::PreviewLoaded,
                );
            }
            Task::none()
        }

        Message::ResultActivated(index) => {
            if index < state.results.len() {
                let path = state.results[index].document.path.clone();
                if let Err(e) = open::that(&path) {
                    error!("Failed to open file: {}", e);
                    state.status = format!("Failed to open: {}", e);
                } else {
                    let _ = state.service.add_recent_search(state.query.clone());
                    state.status = format!(
                        "Opened: {}",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    );
                }
            }
            Task::none()
        }

        Message::SearchSubmitted => {
            if let Some(index) = state.selected_index {
                if index < state.results.len() {
                    let path = state.results[index].document.path.clone();
                    if let Err(e) = open::that(&path) {
                        error!("Failed to open file: {}", e);
                        state.status = format!("Failed to open: {}", e);
                    } else {
                        let _ = state.service.add_recent_search(state.query.clone());
                        state.status = format!(
                            "Opened: {}",
                            path.file_name().unwrap_or_default().to_string_lossy()
                        );
                    }
                }
            } else if !state.results.is_empty() {
                state.selected_index = Some(0);
                let path = state.results[0].document.path.clone();
                if let Err(e) = open::that(&path) {
                    error!("Failed to open file: {}", e);
                    state.status = format!("Failed to open: {}", e);
                } else {
                    let _ = state.service.add_recent_search(state.query.clone());
                    state.status = format!(
                        "Opened: {}",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    );
                }
            }
            Task::none()
        }

        Message::SearchError(msg) => {
            state.results.clear();
            state.selected_index = None;
            state.status = format!("Error: {}", msg);
            Task::none()
        }

        // ── File Operations ─────────────────────────────────────
        Message::OpenFileRequested(index) => {
            if index < state.results.len() {
                let path = state.results[index].document.path.clone();
                if !path.exists() {
                    state.status = "File not found".to_string();
                } else if let Err(e) = open::that(&path) {
                    error!("Failed to open file: {}", e);
                    state.status = format!("Failed to open: {}", e);
                } else {
                    let _ = state.service.add_recent_search(state.query.clone());
                    state.status = format!(
                        "Opened: {}",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    );
                }
            }
            Task::none()
        }

        Message::OpenFolderRequested(index) => {
            if index < state.results.len() {
                let path = &state.results[index].document.path;
                let parent = path.parent().unwrap_or(path);
                if !parent.exists() {
                    state.status = "Folder not found".to_string();
                } else if let Err(e) = open::that(parent) {
                    error!("Failed to open folder: {}", e);
                    state.status = format!("Failed to open folder: {}", e);
                } else {
                    state.status = format!(
                        "Opened folder: {}",
                        parent.file_name().unwrap_or_default().to_string_lossy()
                    );
                }
            }
            Task::none()
        }

        Message::CopyPathRequested(index) => {
            if index < state.results.len() {
                let path = &state.results[index].document.path;
                let path_str = path.display().to_string();
                state.status = "Path copied.".to_string();
                iced::clipboard::write::<Message>(path_str)
            } else {
                Task::none()
            }
        }

        Message::ClipboardCompleted(msg) => {
            state.status = msg;
            Task::none()
        }

        // ── Recent Searches ─────────────────────────────────────
        Message::RecentSearchSelected(query) => {
            state.query = query.clone();
            state.recent_searches_open = false;
            state.search_pending = true;
            state.pending_query = query;

            Task::perform(
                async {
                    tokio::time::sleep(std::time::Duration::from_millis(DEBOUNCE_MS)).await;
                },
                |_| Message::SearchDebounced(String::new()),
            )
        }

        Message::ClearRecentSearches => {
            if let Err(e) = state.service.add_recent_search(String::new()) {
                error!("Failed to clear recent searches: {}", e);
            }
            state.recent_searches_open = false;
            state.status = "Recent searches cleared.".to_string();
            Task::none()
        }

        // ── Keyboard Navigation ─────────────────────────────────
        Message::NavigateUp => {
            if state.results.is_empty() {
                return Task::none();
            }
            let new_index = match state.selected_index {
                Some(i) if i > 0 => Some(i - 1),
                Some(i) => Some(i),
                None => Some(0),
            };
            if let Some(i) = new_index {
                state.selected_index = Some(i);
                let path = &state.results[i].document.path;
                state.status = path
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());
            }
            Task::none()
        }

        Message::NavigateDown => {
            if state.results.is_empty() {
                return Task::none();
            }
            let new_index = match state.selected_index {
                Some(i) if i + 1 < state.results.len() => Some(i + 1),
                Some(i) => Some(i),
                None => Some(0),
            };
            if let Some(i) = new_index {
                state.selected_index = Some(i);
                let path = &state.results[i].document.path;
                state.status = path
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());
            }
            Task::none()
        }

        Message::ActivateSelected => {
            if let Some(index) = state.selected_index {
                if index < state.results.len() {
                    let path = state.results[index].document.path.clone();
                    if let Err(e) = open::that(&path) {
                        error!("Failed to open file: {}", e);
                        state.status = format!("Failed to open: {}", e);
                    } else {
                        let _ = state.service.add_recent_search(state.query.clone());
                        state.status = format!(
                            "Opened: {}",
                            path.file_name().unwrap_or_default().to_string_lossy()
                        );
                    }
                }
            }
            Task::none()
        }

        // ── Preview ─────────────────────────────────────────────
        Message::PreviewRequested(path_str) => {
            let path = std::path::PathBuf::from(&path_str);
            state.preview_loading = true;
            state.preview_error = None;
            state.preview_search_query.clear();

            let preview_service = PreviewService::with_default_config();
            let search_query = state.query.clone();
            let path_clone = path.clone();

            Task::perform(
                async move { preview_service.load_preview(&path_clone, &search_query) },
                Message::PreviewLoaded,
            )
        }

        Message::PreviewLoaded(output) => {
            state.preview_loading = false;
            if let Some(error) = &output.error {
                state.preview_error = Some(error.clone());
                state.current_preview = None;
                state.status = format!("Preview error: {}", error);
            } else {
                state.current_preview = Some(output);
                state.preview_error = None;
            }
            Task::none()
        }

        Message::PreviewFailed(msg) => {
            state.preview_loading = false;
            state.preview_error = Some(msg.clone());
            state.current_preview = None;
            state.status = format!("Preview failed: {}", msg);
            Task::none()
        }

        Message::SearchHighlightsUpdated(query) => {
            state.preview_search_query = query;
            // Re-highlight the preview with new search query if we have a preview
            if let Some(preview) = &state.current_preview {
                let path = preview.path.clone();
                let preview_service = PreviewService::with_default_config();
                let search_query = state.preview_search_query.clone();

                Task::perform(
                    async move { preview_service.load_preview(&path, &search_query) },
                    Message::PreviewLoaded,
                )
            } else {
                Task::none()
            }
        }

        // ── Settings Navigation ─────────────────────────────────
        Message::OpenSettings => {
            state.settings_open = true;
            state.refresh_config_snapshot();
            state.refresh_statistics();
            state.settings_status.clear();
            Task::none()
        }

        Message::CloseSettings => {
            state.settings_open = false;
            state.settings_status.clear();
            Task::none()
        }

        Message::SettingsPageSelected(page) => {
            state.settings_page = page;
            state.settings_status.clear();
            Task::none()
        }

        // ── Indexed Folders ─────────────────────────────────────
        Message::AddFolderRequested => Task::perform(
            async {
                rfd::AsyncFileDialog::new()
                    .set_title("Select Folder to Index")
                    .pick_folder()
                    .await
            },
            |folder| match folder {
                Some(handle) => {
                    let path = handle.path().to_path_buf();
                    Message::FolderAdded(path.display().to_string())
                }
                None => Message::FolderAdded(String::new()),
            },
        ),

        Message::FolderAdded(path_str) => {
            if path_str.is_empty() {
                state.settings_status = "Folder selection cancelled.".to_string();
                return Task::none();
            }
            let path = std::path::PathBuf::from(&path_str);
            match state.service.add_folder(&path) {
                Ok(()) => {
                    state.refresh_config_snapshot();
                    state.settings_status = format!("Added: {}", path_str);
                }
                Err(e) => {
                    state.settings_status = format!("Failed to add folder: {}", e);
                    error!("Failed to add folder: {}", e);
                }
            }
            Task::none()
        }

        Message::RemoveFolderRequested(path_str) => {
            let path = std::path::PathBuf::from(&path_str);
            match state.service.remove_folder(&path) {
                Ok(()) => {
                    state.refresh_config_snapshot();
                    state.settings_status = format!("Removed: {}", path_str);
                }
                Err(e) => {
                    state.settings_status = format!("Failed to remove folder: {}", e);
                    error!("Failed to remove folder: {}", e);
                }
            }
            Task::none()
        }

        Message::EnableFolderRequested(path_str) => {
            let path = std::path::PathBuf::from(&path_str);
            match state.service.enable_folder(&path) {
                Ok(()) => {
                    state.refresh_config_snapshot();
                    state.settings_status = format!("Enabled: {}", path_str);
                }
                Err(e) => {
                    state.settings_status = format!("Failed to enable folder: {}", e);
                    error!("Failed to enable folder: {}", e);
                }
            }
            Task::none()
        }

        Message::DisableFolderRequested(path_str) => {
            let path = std::path::PathBuf::from(&path_str);
            match state.service.disable_folder(&path) {
                Ok(()) => {
                    state.refresh_config_snapshot();
                    state.settings_status = format!("Disabled: {}", path_str);
                }
                Err(e) => {
                    state.settings_status = format!("Failed to disable folder: {}", e);
                    error!("Failed to disable folder: {}", e);
                }
            }
            Task::none()
        }

        Message::FolderRemoved(_) | Message::FolderEnabled(_) | Message::FolderDisabled(_) => {
            Task::none()
        }

        // ── Ignored Folders ─────────────────────────────────────
        Message::AddIgnoredFolderRequested(name) => {
            let trimmed = name.trim().to_string();
            if trimmed.is_empty() {
                state.settings_status = "Please enter a folder name.".to_string();
                return Task::none();
            }
            match state.service.add_ignored_folder(trimmed.clone()) {
                Ok(()) => {
                    state.refresh_config_snapshot();
                    state.settings_status = format!("Added: {}", trimmed);
                }
                Err(e) => {
                    state.settings_status = format!("Failed to add: {}", e);
                    error!("Failed to add ignored folder: {}", e);
                }
            }
            Task::none()
        }

        Message::RemoveIgnoredFolderRequested(name) => {
            match state.service.remove_ignored_folder(&name) {
                Ok(()) => {
                    state.refresh_config_snapshot();
                    state.settings_status = format!("Removed: {}", name);
                }
                Err(e) => {
                    state.settings_status = format!("Failed to remove: {}", e);
                    error!("Failed to remove ignored folder: {}", e);
                }
            }
            Task::none()
        }

        Message::IgnoredFolderAdded(_) | Message::IgnoredFolderRemoved(_) => Task::none(),

        // ── Index Management ────────────────────────────────────
        Message::IndexRequested => {
            state.operation_in_progress = true;
            state.settings_status = "Indexing all folders...".to_string();

            let service_ref = &state.service;
            let stats_result = service_ref.index_all();

            match stats_result {
                Ok(stats) => {
                    let total: u64 = stats.iter().map(|s| s.files_indexed).sum();
                    state.settings_status = format!(
                        "Indexed {} file{}.",
                        total,
                        if total == 1 { "" } else { "s" }
                    );
                }
                Err(e) => {
                    state.settings_status = format!("Indexing failed: {}", e);
                    error!("Indexing failed: {}", e);
                }
            }

            state.operation_in_progress = false;
            state.refresh_statistics();
            Task::none()
        }

        Message::RebuildRequested => {
            state.operation_in_progress = true;
            state.settings_status = "Rebuilding index...".to_string();

            match state.service.rebuild_index() {
                Ok(()) => {
                    state.settings_status = "Index rebuilt successfully.".to_string();
                }
                Err(e) => {
                    state.settings_status = format!("Rebuild failed: {}", e);
                    error!("Rebuild failed: {}", e);
                }
            }

            state.operation_in_progress = false;
            state.refresh_statistics();
            Task::none()
        }

        Message::ClearRequested => {
            state.operation_in_progress = true;
            state.settings_status = "Clearing index...".to_string();

            match state.service.clear_index() {
                Ok(()) => {
                    state.settings_status = "Index cleared successfully.".to_string();
                }
                Err(e) => {
                    state.settings_status = format!("Clear failed: {}", e);
                    error!("Clear failed: {}", e);
                }
            }

            state.operation_in_progress = false;
            state.refresh_statistics();
            Task::none()
        }

        Message::IndexCompleted(msg)
        | Message::RebuildCompleted(msg)
        | Message::ClearCompleted(msg) => {
            state.settings_status = msg;
            state.operation_in_progress = false;
            state.refresh_statistics();
            Task::none()
        }

        Message::StatisticsUpdated => {
            state.refresh_statistics();
            Task::none()
        }
    }
}

/// Renders the application view.
///
/// If settings are open, shows the settings window layout.
/// Otherwise, shows the main search page.
pub fn view(state: &AppState) -> Element<'_, Message> {
    if state.settings_open {
        pages::settings::layout::view(state)
    } else {
        pages::main::view(state)
    }
}
