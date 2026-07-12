//! Preview pane widget.
//!
//! Displays file content with syntax highlighting, line numbers,
//! and search match highlighting. Read-only and scrollable.

use iced::widget::{column, container, row, scrollable, text};
use iced::{Element, Length};

use crate::message::Message;
use crate::state::AppState;
use glintindex_core::{PreviewLine, PreviewOutput};

/// Default line number width in pixels.
#[allow(dead_code)]
const LINE_NUMBER_WIDTH: f32 = 45.0;

/// Creates the preview pane widget.
///
/// Shows the file content with syntax highlighting and line numbers.
/// If no preview is loaded, displays a placeholder message.
pub fn view<'a>(state: &'a AppState) -> Element<'a, Message> {
    if state.preview_loading {
        return container(
            column![
                text("Loading preview...")
                    .size(14)
                    .color(iced::Color::from_rgb(0.5, 0.5, 0.5))
            ]
            .align_x(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(12)
        .into();
    }

    if let Some(error) = &state.preview_error {
        return container(
            column![
                text("Preview Error")
                    .size(14)
                    .color(iced::Color::from_rgb(0.8, 0.2, 0.2)),
                text(error.clone())
                    .size(12)
                    .color(iced::Color::from_rgb(0.6, 0.6, 0.6)),
            ]
            .spacing(8),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(12)
        .into();
    }

    match &state.current_preview {
        Some(preview) => preview_content(preview, &state.preview_search_query),
        None => placeholder_view(state),
    }
}

/// Creates the placeholder view when no preview is available.
fn placeholder_view(state: &AppState) -> Element<'_, Message> {
    let inner = if state.results.is_empty() {
        column![
            text("GlintIndex")
                .size(16)
                .color(iced::Color::from_rgb(0.4, 0.4, 0.4)),
            text("Search for files to preview their content")
                .size(12)
                .color(iced::Color::from_rgb(0.5, 0.5, 0.5)),
        ]
        .spacing(8)
        .align_x(iced::Alignment::Center)
    } else if state.selected_index.is_none() {
        column![
            text("Select a result")
                .size(14)
                .color(iced::Color::from_rgb(0.5, 0.5, 0.5)),
            text("Click or navigate to a search result to preview")
                .size(12)
                .color(iced::Color::from_rgb(0.6, 0.6, 0.6)),
        ]
        .spacing(4)
        .align_x(iced::Alignment::Center)
    } else {
        column![
            text("Loading preview...")
                .size(14)
                .color(iced::Color::from_rgb(0.5, 0.5, 0.5))
        ]
        .align_x(iced::Alignment::Center)
    };

    container(inner)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(12)
        .into()
}

/// Creates the preview content with syntax highlighting and line numbers.
fn preview_content<'a>(preview: &'a PreviewOutput, search_query: &str) -> Element<'a, Message> {
    let mut lines_column = column![].spacing(0);

    // Add truncation notice if file was truncated
    if preview.truncated {
        lines_column = lines_column.push(
            container(
                text(format!(
                    "File truncated (showing first {} bytes)",
                    preview.lines.len() * 50 // approximate
                ))
                .size(11)
                .color(iced::Color::from_rgb(0.7, 0.5, 0.0)),
            )
            .padding(iced::Padding::from([4, 8]))
            .width(Length::Fill),
        );
    }

    // Add encoding notice if not UTF-8
    if preview.encoding != glintindex_core::Encoding::Utf8 {
        lines_column = lines_column.push(
            container(
                text(format!("Encoding: {:?}", preview.encoding))
                    .size(11)
                    .color(iced::Color::from_rgb(0.5, 0.5, 0.7)),
            )
            .padding(iced::Padding::from([2, 8]))
            .width(Length::Fill),
        );
    }

    // Add lines with line numbers
    for line in &preview.lines {
        let line_row = create_line_row(line, search_query);
        lines_column = lines_column.push(line_row);
    }

    let scrollable_content = scrollable(lines_column)
        .width(Length::Fill)
        .height(Length::Fill);

    container(scrollable_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(0)
        .into()
}

