//! Background preview generation.
//!
//! [`spawn_preview`] runs preview generation (syntax highlighting or a
//! disk read) on a detached thread so clicking a result never blocks
//! the UI frame; [`GlintApp::drain_preview_channel`] folds the response
//! into app state and [`GlintApp::invalidate_preview`] drops responses
//! for selections the user has already left.

use std::sync::mpsc;

use crate::app::GlintApp;
use crate::preview_model::{PreviewModel, build_preview_model};

/// Runs preview generation for one selection on a background thread.
///
/// Non-empty `content` uses the in-memory pipeline; empty falls back to
/// the disk loader. The fully built [`PreviewModel`] is tagged with
/// `request_id` so stale responses are dropped, then `ctx` (when
/// present) wakes the UI — egui does not repaint on its own while idle.
pub(crate) fn spawn_preview(
    request_id: u64,
    preview_service: glintindex_core::PreviewService,
    content: String,
    path: std::path::PathBuf,
    query: String,
    ctx: Option<eframe::egui::Context>,
    tx: mpsc::Sender<(u64, PreviewModel)>,
) {
    std::thread::spawn(move || {
        let output = if content.is_empty() {
            preview_service.load_preview(&path, &query)
        } else {
            preview_service.generate_preview(&content, &path, &query)
        };
        let model = build_preview_model(&output);
        let _ = tx.send((request_id, model));
        if let Some(ctx) = ctx {
            ctx.request_repaint();
        }
    });
}

impl GlintApp {
    /// Invalidates the current preview and any in-flight generation.
    ///
    /// Bumps the request id so a worker response for a superseded
    /// selection cannot repopulate the pane.
    pub(crate) fn invalidate_preview(&mut self) {
        self.preview_request_id = self.preview_request_id.wrapping_add(1);
        self.preview_model = None;
        self.preview_loading = false;
    }

    /// Applies a completed background preview for the current selection.
    ///
    /// Newest-wins within the drain; anything whose request id does not
    /// match the current selection is dropped.
    pub(crate) fn drain_preview_channel(&mut self) {
        let Some(rx) = &self.preview_rx else { return };
        let mut newest: Option<(u64, PreviewModel)> = None;
        while let Ok(msg) = rx.try_recv() {
            match &newest {
                Some((id, _)) if msg.0 <= *id => {}
                _ => newest = Some(msg),
            }
        }
        let Some((request_id, model)) = newest else {
            return;
        };
        if request_id != self.preview_request_id {
            return;
        }
        self.preview_model = Some(model);
        self.preview_loading = false;
    }
}
