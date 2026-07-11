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
    }
}

/// Renders the application view.
///
/// Delegates to the main page layout, passing the current state.
pub fn view(state: &AppState) -> Element<'_, Message> {
    pages::main::view(state)
}
