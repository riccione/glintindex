//! File actions module.
//!
//! Provides cross-platform file operations for the GUI:
//! opening files, revealing in file manager, and copying paths.

pub mod clipboard;
pub mod open;
pub mod reveal;

pub use clipboard::copy_path;
pub use open::open_file;
pub use reveal::reveal_in_file_manager;
