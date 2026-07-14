//! Index Management settings page.
//!
//! Provides index management actions (index all, rebuild, clear)
//! and displays application statistics and progress.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, Label, Orientation, ProgressBar, Separator};

use crate::window::WindowState;

/// Builds the Index Management settings page.
pub fn build(state: &Rc<RefCell<WindowState>>) -> GtkBox {
    let content = GtkBox::new(Orientation::Vertical, 12);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);

    let title = Label::builder()
        .label("Index Management")
        .halign(gtk::Align::Start)
        .css_classes(["title-1"])
        .build();
    content.append(&title);
    content.append(&Separator::new(gtk::Orientation::Horizontal));

    // Action buttons
    let actions_box = GtkBox::new(Orientation::Horizontal, 8);

    let index_btn = Button::builder().label("Index All").build();
    let rebuild_btn = Button::builder().label("Rebuild Index").build();
    let clear_btn = Button::builder()
        .label("Clear Index")
        .css_classes(["destructive-action"])
        .build();

    actions_box.append(&index_btn);
    actions_box.append(&rebuild_btn);
    actions_box.append(&clear_btn);
    content.append(&actions_box);

    // Progress bar
    let progress_bar = ProgressBar::builder()
        .show_text(true)
        .visible(false)
        .build();
    content.append(&progress_bar);

    // Status label
    let status_label = Label::builder()
        .halign(gtk::Align::Start)
        .css_classes(["dim-label"])
        .build();
    content.append(&status_label);

    // Statistics section
    let stats_title = Label::builder()
        .label("Statistics")
        .halign(gtk::Align::Start)
        .css_classes(["title-2"])
        .build();
    content.append(&stats_title);
    content.append(&Separator::new(gtk::Orientation::Horizontal));

    let stats_box = GtkBox::new(Orientation::Vertical, 6);
    content.append(&stats_box);

    // Initial refresh
    let state_clone = state.clone();
    let status_clone = status_label.clone();
    let stats_clone = stats_box.clone();
    let progress_clone = progress_bar.clone();
    gtk::glib::idle_add_local(move || {
        refresh_stats(&state_clone, &status_clone, &stats_clone, &progress_clone);
        gtk::glib::ControlFlow::Break
    });

    // Connect Index All button
    let state_clone = state.clone();
    let status_clone = status_label.clone();
    let progress_clone = progress_bar.clone();
    let stats_clone = stats_box.clone();
    index_btn.connect_clicked(move |_| {
        let mut st = state_clone.borrow_mut();
        if st.service.is_indexing() {
            st.status = "Indexing already in progress.".to_string();
            return;
        }
        st.progress_active = true;
        st.status = "Starting background indexing...".to_string();
        match st.service.start_indexing() {
            Ok(_) => {
                progress_clone.set_visible(true);
                progress_clone.set_fraction(0.0);
                progress_clone.set_text(Some("Indexing..."));
            }
            Err(e) => {
                st.status = format!("Failed to start indexing: {e}");
                st.progress_active = false;
            }
        }
        drop(st);
        refresh_stats(&state_clone, &status_clone, &stats_clone, &progress_clone);
        schedule_progress_polling(&state_clone, &status_clone, &stats_clone, &progress_clone);
    });

    // Connect Rebuild button
    let state_clone = state.clone();
    let status_clone = status_label.clone();
    let progress_clone = progress_bar.clone();
    let stats_clone = stats_box.clone();
    rebuild_btn.connect_clicked(move |_| {
        let mut st = state_clone.borrow_mut();
        if st.service.is_indexing() {
            st.status = "A job is already in progress.".to_string();
            return;
        }
        st.progress_active = true;
        st.status = "Starting background rebuild...".to_string();
        match st.service.start_rebuild() {
            Ok(_) => {
                progress_clone.set_visible(true);
                progress_clone.set_fraction(0.0);
                progress_clone.set_text(Some("Rebuilding..."));
            }
            Err(e) => {
                st.status = format!("Failed to start rebuild: {e}");
                st.progress_active = false;
            }
        }
        drop(st);
        refresh_stats(&state_clone, &status_clone, &stats_clone, &progress_clone);
        schedule_progress_polling(&state_clone, &status_clone, &stats_clone, &progress_clone);
    });

    // Connect Clear button
    let state_clone = state.clone();
    let status_clone = status_label.clone();
    let stats_clone = stats_box.clone();
    let progress_clone = progress_bar.clone();
    clear_btn.connect_clicked(move |_| {
        let mut st = state_clone.borrow_mut();
        st.progress_active = true;
        st.status = "Clearing index...".to_string();
        match st.service.clear_index() {
            Ok(()) => {
                st.status = "Index cleared successfully.".to_string();
            }
            Err(e) => {
                st.status = format!("Clear failed: {e}");
            }
        }
        st.progress_active = false;
        st.refresh_statistics();
        drop(st);
        refresh_stats(&state_clone, &status_clone, &stats_clone, &progress_clone);
    });

    content
}

