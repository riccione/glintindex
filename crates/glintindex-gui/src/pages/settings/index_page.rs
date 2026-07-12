//! Index management settings page.
//!
//! Provides index management actions (index all, rebuild, clear)
//! and displays application statistics.

use iced::widget::{button, column, container, row, rule, scrollable, text};

use crate::message::Message;
use crate::state::AppState;

/// Renders the Index management settings page.
///
/// Shows action buttons for index operations and displays
/// current statistics about the index.
pub fn view<'a>(state: &'a AppState) -> iced::Element<'a, Message> {
    let busy = state.operation_in_progress;

    let index_btn = if busy {
        button(text("Index All".to_string()).size(14))
            .padding(iced::Padding::new(8.0).horizontal(16.0))
            .on_press_maybe(None)
    } else {
        button(text("Index All".to_string()).size(14))
            .on_press(Message::StartIndexing)
            .padding(iced::Padding::new(8.0).horizontal(16.0))
    };

    let rebuild_btn = if busy {
        button(text("Rebuild Index".to_string()).size(14))
            .padding(iced::Padding::new(8.0).horizontal(16.0))
            .on_press_maybe(None)
    } else {
        button(text("Rebuild Index".to_string()).size(14))
            .on_press(Message::StartRebuild)
            .padding(iced::Padding::new(8.0).horizontal(16.0))
    };

    let clear_btn = if busy {
        button(text("Clear Index".to_string()).size(14))
            .padding(iced::Padding::new(8.0).horizontal(16.0))
            .style(iced::widget::button::danger)
            .on_press_maybe(None)
    } else {
        button(text("Clear Index".to_string()).size(14))
            .on_press(Message::ClearRequested)
            .padding(iced::Padding::new(8.0).horizontal(16.0))
            .style(iced::widget::button::danger)
    };

    let actions = row![index_btn, rebuild_btn, clear_btn].spacing(8);

    let header = column![
        text("Index Management".to_string()).size(20),
        rule::horizontal(1),
        actions,
    ]
    .spacing(12);

    // Statistics section
    let mut stats_col = column![text("Statistics".to_string()).size(16)].spacing(6);

    if let Some(ref stats) = state.statistics {
        let docs_str = stats.indexed_documents.to_string();
        let folders_str = stats.indexed_folders.to_string();
        let enabled_str = state.enabled_folder_count().to_string();

        stats_col = stats_col
            .push(stat_row("Indexed Documents".to_string(), docs_str))
            .push(stat_row("Configured Folders".to_string(), folders_str))
            .push(stat_row("Enabled Folders".to_string(), enabled_str));

        if let Some(ref result) = stats.last_indexing_result {
            let discovered = result.files_discovered.to_string();
            let indexed = result.files_indexed.to_string();
            let skipped = result.files_skipped.to_string();
            let failed = result.files_failed.to_string();

            stats_col = stats_col
                .push(rule::horizontal(1))
                .push(text("Last Indexing Result".to_string()).size(14))
                .push(stat_row("Files Discovered".to_string(), discovered))
                .push(stat_row("Files Indexed".to_string(), indexed))
                .push(stat_row("Files Skipped".to_string(), skipped))
                .push(stat_row("Files Failed".to_string(), failed));
        }
    } else {
        stats_col = stats_col.push(
            text("No statistics available. Index some folders first.".to_string())
                .size(14)
                .color(iced::Color::from_rgb(0.5, 0.5, 0.5)),
        );
    }

    // Operation status
    let status_section = if state.settings_status.is_empty() {
        column![]
    } else {
        column![
            text(state.settings_status.clone())
                .size(13)
                .color(iced::Color::from_rgb(0.3, 0.5, 0.8))
        ]
    };

    let content = column![header, stats_col, status_section]
        .spacing(16)
        .width(iced::Length::Fill);

    container(scrollable(content).height(iced::Length::Fill))
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .padding(20)
        .into()
}

/// Creates a label-value row for displaying a statistic.
fn stat_row(label: String, value: String) -> iced::Element<'static, Message> {
    row![
        text(label)
            .size(13)
            .color(iced::Color::from_rgb(0.4, 0.4, 0.4)),
        text(value).size(13),
    ]
    .spacing(8)
    .width(iced::Length::Fill)
    .into()
}
