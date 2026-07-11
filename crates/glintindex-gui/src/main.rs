//! GlintIndex GUI — Iced-based desktop search interface.
//!
//! This is the entry point for the graphical user interface.
//! It uses Iced 0.14's function-based API to create a native
//! desktop application with live search capabilities.

mod app;
mod message;
mod pages;
mod state;
mod theme;
mod widgets;

use app::application::{boot, update, view};

/// Application entry point.
///
/// Initializes the Iced application with the boot/update/view
/// function trio and runs it as a windowed application.
fn main() -> iced::Result {
    env_logger::init();

    iced::application(boot, update, view)
        .title("GlintIndex")
        .window_size((1000.0, 700.0))
        .run()
}