/// Schedules a progress polling timer that runs while a job is active.
///
/// The timer fires every 200ms to poll background job progress.
/// When the job finishes, the timer stops automatically.
/// When a new job starts, a new timer is created.
fn schedule_progress_polling(
    state: &Rc<RefCell<WindowState>>,
    status_label: &Label,
    stats_box: &GtkBox,
    progress_bar: &ProgressBar,
) {
    let state_clone = state.clone();
    let status_clone = status_label.clone();
    let progress_clone = progress_bar.clone();
    let stats_clone = stats_box.clone();

    gtk::glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
        let mut st = state_clone.borrow_mut();
        if !st.progress_active {
            // Job is not active — stop polling
            refresh_stats(&state_clone, &status_clone, &stats_clone, &progress_clone);
            return gtk::glib::ControlFlow::Break;
        }
        if let Some(progress) = st.service.current_progress() {
            st.status = progress.status_message.clone();
            if let Some(total) = progress.total_files {
                if total > 0 {
                    let fraction = progress.files_processed as f64 / total as f64;
                    progress_clone.set_fraction(fraction);
                    progress_clone.set_text(Some(&format!(
                        "{} / {} files",
                        progress.files_processed, total
                    )));
                }
            }
        }
        if !st.service.is_indexing() {
            // Store the completed job's progress for statistics display
            if let Some(progress) = st.service.current_progress() {
                st.last_job_progress = Some(progress);
            }
            st.progress_active = false;
            st.refresh_statistics();
            progress_clone.set_visible(false);
            progress_clone.set_fraction(0.0);
            drop(st);
            refresh_stats(&state_clone, &status_clone, &stats_clone, &progress_clone);
            return gtk::glib::ControlFlow::Break;
        }
        drop(st);
        refresh_stats(&state_clone, &status_clone, &stats_clone, &progress_clone);
        gtk::glib::ControlFlow::Continue
    });
}

/// Refreshes the statistics display.
fn refresh_stats(
    state: &Rc<RefCell<WindowState>>,
    status_label: &Label,
    stats_box: &GtkBox,
    progress_bar: &ProgressBar,
) {
    let st = state.borrow();
    status_label.set_text(&st.status);

    if st.progress_active {
        progress_bar.set_visible(true);
    }

    // Clear existing stats
    while let Some(child) = stats_box.first_child() {
        stats_box.remove(&child);
    }

    if let Some(ref stats) = st.statistics {
        add_stat_row(
            stats_box,
            "Indexed Documents",
            &stats.indexed_documents.to_string(),
        );
        add_stat_row(
            stats_box,
            "Configured Folders",
            &stats.indexed_folders.to_string(),
        );
    }

    // Show indexing results from the last completed job
    if let Some(ref progress) = st.last_job_progress {
        add_stat_row(
            stats_box,
            "Files Indexed",
            &progress.files_indexed.to_string(),
        );
        add_stat_row(
            stats_box,
            "Files Re-indexed",
            &progress.files_reindexed.to_string(),
        );
        add_stat_row(
            stats_box,
            "Files Skipped",
            &progress.files_unchanged.to_string(),
        );
        if progress.files_failed > 0 {
            add_stat_row(
                stats_box,
                "Files Failed",
                &progress.files_failed.to_string(),
            );
        }
        if progress.parser_errors > 0 {
            add_stat_row(
                stats_box,
                "Parser Errors",
                &progress.parser_errors.to_string(),
            );
        }
        if progress.parser_panics > 0 {
            add_stat_row(
                stats_box,
                "Parser Panics",
                &progress.parser_panics.to_string(),
            );
        }
    } else if st.statistics.is_none() {
        let label = Label::builder()
            .label("No statistics available. Index some folders first.")
            .css_classes(["dim-label"])
            .build();
        stats_box.append(&label);
    }
}

/// Adds a statistic row to the container.
fn add_stat_row(container: &GtkBox, label: &str, value: &str) {
    let row = GtkBox::new(Orientation::Horizontal, 8);

    let label_widget = Label::builder()
        .label(label)
        .hexpand(true)
        .halign(gtk::Align::Start)
        .css_classes(["dim-label"])
        .build();

    let value_widget = Label::builder()
        .label(value)
        .halign(gtk::Align::End)
        .selectable(true)
        .build();

    row.append(&label_widget);
    row.append(&value_widget);
    container.append(&row);
}
