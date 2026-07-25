//! CLI progress reporting using indicatif.
//!
//! Provides a [`ProgressBarReporter`] that implements the core
//! [`ProgressReporter`] trait and displays a live spinner with
//! streaming statistics using the `indicatif` crate.

use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use indicatif::{ProgressBar, ProgressStyle};

use glintindex_core::scanner::ProgressReporter;

/// Minimum interval between terminal re-renders.
const RENDER_INTERVAL: Duration = Duration::from_millis(100);

/// A progress reporter that displays a live spinner in the terminal.
///
/// Wraps an `indicatif::ProgressBar` in spinner mode and implements the
/// core [`ProgressReporter`] trait. The scanner calls this reporter
/// during file processing, and the spinner updates with streaming
/// statistics at most every 100ms.
pub struct ProgressBarReporter {
    bar: ProgressBar,
    processed: AtomicU64,
    indexed: AtomicU64,
    skipped: AtomicU64,
    failed: AtomicU64,
    last_render: Mutex<Instant>,
}

impl ProgressBarReporter {
    /// Creates a new `ProgressBarReporter` with a spinner indicator.
    pub fn new() -> Self {
        let bar = ProgressBar::new_spinner();
        let style = ProgressStyle::default_spinner()
            .template("{spinner:.green} {prefix:.bold.dim} {msg}")
            .expect("valid template");
        bar.set_style(style);
        bar.set_prefix("Indexing");

        Self {
            bar,
            processed: AtomicU64::new(0),
            indexed: AtomicU64::new(0),
            skipped: AtomicU64::new(0),
            failed: AtomicU64::new(0),
            last_render: Mutex::new(Instant::now()),
        }
    }

    /// Finishes the spinner and clears it.
    pub fn finish_and_clear(&self) {
        self.bar.finish_and_clear();
    }

    /// Renders the streaming statistics to the terminal if enough time
    /// has passed since the last render.
    fn maybe_render(&self) {
        let now = Instant::now();
        let mut last = self.last_render.lock().unwrap();
        if now.duration_since(*last) >= RENDER_INTERVAL {
            *last = now;
            let processed = self.processed.load(Ordering::Relaxed);
            let indexed = self.indexed.load(Ordering::Relaxed);
            let skipped = self.skipped.load(Ordering::Relaxed);
            let failed = self.failed.load(Ordering::Relaxed);
            self.bar.set_message(format!(
                "Processed: {processed} | Indexed: {indexed} | Skipped: {skipped} | Errors: {failed}"
            ));
        }
    }
}

impl ProgressReporter for ProgressBarReporter {
    fn on_file_discovered(&self, _path: &Path) {}

    fn on_file_indexed(&self, _path: &Path) {
        self.processed.fetch_add(1, Ordering::Relaxed);
        self.indexed.fetch_add(1, Ordering::Relaxed);
        self.bar.tick();
        self.maybe_render();
    }

    fn on_file_skipped(&self, _path: &Path) {
        self.processed.fetch_add(1, Ordering::Relaxed);
        self.skipped.fetch_add(1, Ordering::Relaxed);
        self.bar.tick();
        self.maybe_render();
    }

    fn on_file_failed(&self, _path: &Path, _reason: &str) {
        self.processed.fetch_add(1, Ordering::Relaxed);
        self.failed.fetch_add(1, Ordering::Relaxed);
        self.bar.tick();
        self.maybe_render();
    }

    fn on_parser_error(&self, _path: &Path, _parser: &str, _reason: &str) {
        self.processed.fetch_add(1, Ordering::Relaxed);
        self.skipped.fetch_add(1, Ordering::Relaxed);
        self.failed.fetch_add(1, Ordering::Relaxed);
        self.bar.tick();
        self.maybe_render();
    }

    fn on_parser_panic(&self, _path: &Path, _parser: &str) {
        self.processed.fetch_add(1, Ordering::Relaxed);
        self.skipped.fetch_add(1, Ordering::Relaxed);
        self.failed.fetch_add(1, Ordering::Relaxed);
        self.bar.tick();
        self.maybe_render();
    }

    fn on_operation_started(&self, operation: &str) {
        self.bar.set_prefix(operation.to_string());
        self.bar.set_message("");
    }

    fn on_operation_completed(&self) {
        // Force a final render with complete stats
        let processed = self.processed.load(Ordering::Relaxed);
        let indexed = self.indexed.load(Ordering::Relaxed);
        let skipped = self.skipped.load(Ordering::Relaxed);
        let failed = self.failed.load(Ordering::Relaxed);
        self.bar.finish_with_message(format!(
            "Done — Processed: {processed} | Indexed: {indexed} | Skipped: {skipped} | Errors: {failed}"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_bar_reporter_creation() {
        let _reporter = ProgressBarReporter::new();
    }

    #[test]
    fn progress_bar_reporter_counts() {
        let reporter = ProgressBarReporter::new();
        let path = Path::new("/test/file.txt");

        reporter.on_file_indexed(path);
        reporter.on_file_indexed(path);
        reporter.on_file_skipped(path);
        reporter.on_file_failed(path, "error");

        assert_eq!(reporter.processed.load(Ordering::Relaxed), 4);
        assert_eq!(reporter.indexed.load(Ordering::Relaxed), 2);
        assert_eq!(reporter.skipped.load(Ordering::Relaxed), 1);
        assert_eq!(reporter.failed.load(Ordering::Relaxed), 1);
    }
}
