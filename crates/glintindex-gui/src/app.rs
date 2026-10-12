//! Application state and egui update loop.
//!
//! [`GlintApp`] replaces the old GTK `WindowState` + `GlintIndexWindow`:
//! all UI state lives in one owned struct, and [`GlintApp::ui`] draws
//! one frame. The `eframe::App` impl is a thin wrapper so tests can
//! drive `ui` headlessly via `egui::Context::run`.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use eframe::egui;

use glintindex_core::{ApplicationService, PreviewService, SearchResult};

use crate::preview_model::PreviewModel;
use crate::search;
use crate::theme;
use crate::ui;

/// How often to poll a running indexing job.
const INDEX_POLL: Duration = Duration::from_millis(200);

/// Which full-window view is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// Toolbar + results + preview.
    Main,
    /// First-run welcome screen (no indexed folders).
    Empty,
    /// Full-window settings view.
    Settings,
}

/// Settings pages, mirroring the old GTK sidebar navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsPage {
    General,
    Appearance,
    IndexedFolders,
    IgnoredFolders,
    Index,
    About,
}

impl SettingsPage {
    pub const ALL: [SettingsPage; 6] = [
        SettingsPage::General,
        SettingsPage::Appearance,
        SettingsPage::IndexedFolders,
        SettingsPage::IgnoredFolders,
        SettingsPage::Index,
        SettingsPage::About,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Appearance => "Appearance",
            Self::IndexedFolders => "Indexed Folders",
            Self::IgnoredFolders => "Ignored Folders",
            Self::Index => "Index",
            Self::About => "About",
        }
    }
}

/// A background indexing job the app is polling to completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollKind {
    /// Startup auto-index: streams status into the main status bar.
    Startup,
    /// Empty-state first index: switches to the main view when done.
    EmptyIndex,
    /// Index-management page job: feeds the stats panel.
    IndexPage,
}

/// The eframe application.
pub struct GlintApp {
    pub service: ApplicationService,
    pub preview_service: PreviewService,

    // ── Search ────────────────────────────────────────────────
    /// TextEdit buffer (may transiently differ from `query` mid-frame).
    pub search_text: String,
    pub query: String,
    /// Last query text handed to [`Self::dispatch_search`].
    pub last_dispatched: String,
    pub last_query_edit: Option<Instant>,
    pub results: Vec<SearchResult>,
    pub selected_index: Option<usize>,
    pub selected_path: Option<PathBuf>,
    pub preview_model: Option<PreviewModel>,
    pub status: String,
    pub current_page: usize,
    pub total_pages: usize,
    pub per_page: usize,
    pub total_results: usize,
    pub search_rx: Option<mpsc::Receiver<(u64, glintindex_core::SearchResponse)>>,
    pub search_tx: Option<mpsc::Sender<(u64, glintindex_core::SearchResponse)>>,
    pub latest_applied_id: u64,

    // ── Views / settings ──────────────────────────────────────
    pub view: View,
    pub settings_page: SettingsPage,

    // ── Indexing ──────────────────────────────────────────────
    pub poll: Option<PollKind>,
    pub progress_active: bool,
    pub last_job_progress: Option<glintindex_core::tasks::Progress>,
    pub statistics: Option<glintindex_core::ApplicationStatistics>,
    /// Empty-state first-index in progress (buttons disabled).
    pub empty_indexing: bool,
    pub empty_error: Option<String>,

    // ── Appearance ────────────────────────────────────────────
    /// Last applied `(theme, font_size)`; `None` forces a re-apply.
    pub applied_theme: Option<(glintindex_core::Theme, u32)>,
    /// Font size staged by the settings UI but not yet applied.
    ///
    /// The font-size slider stages every change here; the draft is
    /// persisted (and the theme/font re-applied) only when the user
    /// releases the slider handle, presses Enter, or clicks outside
    /// the value box.
    pub font_size_draft: Option<i32>,
    /// Last rounded split ratio written to config.
    pub last_persisted_ratio: Option<f32>,
}

