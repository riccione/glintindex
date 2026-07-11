//! Iced application entry point.
//!
//! Wires together the state, messages, and view into a running
//! Iced application using the function-based API.

use iced::{Element, Task};
use log::error;

use crate::message::Message;
use crate::pages;
use crate::state::AppState;

/// Returns the default configuration file path for the current platform.
///
/// Uses platform-standard locations via the `dirs` crate:
/// - Linux: `~/.config/glintindex/config.toml`
/// - macOS: `~/Library/Application Support/glintindex/config.toml`
/// - Windows: `C:\Users\<user>\AppData\Roaming\glintindex\config.toml`
fn default_config_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("glintindex")
        .join("config.toml")
}

/// Boots the application, returning the initial state.
///
/// Initializes `ApplicationService` with the default config path
/// and wraps it in the application state.
pub fn boot() -> (AppState, Task<Message>) {
    let config_path = default_config_path();

    let service = glintindex_core::ApplicationService::with_config_path(&config_path)
        .unwrap_or_else(|e| {
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

            if state.query.trim().is_empty() {
                state.results.clear();
                state.selected_index = None;
                return Task::none();
            }

            let query_obj = glintindex_core::SearchQuery::new(&state.query);

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
            }
            Task::none()
        }

        Message::SearchSubmitted => {
            if let Some(index) = state.selected_index {
                if index < state.results.len() {
                    let path = &state.results[index].document.path;
                    if let Err(e) = open::that(path) {
                        error!("Failed to open file: {}", e);
                        state.status = format!("Failed to open: {}", e);
                    }
                }
            } else if !state.results.is_empty() {
                state.selected_index = Some(0);
                let path = &state.results[0].document.path;
                if let Err(e) = open::that(path) {
                    error!("Failed to open file: {}", e);
                    state.status = format!("Failed to open: {}", e);
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
