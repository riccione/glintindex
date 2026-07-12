//! Index management settings page.
//!
//! Provides index management actions (index all, rebuild, clear)
//! and displays application statistics.

use iced::widget::{button, column, container, progress_bar, row, rule, scrollable, text};

use crate::message::Message;
use crate::state::AppState;

/// Renders the Index management settings page.
///
/// Shows action buttons for index operations, displays current
/// statistics, and shows a progress bar during indexing.
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

    // Progress section (shown during indexing)
    let progress_section = if let Some(ref progress) = state.current_progress {
        let status_text = text(progress.status_message.clone())
            .size(14)
            .color(iced::Color::from_rgb(0.3, 0.5, 0.8));

        // Progress bar
        let (pos, len) = match progress.total_files {
            Some(total) if total > 0 => (progress.files_processed as f32, total as f32),
            _ => (0.0, 1.0), // Indeterminate
        };
        let bar = progress_bar(0.0..=len, pos)
            .length(iced::Length::Fill)
            .girth(20);

        // File count text
        let count_text = match progress.total_files {
            Some(total) => text(format!("{} / {} files", progress.files_processed, total))
                .size(13)
                .color(iced::Color::from_rgb(0.4, 0.4, 0.4)),
            None => text(format!("{} files processed", progress.files_processed))
                .size(13)
                .color(iced::Color::from_rgb(0.4, 0.4, 0.4)),
        };

        // Current file
        let file_text = match &progress.current_file {
            Some(name) => text(name.as_str())
                .size(12)
                .color(iced::Color::from_rgb(0.5, 0.5, 0.5)),
            None => text("".to_string())
                .size(12)
                .color(iced::Color::from_rgb(0.5, 0.5, 0.5)),
        };

        // Stats summary
        let stats_line = format!(
            "Indexed: {} | Skipped: {} | Errors: {} | Panics: {}",
            progress.files_indexed,
            progress.files_skipped,
            progress.parser_errors,
            progress.parser_panics
        );
        let stats_text = text(stats_line)
            .size(11)
            .color(iced::Color::from_rgb(0.5, 0.5, 0.5));

        column![status_text, bar, count_text, file_text, stats_text].spacing(6)
    } else {
        column![]
    };

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
    let status_section = if state.settings_status.is_empty() && state.current_progress.is_none() {
        column![]
    } else if state.current_progress.is_none() && !state.settings_status.is_empty() {
        column![
            text(state.settings_status.clone())
                .size(13)
                .color(iced::Color::from_rgb(0.3, 0.5, 0.8))
        ]
    } else {
        column![]
    };

    let content = column![header, progress_section, stats_col, status_section]
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
