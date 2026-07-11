//! Search bar widget.
//!
//! Renders a single-line text input with placeholder text.
//! Translates text changes into `Message::SearchChanged` and
//! Enter key presses into `Message::SearchSubmitted`.

use iced::widget::{TextInput, text_input};

use crate::message::Message;

/// The placeholder text shown in the search input.
const PLACEHOLDER: &str = "Search files...";

/// Creates the search bar text input widget.
///
/// The input updates on every keystroke, triggering live search
/// through `Message::SearchChanged`. Pressing Enter fires
/// `Message::SearchSubmitted`.
pub fn view<'a>(query: &'a str) -> TextInput<'a, Message> {
    text_input(PLACEHOLDER, query)
        .on_input(Message::SearchChanged)
        .on_submit(Message::SearchSubmitted)
        .padding(12)
        .width(iced::Length::Fill)
}
