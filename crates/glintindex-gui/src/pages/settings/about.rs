//! About settings page.
//!
//! Displays application metadata including name, version,
//! Rust compiler version, license, and project description.

use iced::widget::{column, container, rule, text};

use crate::message::Message;

/// Application version from Cargo.toml at compile time.
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application name from Cargo.toml at compile time.
const APP_NAME: &str = env!("CARGO_PKG_NAME");

/// Application description from Cargo.toml at compile time.
const APP_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

/// Rust compiler version at compile time.
const RUST_VERSION: &str = env!("CARGO_PKG_RUST_VERSION");

/// Application license from Cargo.toml at compile time.
const APP_LICENSE: &str = env!("CARGO_PKG_LICENSE");

/// Renders the About settings page.
///
/// Shows application metadata in a clean, readable format.
pub fn view(_state: &crate::state::AppState) -> iced::Element<'static, Message> {
    let content = column![
        text("About".to_string()).size(20),
        rule::horizontal(1),
        info_row("Application".to_string(), APP_NAME.to_string()),
        info_row("Version".to_string(), APP_VERSION.to_string()),
        info_row("Description".to_string(), APP_DESCRIPTION.to_string()),
        info_row("Rust Version".to_string(), RUST_VERSION.to_string()),
        info_row("License".to_string(), APP_LICENSE.to_string()),
    ]
    .spacing(12)
    .width(iced::Length::Fill);

    container(content)
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
