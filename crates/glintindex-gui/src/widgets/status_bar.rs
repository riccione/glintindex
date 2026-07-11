//! Status bar widget.
//!
//! Renders a thin bar at the bottom of the window showing
//! a status message (e.g. result count, folder count).

use iced::widget::{container, rule, text};

use crate::message::Message;

/// Creates the status bar widget.
///
/// Displays a short status message at the bottom of the window.
/// Also renders a thin separator line above the text.
pub fn view<'a>(status: &'a str, result_count: usize) -> iced::Element<'a, Message> {
    let status_text = if result_count > 0 {
        format!(
            "{} — {} result{}",
            status,
            result_count,
            if result_count == 1 { "" } else { "s" }
        )
    } else {
        status.to_string()
    };

    container(iced::widget::column![
        rule::horizontal(1),
        text(status_text)
            .size(12)
            .color(iced::Color::from_rgb(0.4, 0.4, 0.4))
    ])
    .width(iced::Length::Fill)
    .padding(iced::Padding::new(6.0).horizontal(12.0))
    .into()
}
