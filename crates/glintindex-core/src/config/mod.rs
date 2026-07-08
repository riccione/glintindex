pub mod defaults;
pub mod loader;

#[allow(clippy::module_inception)]
mod config;

pub use config::{AppConfig, Theme};
