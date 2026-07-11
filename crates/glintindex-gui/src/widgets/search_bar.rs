//! Search bar widget.
//!
//! Renders a single-line text input with placeholder text.
//! Translates text changes into `Message::SearchChanged` and
//! Enter key presses into `Message::SearchSubmitted`.
//! Supports keyboard navigation for recent searches dropdown.

use iced::widget::{column, container, text, text_input};

use crate::message::Message;

/// The placeholder text shown in the search input.
const PLACEHOLDER: &str = "Search files...";

/// Creates the search bar text input widget.
///
/// The input updates on every keystroke, triggering live search
/// through `Message::SearchChanged`. Pressing Enter fires
/// `Message::SearchSubmitted`.
///
/// When `recent_searches_open` is true, displays a dropdown
/// of recent searches below the input.
pub fn view<'a>(
    query: &'a str,
    recent_searches: &'a [String],
    recent_searches_open: bool,
) -> iced::Element<'a, Message> {
    let input = text_input(PLACEHOLDER, query)
        .on_input(Message::SearchChanged)
        .on_submit(Message::SearchSubmitted)
        .padding(12)
        .width(iced::Length::Fill);

    if recent_searches_open && !recent_searches.is_empty() {
        let dropdown = container(
            column(
                recent_searches
                    .iter()
                    .map(|search| {
                        container(text(search).size(14))
                            .width(iced::Length::Fill)
                            .padding(iced::Padding::new(8.0).horizontal(12.0))
                            .into()
                    })
                    .collect::<Vec<iced::Element<'_, Message>>>(),
            )
            .spacing(1),
        )
        .width(iced::Length::Fill)
        .max_height(200)
        .style(iced::widget::container::bordered_box);

        column![input, dropdown].into()
    } else {
        input.into()
    }
}
