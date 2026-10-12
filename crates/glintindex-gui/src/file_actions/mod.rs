//! File actions module.
//!
//! Provides cross-platform file operations for the GUI:
//! opening files and revealing in the file manager. Path copying is
//! handled by the egui context (`ctx.copy_text`).

pub mod open;
pub mod reveal;

pub use open::open_file;
pub use reveal::reveal_in_file_manager;
