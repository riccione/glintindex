//! Reveal file in the system file manager.
//!
//! Opens the parent directory of a file in the default file manager.
//! On Linux, attempts to select the file if supported.

use std::path::Path;

/// Reveals a file in the system file manager.
///
/// Opens the parent directory and, on Linux, attempts to select
/// the file using the FreeDesktop file manager interface.
///
/// # Platform Behavior
///
/// - **Linux**: Opens the parent directory. File selection is attempted
///   but falls back to just opening the directory if not supported.
/// - **macOS**: Opens Finder at the parent directory.
/// - **Windows**: Opens Explorer at the parent directory.
///
/// # Errors
///
/// Returns an error if the file manager cannot be opened.
pub fn reveal_in_file_manager(path: &Path) -> anyhow::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("/"));

    // Try to reveal the file in the file manager
    #[cfg(target_os = "linux")]
    {
        // Try org.freedesktop.FileManager1 DBus interface first
        let arg = path.to_string_lossy().to_string();
        let result = std::process::Command::new("dbus-send")
            .arg("--session")
            .arg("--dest=org.freedesktop.FileManager1")
            .arg("--type=method_call")
            .arg("/org/freedesktop/FileManager1")
            .arg("org.freedesktop.FileManager1.ShowItems")
            .arg(format!("array:s:string:{arg}"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        if let Ok(status) = result {
            if status.success() {
                return Ok(());
            }
        }

        // Fallback: open the parent directory
        open::that(parent).map_err(|e| anyhow::anyhow!("Failed to open folder: {e}"))
    }

    #[cfg(not(target_os = "linux"))]
    {
        open::that(parent).map_err(|e| anyhow::anyhow!("Failed to open folder: {e}"))
    }
}
