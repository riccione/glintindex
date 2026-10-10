use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, Orientation, Paned, PolicyType,
    ScrolledWindow, Stack, TextBuffer, Window,
};

use glintindex_core::{ApplicationService, PreviewService, SearchResult};

use crate::theme::ThemeManager;
use crate::ui;

/// The main application window.
pub struct GlintIndexWindow {
    window: ApplicationWindow,
    #[allow(dead_code)]
    state: Rc<RefCell<WindowState>>,
    _main_content: MainContent,
}

/// UI controller for switching between empty state and main content views.
struct MainContent {
    stack: Stack,
}

impl MainContent {
    fn refresh(&self, service: &ApplicationService) {
        self.stack
            .set_visible_child_name(if service.has_enabled_folders() {
                "main"
            } else {
                "empty"
            });
    }
}

/// Mutable application state held inside the window.
pub struct WindowState {
    pub service: ApplicationService,
    pub preview_service: PreviewService,
    pub query: String,
    pub results: Vec<SearchResult>,
    pub selected_index: Option<usize>,
    pub status: String,
    pub preview_text: String,
    pub progress_active: bool,
    #[allow(dead_code)]
    pub progress_message: String,
    pub statistics: Option<glintindex_core::ApplicationStatistics>,
    /// Reference to the currently open settings window, if any.
    pub settings_window: Option<Window>,
    /// Reference to the preview TextBuffer for updating content.
    pub preview_buffer: Option<TextBuffer>,
    /// Progress from the most recent completed background job.
    pub last_job_progress: Option<glintindex_core::tasks::Progress>,
    /// Centralized theme manager for CSS loading and application.
    pub theme_manager: ThemeManager,
    /// Current page number (1-based).
    pub current_page: usize,
    /// Total number of pages.
    pub total_pages: usize,
    /// Number of results per page.
    pub per_page: usize,
    /// Total number of search results across all pages.
    pub total_results: usize,
    /// Reference to the status bar label for live updates.
    pub status_label: Option<gtk::Label>,
    /// Reference to the pagination page label.
    pub pagination_page_label: Option<gtk::Label>,
    /// Reference to the previous page button.
    pub pagination_prev_btn: Option<gtk::Button>,
    /// Reference to the next page button.
    pub pagination_next_btn: Option<gtk::Button>,
    /// Sender for background search requests.
    pub search_tx: Option<mpsc::Sender<(u64, glintindex_core::SearchResponse)>>,
}

