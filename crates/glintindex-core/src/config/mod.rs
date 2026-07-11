//! Configuration management for GlintIndex.
//!
//! This module provides the application configuration, including
//! indexed folders, ignored folders, index directory, theme,
//! preview size, and recent searches.

pub mod defaults;
pub mod loader;
pub mod paths;

#[allow(clippy::module_inception)]
mod config;

pub use config::{AppConfig, Theme};
pub use paths::AppPaths;