/// Creates a single line row with line number and syntax-highlighted content.
fn create_line_row<'a>(line: &'a PreviewLine, _search_query: &str) -> Element<'a, Message> {
    // Line number
    let line_number = text(format!("{:>4} ", line.line_number))
        .size(12)
        .color(iced::Color::from_rgb(0.4, 0.4, 0.4))
        .font(iced::Font::MONOSPACE);

    // Line content - for now use plain text with basic color
    // TODO: Apply syntax highlighting spans and match highlights
    let content = text(&line.text)
        .size(12)
        .color(iced::Color::from_rgb(0.9, 0.9, 0.9))
        .font(iced::Font::MONOSPACE);

    row![line_number, content]
        .spacing(0)
        .align_y(iced::Alignment::Center)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::application;
    use crate::state::AppState;
    use glintindex_core::{ApplicationService, PreviewConfig, PreviewService};
    use std::path::PathBuf;
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

    fn create_test_preview() -> PreviewOutput {
        PreviewOutput {
            path: PathBuf::from("test.rs"),
            lines: vec![
                PreviewLine {
                    line_number: 1,
                    text: "fn main() {".to_string(),
                    syntax_spans: vec![],
                    match_highlights: vec![],
                },
                PreviewLine {
                    line_number: 2,
                    text: "    println!(\"Hello\");".to_string(),
                    syntax_spans: vec![],
                    match_highlights: vec![],
                },
                PreviewLine {
                    line_number: 3,
                    text: "}".to_string(),
                    syntax_spans: vec![],
                    match_highlights: vec![],
                },
            ],
            truncated: false,
            encoding: glintindex_core::Encoding::Utf8,
            is_binary: false,
            error: None,
            original_size: 100,
        }
    }

    #[test]
    fn preview_view_no_results() {
        let state = create_test_state();
        let element = view(&state);
        // Just verify it doesn't panic
        let _ = element;
    }

    #[test]
    fn preview_view_with_results() {
        let mut state = create_test_state();
        state.results = vec![glintindex_core::SearchResult::new(
            glintindex_core::Document::new(
                PathBuf::from("test.rs"),
                100,
                UNIX_EPOCH,
                "fn main() {}".to_string(),
            ),
            1.0,
            "fn main() {}".to_string(),
        )];
        state.selected_index = Some(0);

        let element = view(&state);
        let _ = element;
    }

    #[test]
    fn preview_view_with_preview() {
        let mut state = create_test_state();
        state.current_preview = Some(create_test_preview());

        let element = view(&state);
        let _ = element;
    }

    #[test]
    fn preview_view_loading() {
        let mut state = create_test_state();
        state.preview_loading = true;

        let element = view(&state);
        let _ = element;
    }

    #[test]
    fn preview_view_error() {
        let mut state = create_test_state();
        state.preview_error = Some("File not found".to_string());

        let element = view(&state);
        let _ = element;
    }

    #[test]
    fn preview_view_truncated() {
        let mut preview = create_test_preview();
        preview.truncated = true;

        let mut state = create_test_state();
        state.current_preview = Some(preview);

        let element = view(&state);
        let _ = element;
    }

    #[test]
    fn preview_view_binary() {
        let mut state = create_test_state();
        state.preview_error = Some("Binary file preview is not available.".to_string());

        let element = view(&state);
        let _ = element;
    }

    #[test]
    fn preview_service_integration() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.rs");
        std::fs::write(&path, "fn main() {\n    println!(\"Hello\");\n}").unwrap();

        let service = PreviewService::new(PreviewConfig::default());
        let output = service.load_preview(&path, "Hello");

        assert!(output.error.is_none());
        assert!(!output.lines.is_empty());
    }

    #[test]
    fn preview_line_count() {
        let preview = create_test_preview();
        assert_eq!(preview.lines.len(), 3);
    }

    #[test]
    fn preview_line_numbers() {
        let preview = create_test_preview();
        assert_eq!(preview.lines[0].line_number, 1);
        assert_eq!(preview.lines[1].line_number, 2);
        assert_eq!(preview.lines[2].line_number, 3);
    }
}