impl GlintIndexWindow {
    /// Creates the main window for the application.
    pub fn new(app: &Application) -> Self {
        let service = ApplicationService::with_default_config()
            .expect("Failed to initialize ApplicationService");

        let theme = service.config().theme;
        let font_size = service.config().clamped_font_size();
        let per_page = service.config().pagination.default_page_size;
        let preview_context_lines = service.config().clamped_preview_context_lines();

        let statistics = service.statistics().ok();

        // Create the search channel
        let (search_tx, search_rx) = mpsc::channel::<(u64, glintindex_core::SearchResponse)>();

        let state = Rc::new(RefCell::new(WindowState {
            preview_service: PreviewService::new(glintindex_core::PreviewConfig {
                context_lines: preview_context_lines,
                ..Default::default()
            }),
            service,
            query: String::new(),
            results: Vec::new(),
            selected_index: None,
            status: "Ready".to_string(),
            preview_text: String::new(),
            progress_active: false,
            progress_message: String::new(),
            statistics,
            settings_window: None,
            preview_buffer: None,
            last_job_progress: None,
            theme_manager: ThemeManager::new(theme, font_size),
            current_page: 1,
            total_pages: 1,
            per_page,
            total_results: 0,
            status_label: None,
            pagination_page_label: None,
            pagination_prev_btn: None,
            pagination_next_btn: None,
            search_tx: Some(search_tx),
        }));

        // Keep the receiver outside of state since it's not Clone
        // (it's consumed by build_toolbar below)

        // ── Build the widget tree ──────────────────────────────────

        // Results list (left pane)
        let results_listbox = ui::results::build(&state);

        // Preview pane (right pane)
        let (preview_widget, preview_buffer) = ui::preview::build_with_buffer();

        // Store the preview buffer reference in state
        {
            let mut st = state.borrow_mut();
            st.preview_buffer = Some(preview_buffer);
        }

        // Action bar for selected result
        let open_btn = Button::builder().label("Open").build();
        let open_folder_btn = Button::builder().label("Open Folder").build();
        let copy_path_btn = Button::builder().label("Copy Path").build();

        // Initially disabled (no selection)
        open_btn.set_sensitive(false);
        open_folder_btn.set_sensitive(false);
        copy_path_btn.set_sensitive(false);

        let action_bar = GtkBox::new(Orientation::Horizontal, 8);
        action_bar.append(&open_btn);
        action_bar.append(&open_folder_btn);
        action_bar.append(&copy_path_btn);

        // Create the scroll area wrapping ONLY the preview widget
        let preview_scroll = ScrolledWindow::builder()
            .child(&preview_widget)
            .vscrollbar_policy(PolicyType::Automatic)
            .hscrollbar_policy(PolicyType::Automatic)
            .vexpand(true)
            .hexpand(true)
            .build();

        // Preview pane: Scrollable preview (top) + static action bar (bottom)
        let preview_pane = GtkBox::new(Orientation::Vertical, 0);
        preview_pane.append(&preview_scroll);
        preview_pane.append(&action_bar);

        // Connect result selection to preview loading and button state updates
        {
            let state_clone = state.clone();
            let open_btn_clone = open_btn.clone();
            let open_folder_btn_clone = open_folder_btn.clone();
            let copy_path_btn_clone = copy_path_btn.clone();
            results_listbox.connect_row_selected(move |_listbox, row| {
                if let Some(row) = row {
                    let index = row.index() as usize;
                    let mut st = state_clone.borrow_mut();
                    st.selected_index = Some(index);
                    if index < st.results.len() {
                        let path = st.results[index].document.path.clone();
                        st.status = path
                            .file_name()
                            .map(|f| f.to_string_lossy().to_string())
                            .unwrap_or_else(|| path.display().to_string());

                        // Enable action buttons
                        let file_exists = path.exists();
                        open_btn_clone.set_sensitive(file_exists);
                        open_folder_btn_clone.set_sensitive(true);
                        copy_path_btn_clone.set_sensitive(true);

                        // Load preview — use indexed text when available for consistency
                        // with search results, fall back to disk for edge cases
                        // (empty files indexed via watcher, corrupt stored fields).
                        // For metadata-only indexed files, show a placeholder.
                        let output = if st.results[index].document.is_metadata_only {
                            glintindex_core::PreviewOutput {
                                path: path.clone(),
                                lines: vec![glintindex_core::PreviewLine {
                                    line_number: 1,
                                    text:
                                        "Preview unavailable — indexed by file name and path only"
                                            .into(),
                                    syntax_spans: vec![],
                                    match_highlights: vec![],
                                }],
                                truncated: false,
                                encoding: glintindex_core::Encoding::Utf8,
                                is_binary: false,
                                error: None,
                                original_size: 0,
                            }
                        } else if !st.results[index].document.content.is_empty() {
                            st.preview_service.generate_preview(
                                &st.results[index].document.content,
                                &path,
                                &st.query,
                            )
                        } else {
                            st.preview_service.load_preview(&path, &st.query)
                        };
                        st.preview_text = format_preview_content(&output);

                        // Update the actual TextBuffer so the preview pane shows content
                        if let Some(ref buffer) = st.preview_buffer {
                            buffer.set_text(&st.preview_text);
                        }
                    }
                } else {
                    // No selection — disable all buttons
                    open_btn_clone.set_sensitive(false);
                    open_folder_btn_clone.set_sensitive(false);
                    copy_path_btn_clone.set_sensitive(false);
                }
            });
        }

        // Status bar (integrated with pagination controls)
        let status_bar_widgets = ui::status_bar::build(&state);

        // Store widget references in state for reactive updates
        {
            let mut st = state.borrow_mut();
            st.status_label = Some(status_bar_widgets.status_label);
            st.pagination_page_label = Some(status_bar_widgets.page_label);
            st.pagination_prev_btn = Some(status_bar_widgets.prev_btn);
            st.pagination_next_btn = Some(status_bar_widgets.next_btn);
        }

        // Scrolled windows for results and preview
        let results_scroll = ScrolledWindow::builder()
            .child(&results_listbox)
            .vscrollbar_policy(PolicyType::Automatic)
            .hscrollbar_policy(PolicyType::Never)
            .build();

        // Paned split view
        let initial_width = 1000;
        let initial_ratio = {
            let st = state.borrow();
            st.service.config().main_split_ratio_f32()
        };
        let initial_position = (initial_width as f32 * initial_ratio) as i32;

        let paned = Paned::builder()
            .orientation(Orientation::Horizontal)
            .start_child(&results_scroll)
            .end_child(&preview_pane)
            .position(initial_position)
            .build();

        // Save split ratio when divider is dragged
        {
            let state_clone = state.clone();
            paned.connect_position_notify(move |paned| {
                let position = paned.position();
                let total = paned.width() as f32;
                if total > 0.0 {
                    let ratio = (position as f32 / total).clamp(0.15, 0.85);
                    let rounded = (ratio * 100.0).round() / 100.0;
                    let mut st = state_clone.borrow_mut();
                    let _ = st.service.set_main_split_ratio(rounded);
                }
            });
        }

        // Toolbar with settings button and search entry
        let (toolbar, settings_btn) =
            ui::toolbar::build_toolbar(&state, &results_listbox, search_rx);

        // ── Content stack: empty state or main UI ──────────────────
        let content_stack = Stack::builder().build();
        let view_stack = content_stack.clone();

        // Create the window first so empty_state can reference it
        let window = ApplicationWindow::builder()
            .application(app)
            .title("GlintIndex")
            .default_width(1000)
            .default_height(700)
            .build();

        // Main vertical layout (toolbar + paned + status bar)
        let main_content = GtkBox::new(Orientation::Vertical, 0);
        main_content.prepend(&toolbar);
        main_content.append(&paned);
        main_content.append(&status_bar_widgets.container);

        // Empty state (shown when no indexed folders configured)
        let empty_state = ui::empty_state::build(&state, &window, view_stack.clone());

        content_stack.add_named(&empty_state, Some("empty"));
        content_stack.add_named(&main_content, Some("main"));

        let main_content_ctrl = MainContent {
            stack: content_stack.clone(),
        };
        {
            let st = state.borrow();
            main_content_ctrl.refresh(&st.service);
        }

        window.set_child(Some(&content_stack));

        // Wire up action button click handlers (after window creation for clipboard access)
        {
            let state_clone = state.clone();
            open_btn.connect_clicked(move |_| {
                let st = state_clone.borrow();
                if let Some(index) = st.selected_index {
                    if index < st.results.len() {
                        let path = &st.results[index].document.path;
                        if let Err(e) = crate::file_actions::open_file(path) {
                            tracing::warn!(
                                target: "glintindex::gui",
                                path = %path.display(),
                                error = %e,
                                "open file failed"
                            );
                        }
                    }
                }
            });
        }
        {
            let state_clone = state.clone();
            open_folder_btn.connect_clicked(move |_| {
                let st = state_clone.borrow();
                if let Some(index) = st.selected_index {
                    if index < st.results.len() {
                        let path = &st.results[index].document.path;
                        if let Err(e) = crate::file_actions::reveal_in_file_manager(path) {
                            tracing::warn!(
                                target: "glintindex::gui",
                                path = %path.display(),
                                error = %e,
                                "reveal in file manager failed"
                            );
                        }
                    }
                }
            });
        }
        {
            let state_clone = state.clone();
            let window_ref = window.clone();
            copy_path_btn.connect_clicked(move |_| {
                let st = state_clone.borrow();
                if let Some(index) = st.selected_index {
                    if index < st.results.len() {
                        let path = &st.results[index].document.path;
                        if let Err(e) = crate::file_actions::copy_path(path, &window_ref) {
                            tracing::warn!(
                                target: "glintindex::gui",
                                path = %path.display(),
                                error = %e,
                                "copy path failed"
                            );
                        }
                    }
                }
            });
        }

        // Connect settings button to open/close settings window (toggle)
        {
            let window_clone = window.clone();
            let state_clone = state.clone();
            let view_stack_clone = view_stack.clone();
            settings_btn.connect_clicked(move |_| {
                let window_to_close = {
                    let st = state_clone.borrow();
                    st.settings_window.clone()
                };

                if let Some(existing) = window_to_close {
                    existing.set_visible(false);
                    let mut st = state_clone.borrow_mut();
                    st.settings_window = None;
                } else {
                    ui::settings::show_settings(
                        &window_clone,
                        &state_clone,
                        None,
                        view_stack_clone.clone(),
                    );
                }
            });
        }

        // Start indexing at startup when enabled (runs on a background thread)
        {
            let should_auto_index = {
                let st = state.borrow();
                st.service.config().index_on_startup
                    && st.service.has_enabled_folders()
                    && !st.service.is_indexing()
            };
            if should_auto_index {
                let mut st = state.borrow_mut();
                match st.service.start_indexing() {
                    Ok(_) => {
                        tracing::info!(
                            target: "glintindex::gui",
                            "starting startup indexing job"
                        );
                        st.status = "Startup indexing started...".to_string();
                        st.progress_active = true;
                        if let Some(ref label) = st.status_label {
                            label.set_text(&st.status);
                        }
                        drop(st);
                        schedule_startup_job_watcher(&state);
                    }
                    Err(e) => {
                        tracing::warn!(
                            target: "glintindex::gui",
                            error = %e,
                            "startup indexing failed to start"
                        );
                    }
                }
            }
        }

        Self {
            window,
            state,
            _main_content: main_content_ctrl,
        }
    }

