//! Preview pane widget.
//!
//! Displays file content using `text_editor` in read-only mode with
//! syntax highlighting via `iced::highlighter`. This provides native
//! text selection and OS-native copy shortcuts (Ctrl+C / Cmd+C).
//!
//! # Why `text_editor` instead of `text`?
//!
//! Iced's `text()` widget does not support text selection. Only
//! `text_editor()` provides native text selection. Since
//! `text_editor()` borrows its `Content` by reference, the `Content`
//! must be stored in application state ([`AppState::preview_content`])
//! to outlive the view function.
//!
//! Editing is disabled by omitting the `on_action` callback — the
//! editor remains strictly read-only.

use iced::widget::{column, container, scrollable, text, text_editor};
use iced::{Element, Length};

use crate::message::Message;
use crate::state::AppState;
use glintindex_core::PreviewOutput;

/// Creates the preview pane widget.
///
/// Shows the file content with syntax highlighting using `text_editor`
/// in read-only mode. Supports native text selection and copying.
/// If no preview is loaded, displays a placeholder message.
pub fn view<'a>(state: &'a AppState) -> Element<'a, Message> {
    if state.preview_loading {
        return container(
            column![
                text("Loading preview...")
                    .size(14)
                    .color(iced::Color::from_rgb(0.3, 0.3, 0.3))
            ]
            .align_x(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(12)
        .style(|_theme: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0.92, 0.92, 0.92,
            ))),
            ..container::Style::default()
        })
        .into();
    }

    if let Some(error) = &state.preview_error {
        return container(
            column![
                text("Preview Error")
                    .size(14)
                    .color(iced::Color::from_rgb(0.8, 0.1, 0.1)),
                text(error.clone())
                    .size(12)
                    .color(iced::Color::from_rgb(0.3, 0.3, 0.3)),
            ]
            .spacing(8),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(12)
        .style(|_theme: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0.92, 0.92, 0.92,
            ))),
            ..container::Style::default()
        })
        .into();
    }

    match &state.current_preview {
        Some(preview) => preview_content(preview, state),
        None => placeholder_view(state),
    }
}

/// Creates the placeholder view when no preview is available.
fn placeholder_view(state: &AppState) -> Element<'_, Message> {
    let inner = if state.results.is_empty() {
        column![
            text("GlintIndex")
                .size(16)
                .color(iced::Color::from_rgb(0.2, 0.2, 0.2)),
            text("Search for files to preview their content")
                .size(12)
                .color(iced::Color::from_rgb(0.3, 0.3, 0.3)),
        ]
        .spacing(8)
        .align_x(iced::Alignment::Center)
    } else if state.selected_index.is_none() {
        column![
            text("Select a result")
                .size(14)
                .color(iced::Color::from_rgb(0.3, 0.3, 0.3)),
            text("Click or navigate to a search result to preview")
                .size(12)
                .color(iced::Color::from_rgb(0.4, 0.4, 0.4)),
        ]
        .spacing(4)
        .align_x(iced::Alignment::Center)
    } else {
        column![
            text("Loading preview...")
                .size(14)
                .color(iced::Color::from_rgb(0.3, 0.3, 0.3))
        ]
        .align_x(iced::Alignment::Center)
    };

    container(inner)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(12)
        .style(|_theme: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0.92, 0.92, 0.92,
            ))),
            ..container::Style::default()
        })
        .into()
}

/// Determines the syntax language from a file path extension.
fn syntax_language(path: &std::path::Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("txt")
        .to_string()
}

/// Creates the preview content using `text_editor` with syntax highlighting.
///
/// The `text_editor` uses `on_action` to receive events, but the
/// update handler filters out editing actions — keeping the preview
/// read-only while allowing text selection and copying.
fn preview_content<'a>(preview: &'a PreviewOutput, state: &'a AppState) -> Element<'a, Message> {
    let extension = syntax_language(&preview.path);

    let editor = text_editor(&state.preview_content)
        .on_action(Message::PreviewAction)
        .highlight(&extension, iced::highlighter::Theme::InspiredGitHub)
        .height(Length::Fill)
        .font(iced::Font::MONOSPACE)
        .size(12);

    container(scrollable(editor).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(0)
        .style(|_theme: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0.92, 0.92, 0.92,
            ))),
            ..container::Style::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
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
                glintindex_core::PreviewLine {
                    line_number: 1,
                    text: "fn main() {".to_string(),
                    syntax_spans: vec![],
                    match_highlights: vec![],
                },
                glintindex_core::PreviewLine {
                    line_number: 2,
                    text: "    println!(\"Hello\");".to_string(),
                    syntax_spans: vec![],
                    match_highlights: vec![],
                },
                glintindex_core::PreviewLine {
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
        let preview = create_test_preview();
        state.update_preview_content(&preview);
        state.current_preview = Some(preview);

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
        state.update_preview_content(&preview);
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

    #[test]
    fn syntax_language_detection() {
        assert_eq!(syntax_language(std::path::Path::new("test.rs")), "rs");
        assert_eq!(syntax_language(std::path::Path::new("test.py")), "py");
        assert_eq!(syntax_language(std::path::Path::new("test.js")), "js");
        assert_eq!(syntax_language(std::path::Path::new("test")), "txt");
    }

    #[test]
    fn preview_content_update() {
        let mut state = create_test_state();
        let preview = create_test_preview();
        state.update_preview_content(&preview);
    }
}
