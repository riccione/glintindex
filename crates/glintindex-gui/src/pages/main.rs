//! Main page layout.
//!
//! Composes the search bar, split view (results + preview), and
//! status bar into the primary application layout.

use iced::widget::{column, container, row};

use crate::message::Message;
use crate::state::AppState;
use crate::widgets::{preview, results_list, search_bar, status_bar};

/// Renders the main page layout.
///
/// Layout structure:
/// ```text
/// ┌─────────────────────────────────────────┐
/// │  Search Input                          │
/// ├──────────────────┬──────────────────────┤
/// │  Results List    │  Preview Pane        │
/// │                  │                      │
/// ├──────────────────┴──────────────────────┤
/// │  Status Bar                             │
/// └─────────────────────────────────────────┘
/// ```
pub fn view<'a>(state: &'a AppState) -> iced::Element<'a, Message> {
    // Search bar — full width, fixed height
    let search = search_bar::view(&state.query);

    // Results list — left pane, takes available width
    let results = results_list::view(&state.results, state.selected_index);

    // Preview pane — right pane, takes remaining width
    let selected = state.selected_result();
    let preview = preview::view(selected);

    // Split view: results (1/3) + preview (2/3)
    let split = row![results, preview].spacing(4);

    // Main layout: search | split | status
    let layout = column![
        search,
        split,
        status_bar::view(&state.status, state.results.len())
    ]
    .spacing(4);

    container(layout)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .padding(12)
        .into()
}
