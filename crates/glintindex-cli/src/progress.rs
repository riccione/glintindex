//! CLI progress reporting using indicatif.
//!
//! Provides a [`ProgressBarReporter`] that implements the core
//! [`ProgressReporter`] trait and displays a live progress bar
//! using the `indicatif` crate.

use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use indicatif::{ProgressBar, ProgressStyle};

use glintindex_core::scanner::ProgressReporter;

/// A progress reporter that displays a live progress bar in the terminal.
///
/// Wraps an `indicatif::ProgressBar` and implements the core
/// [`ProgressReporter`] trait. The scanner calls this reporter during
/// file processing, and the progress bar updates in real-time.
pub struct ProgressBarReporter {
    bar: ProgressBar,
    total_hint: AtomicU64,
    last_file: Mutex<String>,
}

impl ProgressBarReporter {
    /// Creates a new `ProgressBarReporter` with the given total file count.
    ///
    /// If `total` is 0 or unknown, a spinner-style progress indicator
    /// is used instead of a determinate progress bar.
    pub fn new(total: u64) -> Self {
        let bar = if total > 0 {
            let pb = ProgressBar::new(total);
            let style = ProgressStyle::default_bar()
                .template(
                    "{prefix:.bold.dim} {spinner:.green} {bar:40.cyan/blue} {pos}/{len} {msg}",
                )
                .expect("valid template")
                .progress_chars("█░░");
            pb.set_style(style);
            pb.set_prefix("Indexing");
            pb
        } else {
            let pb = ProgressBar::new_spinner();
            let style = ProgressStyle::default_spinner()
                .template("{prefix:.bold.dim} {spinner:.green} {msg}")
                .expect("valid template");
            pb.set_style(style);
            pb.set_prefix("Indexing");
            pb
        };

        Self {
            bar,
            total_hint: AtomicU64::new(total),
            last_file: Mutex::new(String::new()),
        }
    }

    /// Finishes the progress bar with a clear.
    pub fn finish_and_clear(&self) {
        self.bar.finish_and_clear();
    }
}

impl ProgressReporter for ProgressBarReporter {
    fn on_file_discovered(&self, _path: &Path) {
        // Don't update on discovery - too frequent
    }

    fn on_file_indexed(&self, path: &Path) {
        let pos = self.bar.position();
        self.bar.set_position(pos + 1);

        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy().to_string();
            let truncated = truncate_path(&name_str, 40);
            if let Ok(mut last) = self.last_file.lock() {
                *last = truncated.clone();
            }
            self.bar.set_message(format!("Current: {}", truncated));
        }
    }

    fn on_file_skipped(&self, path: &Path) {
        let pos = self.bar.position();
        self.bar.set_position(pos + 1);

        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy().to_string();
            let truncated = truncate_path(&name_str, 40);
            self.bar.set_message(format!("Skipped: {}", truncated));
        }
    }

    fn on_file_failed(&self, path: &Path, _reason: &str) {
        let pos = self.bar.position();
        self.bar.set_position(pos + 1);

        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy().to_string();
            let truncated = truncate_path(&name_str, 40);
            self.bar.set_message(format!("Failed: {}", truncated));
        }
    }

    fn on_parser_error(&self, path: &Path, parser: &str, _reason: &str) {
        let pos = self.bar.position();
        self.bar.set_position(pos + 1);

        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy().to_string();
            let truncated = truncate_path(&name_str, 40);
            self.bar
                .set_message(format!("{parser} error: {}", truncated));
        }
    }

    fn on_parser_panic(&self, path: &Path, parser: &str) {
        let pos = self.bar.position();
        self.bar.set_position(pos + 1);

        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy().to_string();
            let truncated = truncate_path(&name_str, 40);
            self.bar
                .set_message(format!("{parser} panic: {}", truncated));
        }
    }

    fn set_total_files(&self, total: u64) {
        if total > 0 && self.total_hint.load(Ordering::Relaxed) == 0 {
            self.total_hint.store(total, Ordering::Relaxed);
            self.bar.set_length(total);
            self.bar.set_style(
                ProgressStyle::default_bar()
                    .template(
                        "{prefix:.bold.dim} {spinner:.green} {bar:40.cyan/blue} {pos}/{len} {msg}",
                    )
                    .expect("valid template")
                    .progress_chars("█░░"),
            );
        }
    }

    fn on_operation_started(&self, operation: &str) {
        self.bar.set_prefix(operation.to_string());
        self.bar.set_message("");
    }

    fn on_operation_completed(&self) {
        self.bar.set_message("Done");
    }
}

/// Truncates a path string to the given maximum length, adding "..." if needed.
fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else {
        format!("...{}", &path[path.len() - max_len + 3..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_path_short() {
        assert_eq!(truncate_path("hello.txt", 20), "hello.txt");
    }

    #[test]
    fn truncate_path_long() {
        let result = truncate_path("this_is_a_very_long_filename_that_needs_truncation.txt", 20);
        assert!(result.len() <= 20);
        assert!(result.starts_with("..."));
    }

    #[test]
    fn progress_bar_reporter_creation() {
        let _reporter = ProgressBarReporter::new(100);
    }

    #[test]
    fn progress_bar_reporter_zero_total() {
        let _reporter = ProgressBarReporter::new(0);
    }
}
