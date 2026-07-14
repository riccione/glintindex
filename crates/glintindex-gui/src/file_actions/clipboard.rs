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
    // Use the standard system clipboard
    let clipboard = window.clipboard();

    // Optional: Use canonicalize() if you want to ensure the path is absolute
    let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let path_str = abs_path.to_string_lossy().to_string();

    clipboard.set_text(&path_str);
    Ok(())
}
