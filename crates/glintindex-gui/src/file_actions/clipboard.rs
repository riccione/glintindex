//! Clipboard operations for copying file paths.
//!
//! Uses GTK's clipboard API for cross-platform clipboard access.

use std::path::Path;

use gtk::prelude::*;

/// Copies a file path to the system clipboard.
///
/// Uses GTK4's clipboard API which works across Linux, macOS, and Windows.
///
/// # Arguments
///
/// * `path` - The file path to copy to the clipboard.
/// * `window` - The GTK window to attach the clipboard to.
///
/// # Errors
///
/// Returns an error if the clipboard cannot be accessed.
pub fn copy_path(path: &Path, window: &gtk::ApplicationWindow) -> anyhow::Result<()> {
    let clipboard = window.primary_clipboard();
    let path_str = path.to_string_lossy().to_string();
    clipboard.set_text(&path_str);
    Ok(())
}