impl GlintApp {
    /// eframe entry point.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let service = ApplicationService::with_default_config()
            .expect("Failed to initialize ApplicationService");
        Self::with_service(service)
    }

    /// Builds the app around an existing service (shared with tests).
    pub fn with_service(service: ApplicationService) -> Self {
        let per_page = service.config().pagination.default_page_size.max(1);
        let preview_context_lines = service.config().clamped_preview_context_lines();
        let statistics = service.statistics().ok();
        let (search_tx, search_rx) = mpsc::channel::<(u64, glintindex_core::SearchResponse)>();

        let view = if service.has_enabled_folders() {
            View::Main
        } else {
            View::Empty
        };

        let mut app = Self {
            preview_service: PreviewService::new(glintindex_core::PreviewConfig {
                context_lines: preview_context_lines,
                ..Default::default()
            }),
            service,
            search_text: String::new(),
            query: String::new(),
            last_dispatched: String::new(),
            last_query_edit: None,
            results: Vec::new(),
            selected_index: None,
            selected_path: None,
            preview_model: None,
            status: "Ready".to_string(),
            current_page: 1,
            total_pages: 1,
            per_page,
            total_results: 0,
            search_rx: Some(search_rx),
            search_tx: Some(search_tx),
            latest_applied_id: 0,
            view,
            settings_page: SettingsPage::General,
            poll: None,
            progress_active: false,
            last_job_progress: None,
            statistics,
            empty_indexing: false,
            empty_error: None,
            applied_theme: None,
            font_size_draft: None,
            last_persisted_ratio: None,
        };

        app.startup_indexing();
        app
    }

    /// Starts auto-indexing on launch when configured.
    fn startup_indexing(&mut self) {
        let should = self.service.config().index_on_startup
            && self.service.has_enabled_folders()
            && !self.service.is_indexing();
        if !should {
            return;
        }
        match self.service.start_indexing() {
            Ok(_) => {
                tracing::info!(target: "glintindex::gui", "starting startup indexing job");
                self.status = "Startup indexing started...".to_string();
                self.progress_active = true;
                self.poll = Some(PollKind::Startup);
            }
            Err(e) => {
                tracing::warn!(target: "glintindex::gui", error = %e, "startup indexing failed to start");
            }
        }
    }

    /// One frame of UI. Called by `eframe::App::ui` and by tests.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.sync_theme(&ctx);
        self.drain_search_channel();
        self.poll_indexing(&ctx);
        self.handle_debounce(&ctx);

        if self.view == View::Settings {
            ui::settings::show(self, ui);
            return;
        }
        if self.view == View::Empty {
            ui::empty_state::show(self, ui);
            return;
        }

        ui::toolbar::show(self, ui);
        ui::status_bar::show(self, ui);
        ui::results::show(self, ui);
        ui::preview::show(self, ui);
    }

    /// Re-applies theme/font when config changes or the OS flips mode.
    fn sync_theme(&mut self, ctx: &egui::Context) {
        let theme = self.service.config().theme;
        let font_size = self.service.config().clamped_font_size();
        let system_flip = theme == glintindex_core::Theme::System;
        if self.applied_theme != Some((theme, font_size)) || system_flip {
            theme::apply_theme(ctx, theme, font_size);
            self.applied_theme = Some((theme, font_size));
        }
    }

    /// Drains background search results, keeping only the newest ID.
    fn drain_search_channel(&mut self) {
        let Some(rx) = &self.search_rx else { return };
        let mut newest: Option<(u64, glintindex_core::SearchResponse)> = None;
        while let Ok(msg) = rx.try_recv() {
            match &newest {
                Some((id, _)) if msg.0 <= *id => {}
                _ => newest = Some(msg),
            }
        }
        let Some((query_id, response)) = newest else {
            return;
        };
        if query_id < self.latest_applied_id {
            return;
        }
        // Results from a search dispatched before the input was cleared
        // must not repopulate the list.
        if self.query.trim().is_empty() {
            return;
        }
        self.latest_applied_id = query_id;
        search::apply_search_response(self, response);
    }

    /// Polls a running indexing job; re-arms repaint while it runs.
    fn poll_indexing(&mut self, ctx: &egui::Context) {
        let Some(kind) = self.poll else { return };

        enum Outcome {
            Running(Option<String>),
            Completed(Option<glintindex_core::tasks::Progress>),
            Failed(String),
        }

        let outcome = if self.service.is_indexing() {
            Outcome::Running(self.service.current_progress().map(|p| p.status_message))
        } else {
            match self.service.job_status() {
                Some(status) => match status.state {
                    glintindex_core::tasks::JobState::Failed(msg) => Outcome::Failed(msg),
                    _ => Outcome::Completed(self.service.current_progress()),
                },
                None => Outcome::Completed(self.service.current_progress()),
            }
        };

        match outcome {
            Outcome::Running(message) => {
                if kind == PollKind::Startup {
                    self.status = message.unwrap_or_else(|| "Indexing...".to_string());
                }
                ctx.request_repaint_after(INDEX_POLL);
            }
            Outcome::Completed(progress) => {
                if let Some(p) = progress {
                    self.last_job_progress = Some(p);
                }
                self.progress_active = false;
                self.statistics = self.service.statistics().ok();
                match kind {
                    PollKind::Startup => self.status = "Indexing completed".to_string(),
                    PollKind::EmptyIndex => {
                        self.empty_indexing = false;
                        self.view = if self.service.has_enabled_folders() {
                            View::Main
                        } else {
                            View::Empty
                        };
                    }
                    PollKind::IndexPage => {}
                }
                self.poll = None;
            }
            Outcome::Failed(msg) => {
                self.progress_active = false;
                self.status = format!("Indexing failed: {msg}");
                if kind == PollKind::EmptyIndex {
                    self.empty_indexing = false;
                    self.empty_error = Some(format!("Failed to start indexing: {msg}"));
                }
                self.poll = None;
            }
        }
    }

    /// Selects a result row: updates status, buttons, and loads preview.
    pub fn select_result(&mut self, index: usize) {
        if index >= self.results.len() {
            return;
        }
        self.selected_index = Some(index);
        let path = self.results[index].document.path.clone();
        self.status = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        self.selected_path = Some(path.clone());

        // Use indexed text when available for consistency with search
        // results, fall back to disk for edge cases; metadata-only files
        // get a placeholder.
        let output = if self.results[index].document.is_metadata_only {
            glintindex_core::PreviewOutput {
                path,
                lines: vec![glintindex_core::PreviewLine {
                    line_number: 1,
                    text: "Preview unavailable — indexed by file name and path only".into(),
                    syntax_spans: vec![],
                    match_highlights: vec![],
                }],
                truncated: false,
                encoding: glintindex_core::Encoding::Utf8,
                is_binary: false,
                error: None,
                original_size: 0,
            }
        } else if !self.results[index].document.content.is_empty() {
            self.preview_service.generate_preview(
                &self.results[index].document.content,
                &self.results[index].document.path,
                &self.query,
            )
        } else {
            self.preview_service
                .load_preview(&self.results[index].document.path, &self.query)
        };
        self.preview_model = Some(crate::preview_model::build_preview_model(&output));
    }

    /// Stages a font-size change without applying it yet.
    ///
    /// Called for every slider/input change; the staged value becomes
    /// effective only via [`Self::commit_font_size`].
    pub fn stage_font_size(&mut self, size: i32) {
        let min = crate::ui::settings::FONT_SIZE_MIN;
        let max = crate::ui::settings::FONT_SIZE_MAX;
        self.font_size_draft = Some(size.clamp(min, max));
    }

    /// Applies a staged font-size change, if any differs from config.
    pub fn commit_font_size(&mut self) {
        let Some(size) = self.font_size_draft.take() else {
            return;
        };
        if size == self.service.config().clamped_font_size() as i32 {
            return;
        }
        let _ = self.service.set_font_size(size as u32);
        self.applied_theme = None;
    }

    /// Drops a staged font-size change without applying it (Escape).
    pub fn discard_font_size(&mut self) {
        self.font_size_draft = None;
    }

    /// Leaves settings, returning to main or empty depending on folders.
    pub fn leave_settings(&mut self) {
        // Leaving via Back or a nav click counts as "clicked outside":
        // apply any pending font-size change (Escape discards first).
        self.commit_font_size();
        self.view = if self.service.has_enabled_folders() {
            View::Main
        } else {
            View::Empty
        };
    }

    /// Adds a folder picked via the native dialog and starts indexing.
    pub fn add_folder_and_index(&mut self, path: &Path) {
        if let Err(e) = self.service.add_folder(path) {
            self.empty_error = Some(format!("Failed to add folder: {e}"));
            return;
        }
        match self.service.start_indexing() {
            Ok(_) => {
                self.empty_error = None;
                self.empty_indexing = true;
                self.progress_active = true;
                self.poll = Some(PollKind::EmptyIndex);
            }
            Err(e) => {
                self.empty_error = Some(format!("Failed to start indexing: {e}"));
            }
        }
    }

    /// Starts a job from the index-management page.
    pub fn start_index_job(&mut self, kind: IndexJob) {
        let result = match kind {
            IndexJob::IndexAll => self.service.start_indexing().map(|_| ()),
            IndexJob::Rebuild => self.service.start_rebuild().map(|_| ()),
            IndexJob::Clear => self.service.clear_index(),
        };
        match result {
            Ok(_) => {
                self.progress_active = true;
                if matches!(kind, IndexJob::Clear) {
                    self.progress_active = false;
                    self.statistics = self.service.statistics().ok();
                } else {
                    self.poll = Some(PollKind::IndexPage);
                }
            }
            Err(e) => {
                self.status = format!("Operation failed: {e}");
                tracing::warn!(target: "glintindex::gui", error = %e, "index job failed to start");
            }
        }
    }
}

/// Jobs the index-management page can trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexJob {
    IndexAll,
    Rebuild,
    Clear,
}

impl eframe::App for GlintApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.ui(ui);
    }
}
