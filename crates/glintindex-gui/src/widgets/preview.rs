//! Preview pane widget.
//!
//! Displays the text content of a selected search result.
//! Read-only, scrollable, preserves line breaks.
//! No syntax highlighting or editing.

use iced::widget::{column, container, scrollable, text};

use glintindex_core::SearchResult;

/// Creates the preview pane widget.
///
/// Shows the document's text content (from the search snippet)
/// in a read-only, scrollable container. If no result is selected,
/// displays a placeholder message.
pub fn view<'a>(
    result: Option<&'a SearchResult>,
) -> container::Container<'a, crate::message::Message> {
    let inner = match result {
        Some(result) => {
            let content = if result.snippet.is_empty() {
                &result.document.content
            } else {
                &result.snippet
            };

            let display_text = if content.is_empty() {
                "(no content)".to_string()
            } else {
                strip_html_tags(content)
            };

            column![scrollable(
                text(display_text).size(13).width(iced::Length::Fill)
            )]
        }
        None => {
            column![
                text("Select a result to preview")
                    .size(14)
                    .color(iced::Color::from_rgb(0.5, 0.5, 0.5))
            ]
        }
    };

    container(inner)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .padding(12)
}

/// Strips HTML tags from a string for plain text display.
fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;

    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }

    result.trim().to_string()
}
