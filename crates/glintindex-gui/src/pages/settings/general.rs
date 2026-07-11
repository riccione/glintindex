//! General settings page.
//!
//! Displays read-only application information including the
//! configuration file location, index directory, and application version.

use iced::widget::{column, container, rule, scrollable, text};

use crate::message::Message;
use crate::state::AppState;

/// Application version from Cargo.toml at compile time.
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application name from Cargo.toml at compile time.
const APP_NAME: &str = env!("CARGO_PKG_NAME");

/// Renders the General settings page.
///
/// Shows configuration location, index location, and application version.
pub fn view<'a>(state: &'a AppState) -> iced::Element<'a, Message> {
    let config_display = state
        .service
        .config()
        .index_directory
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(not available)".to_string());

    let index_display = state.service.index_path().display().to_string();

    let content = column![
        text("General".to_string()).size(20),
        rule::horizontal(1),
        info_row("Application".to_string(), APP_NAME.to_string()),
        info_row("Version".to_string(), APP_VERSION.to_string()),
        info_row("Config Directory".to_string(), config_display),
        info_row("Index Directory".to_string(), index_display),
    ]
    .spacing(12)
    .width(iced::Length::Fill);

    container(scrollable(content).height(iced::Length::Fill))
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .padding(20)
        .into()
}

/// Creates a label-value row for displaying information.
fn info_row(label: String, value: String) -> iced::Element<'static, Message> {
    column![
        text(label)
            .size(12)
            .color(iced::Color::from_rgb(0.4, 0.4, 0.4)),
        text(value).size(14),
    ]
    .spacing(2)
    .width(iced::Length::Fill)
    .into()
}
