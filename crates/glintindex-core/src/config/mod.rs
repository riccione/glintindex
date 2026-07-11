pub mod defaults;
pub mod loader;
pub mod paths;

#[allow(clippy::module_inception)]
mod config;

pub use config::{AppConfig, Theme};
pub use paths::AppPaths;
