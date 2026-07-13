//! Main application window.
//!
//! Creates the primary application window with header bar, search,
//! split view (results + preview), and status bar.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Orientation, Paned, PolicyType, ScrolledWindow,
    TextBuffer, Window,
};

use glintindex_core::{ApplicationService, PreviewService, SearchResult};

use crate::ui;

/// The main application window.
pub struct GlintIndexWindow {
    window: ApplicationWindow,
    #[allow(dead_code)]
    state: Rc<RefCell<WindowState>>,
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
}

impl GlintIndexWindow {
    /// Creates the main window for the application.
    pub fn new(app: &Application) -> Self {
        let service = ApplicationService::with_default_config()
            .expect("Failed to initialize ApplicationService");

        let status = compute_status(&service);

        let statistics = service.statistics().ok();
        let state = Rc::new(RefCell::new(WindowState {
            preview_service: PreviewService::with_default_config(),
            service,
            query: String::new(),
            results: Vec::new(),
            selected_index: None,
            status,
            preview_text: String::new(),
            progress_active: false,
            progress_message: String::new(),
            statistics,
            settings_window: None,
            preview_buffer: None,
        }));

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

        // Connect result selection to preview loading
        {
            let state_clone = state.clone();
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

                        // Load preview synchronously (fast with cached SyntaxHighlighter)
                        let output = st.preview_service.load_preview(&path, &st.query);
                        st.preview_text = format_preview_content(&output);

                        // Update the actual TextBuffer so the preview pane shows content
                        if let Some(ref buffer) = st.preview_buffer {
                            buffer.set_text(&st.preview_text);
                        }
                    }
                }
            });
        }

        // Status bar
        let status_bar = ui::status_bar::build(&state);

        // Scrolled windows for results and preview
        let results_scroll = ScrolledWindow::builder()
            .child(&results_listbox)
            .vscrollbar_policy(PolicyType::Automatic)
            .hscrollbar_policy(PolicyType::Never)
            .build();

        let preview_scroll = ScrolledWindow::builder()
            .child(&preview_widget)
            .vscrollbar_policy(PolicyType::Automatic)
            .hscrollbar_policy(PolicyType::Automatic)
            .build();

        // Paned split view
        let paned = Paned::builder()
            .orientation(Orientation::Horizontal)
            .start_child(&results_scroll)
            .end_child(&preview_scroll)
            .position(350)
            .build();

        // Search bar + settings button
        let (header, settings_btn) = ui::search_bar::build(&state, &results_listbox);

        // Main vertical layout
        let content = GtkBox::new(Orientation::Vertical, 4);
        content.append(&header);
        content.append(&paned);
        content.append(&status_bar);

        // Create the window
        let window = ApplicationWindow::builder()
            .application(app)
            .title("GlintIndex")
            .default_width(1000)
            .default_height(700)
            .child(&content)
            .build();

        // Connect settings button to open/close settings window (toggle)
        {
            let window_clone = window.clone();
            let state_clone = state.clone();
            settings_btn.connect_clicked(move |_| {
                // Clone the window reference while borrowing state,
                // then drop the borrow before calling close() to avoid
                // a double-borrow panic when close-request fires.
                let window_to_close = {
                    let st = state_clone.borrow();
                    st.settings_window.clone()
                };

                if let Some(existing) = window_to_close {
                    // Settings window is open — close it
                    // The close-request handler will clear settings_window
                    existing.close();
                } else {
                    // Settings window is closed — open it
                    ui::settings::show_settings(&window_clone, &state_clone);
                }
            });
        }

        Self { window, state }
    }

    /// Presents the window to the user.
    pub fn present(&self) {
        self.window.present();
    }
}

impl WindowState {
    /// Refreshes the cached statistics from the service.
    pub fn refresh_statistics(&mut self) {
        self.statistics = self.service.statistics().ok();
    }
}

/// Computes the initial status message from the service.
fn compute_status(service: &ApplicationService) -> String {
    let folder_count = service.indexed_folders().len();
    if folder_count == 0 {
        "No folders configured".to_string()
    } else {
        format!(
            "Ready — {} folder{} configured",
            folder_count,
            if folder_count == 1 { "" } else { "s" }
        )
    }
}

/// Formats preview output into displayable text.
fn format_preview_content(output: &glintindex_core::PreviewOutput) -> String {
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
