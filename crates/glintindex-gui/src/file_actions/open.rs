//! Open file with the system's default application.
//!
//! Uses the `open` crate for cross-platform file opening.

use std::path::Path;

/// Opens a file with the operating system's default application.
///
/// On Linux, this uses `xdg-open` via the `open` crate.
/// On macOS, this uses `open`.
/// On Windows, this uses `ShellExecuteW`.
///
/// # Errors
///
/// Returns an error if no default application is configured
/// or if the file cannot be opened.
pub fn open_file(path: &Path) -> anyhow::Result<()> {
    open::that(path).map_err(|e| anyhow::anyhow!("Failed to open file: {e}"))
}
