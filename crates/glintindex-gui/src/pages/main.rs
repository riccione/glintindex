//! Main page layout.
//!
//! Composes the search bar, split view (results + preview), and
//! status bar into the primary application layout.

use iced::widget::{button, column, container, row, text};

use crate::message::Message;
use crate::state::AppState;
use crate::widgets::{preview, results_list, search_bar, status_bar};

/// Renders the main page layout.
///
/// Layout structure:
/// ```text
/// ┌─────────────────────────────────────────┐
/// │  [Settings]       Search Input          │
/// ├──────────────────┬──────────────────────┤
/// │  Results List    │  Preview Pane        │
/// │                  │                      │
/// ├──────────────────┴──────────────────────┤
/// │  Status Bar                             │
/// └─────────────────────────────────────────┘
/// ```
pub fn view<'a>(state: &'a AppState) -> iced::Element<'a, Message> {
    // Settings button
    let settings_btn = button(text("Settings".to_string()).size(13))
        .on_press(Message::OpenSettings)
        .padding(iced::Padding::new(6.0).horizontal(12.0));

    // Search bar — full width, fixed height
    let search = search_bar::view(&state.query);

    // Header row: settings button + search bar
    let header = row![settings_btn, search]
        .spacing(8)
        .align_y(iced::Alignment::Center);

    // Results list — left pane, takes available width
    let results = results_list::view(&state.results, state.selected_index);

    // Preview pane — right pane, takes remaining width
    let selected = state.selected_result();
    let preview = preview::view(selected);

    // Split view: results (1/3) + preview (2/3)
    let split = row![results, preview].spacing(4).height(iced::Length::Fill);

    // Main layout: header | split | status
    let layout = column![
        header,
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