    /// Presents the window to the user.
    pub fn present(&self) {
        // Re-trigger style update right before presenting to guarantee repaint
        let st = self.state.borrow();
        let theme = st.service.config().theme;
        let font_size = st.service.config().clamped_font_size();
        st.theme_manager.apply(theme, font_size);

        self.window.present();
    }
}

impl WindowState {
    /// Refreshes the cached statistics from the service.
    pub fn refresh_statistics(&mut self) {
        self.statistics = self.service.statistics().ok();
    }
}

/// Schedules a timer that tracks the startup indexing job to completion.
///
/// Updates the status bar with live progress while the job runs, then
/// stores the final progress in state and shows a completion or failure
/// message. The timer stops as soon as the job is no longer running.
fn schedule_startup_job_watcher(state: &Rc<RefCell<WindowState>>) {
    let state_clone = state.clone();

    gtk::glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
        enum JobOutcome {
            Running(Option<String>),
            Completed(glintindex_core::tasks::Progress),
            Failed(String),
            Finished,
        }

        // Short borrow, released before any widget updates
        let outcome = {
            let st = state_clone.borrow();
            if st.service.is_indexing() {
                JobOutcome::Running(st.service.current_progress().map(|p| p.status_message))
            } else {
                match st.service.job_status() {
                    Some(status) => match status.state {
                        glintindex_core::tasks::JobState::Failed(msg) => JobOutcome::Failed(msg),
                        _ => match st.service.current_progress() {
                            Some(progress) => JobOutcome::Completed(progress),
                            None => JobOutcome::Finished,
                        },
                    },
                    None => JobOutcome::Finished,
                }
            }
        };

        match outcome {
            JobOutcome::Running(message) => {
                let message = message.unwrap_or_else(|| "Indexing...".to_string());
                let mut st = state_clone.borrow_mut();
                st.status = message.clone();
                if let Some(ref label) = st.status_label {
                    label.set_text(&message);
                }
                gtk::glib::ControlFlow::Continue
            }
            JobOutcome::Completed(progress) => {
                let mut st = state_clone.borrow_mut();
                st.last_job_progress = Some(progress);
                st.progress_active = false;
                st.refresh_statistics();
                st.status = "Indexing completed".to_string();
                if let Some(ref label) = st.status_label {
                    label.set_text(&st.status);
                }
                gtk::glib::ControlFlow::Break
            }
            JobOutcome::Failed(msg) => {
                let mut st = state_clone.borrow_mut();
                st.progress_active = false;
                st.status = format!("Indexing failed: {msg}");
                if let Some(ref label) = st.status_label {
                    label.set_text(&st.status);
                }
                gtk::glib::ControlFlow::Break
            }
            JobOutcome::Finished => {
                // Job ended without a captured progress snapshot
                let mut st = state_clone.borrow_mut();
                st.progress_active = false;
                st.status = "Indexing completed".to_string();
                if let Some(ref label) = st.status_label {
                    label.set_text(&st.status);
                }
                gtk::glib::ControlFlow::Break
            }
        }
    });
}

/// Formats preview output into displayable text.
pub fn format_preview_content(output: &glintindex_core::PreviewOutput) -> String {
    let mut content = String::new();

    if output.truncated {
        content.push_str(&format!(
            "// File truncated (showing first {} bytes)\n",
            output.lines.len() * 50
        ));
    }

    if output.encoding != glintindex_core::Encoding::Utf8 {
        content.push_str(&format!("// Encoding: {:?}\n", output.encoding));
    }

    for line in &output.lines {
        content.push_str(&format!("{:>4} {}\n", line.line_number, line.text));
    }

    content
}
