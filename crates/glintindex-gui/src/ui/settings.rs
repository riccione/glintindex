//! Settings view (full-window in-app overlay).
//!
//! Sidebar page list + content pages, replacing the old GTK settings
//! window. Esc or the Back button returns to the main/empty view.

use std::path::PathBuf;

use eframe::egui::{self, Align, Layout, RichText};

use crate::app::{GlintApp, IndexJob, SettingsPage, View};

/// Application version from Cargo.toml at compile time.
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Application name from Cargo.toml at compile time.
const APP_NAME: &str = env!("CARGO_PKG_NAME");
/// Application description from Cargo.toml at compile time.
const APP_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
/// Rust compiler version at compile time.
const RUST_VERSION: &str = env!("CARGO_PKG_RUST_VERSION");
/// Application license from Cargo.toml at compile time.
const APP_LICENSE: &str = env!("CARGO_PKG_LICENSE");

/// Minimum allowed font size.
pub(crate) const FONT_SIZE_MIN: i32 = 8;
/// Maximum allowed font size.
pub(crate) const FONT_SIZE_MAX: i32 = 32;

/// Draws the settings view.
pub fn show(app: &mut GlintApp, ui: &mut egui::Ui) {
    egui::Panel::top("settings_header").show(ui, |ui| {
        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                app.leave_settings();
            }
            ui.heading("Settings");
        });
    });

    egui::Panel::left("settings_nav")
        .resizable(false)
        .exact_size(nav_panel_width(ui))
        .show(ui, |ui| {
            for page in SettingsPage::ALL {
                if ui
                    .selectable_label(app.settings_page == page, page.label())
                    .clicked()
                {
                    app.settings_page = page;
                    if page != SettingsPage::Appearance {
                        // Clicking a nav item while a font change is
                        // pending counts as "clicked outside": apply it.
                        app.commit_font_size();
                    }
                }
            }
        });

    egui::CentralPanel::default().show(ui, |ui| {
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            app.discard_font_size();
            app.leave_settings();
            return;
        }
        egui::ScrollArea::vertical().show(ui, |ui| match app.settings_page {
            SettingsPage::General => general_page(app, ui),
            SettingsPage::Appearance => appearance_page(app, ui),
            SettingsPage::IndexedFolders => indexed_folders_page(app, ui),
            SettingsPage::IgnoredFolders => ignored_folders_page(app, ui),
            SettingsPage::Index => index_page(app, ui),
            SettingsPage::About => about_page(ui),
        });
    });
}

/// Page title + separator.
fn page_header(ui: &mut egui::Ui, title: &str) {
    ui.heading(title);
    ui.separator();
}

/// Width of the settings sidebar for the current body font.
///
/// Sized to fit the longest page label on one line (button frame padding
/// included), so labels never wrap even at the maximum font size (32).
/// The panel is non-resizable with an exact size, so the width tracks
/// font-size changes live in both directions.
fn nav_panel_width(ui: &egui::Ui) -> f32 {
    let style = ui.style();
    let font_id = style
        .text_styles
        .get(&egui::TextStyle::Body)
        .cloned()
        .unwrap_or_else(|| egui::FontId::proportional(14.0));
    // Horizontal room inside a selectable row: frame padding on both
    // sides, plus slack for the selected-frame stroke/expansion.
    let row_padding = style.spacing.button_padding.x * 2.0 + 12.0;
    let widest = SettingsPage::ALL
        .iter()
        .map(|page| measure_label_width(ui, page.label(), &font_id))
        .fold(0.0, f32::max);
    (widest + row_padding).clamp(140.0, 420.0)
}

/// No-wrap pixel width of `text` rendered with `font_id`.
fn measure_label_width(ui: &egui::Ui, text: &str, font_id: &egui::FontId) -> f32 {
    ui.ctx()
        .fonts_mut(|fonts| {
            fonts.layout_no_wrap(text.to_owned(), font_id.clone(), egui::Color32::WHITE)
        })
        .size()
        .x
}

/// Label-value info row.
fn info_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(RichText::new(label).weak().small());
    ui.add(egui::Label::new(value).selectable(true).wrap());
    ui.add_space(6.0);
}

// ── General ────────────────────────────────────────────────────

fn general_page(app: &GlintApp, ui: &mut egui::Ui) {
    page_header(ui, "General");
    let config_display = app
        .service
        .config()
        .index_directory
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(not available)".to_string());
    let index_display = app.service.index_path().display().to_string();

    info_row(ui, "Application", APP_NAME);
    info_row(ui, "Version", APP_VERSION);
    info_row(ui, "Config Directory", &config_display);
    info_row(ui, "Index Directory", &index_display);
}

// ── Appearance ─────────────────────────────────────────────────

fn appearance_page(app: &mut GlintApp, ui: &mut egui::Ui) {
    page_header(ui, "Appearance");

    ui.label(RichText::new("Theme").strong());
    let current = app.service.config().theme;
    for (theme, label) in [
        (glintindex_core::Theme::System, "System"),
        (glintindex_core::Theme::Light, "Light"),
        (glintindex_core::Theme::Dark, "Dark"),
    ] {
        if ui.selectable_label(current == theme, label).clicked() {
            let _ = app.service.set_theme(theme);
            app.applied_theme = None;
        }
    }

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    ui.label(RichText::new("Font Size").strong());
    let current = app.service.config().clamped_font_size() as i32;
    let mut size = app.font_size_draft.unwrap_or(current);
    let slider = ui.add(
        egui::Slider::new(&mut size, FONT_SIZE_MIN..=FONT_SIZE_MAX)
            .text("Application font size (8–32)"),
    );
    if slider.changed() {
        // Stage only: the slider and value box show the draft live,
        // but the font is applied on release / Enter / click-outside.
        app.stage_font_size(size);
    }
    let escaped = ui.input(|i| i.key_pressed(egui::Key::Escape));
    if (slider.drag_stopped() || slider.lost_focus()) && !escaped {
        app.commit_font_size();
    } else if app.font_size_draft.is_some()
        && !slider.has_focus()
        && !slider.dragged()
        && !slider.changed()
    {
        // Self-heal: a draft from an interaction that ended without a
        // release/blur event (e.g. an aborted drag) applies now.
        app.commit_font_size();
    }
}

// ── Indexed Folders ────────────────────────────────────────────

fn indexed_folders_page(app: &mut GlintApp, ui: &mut egui::Ui) {
    page_header(ui, "Indexed Folders");

    if ui.button("Add Folder…").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Select Folder to Index")
            .pick_folder()
        {
            if let Err(e) = app.service.add_folder(&path) {
                tracing::warn!(
                    target: "glintindex::gui",
                    path = %path.display(),
                    error = %e,
                    "add folder failed"
                );
            }
        }
    }

    ui.add_space(8.0);

    let folders: Vec<(PathBuf, bool)> = app
        .service
        .indexed_folders()
        .iter()
        .map(|f| (f.path.clone(), f.enabled))
        .collect();

    if folders.is_empty() {
        ui.weak("No folders configured yet.");
        return;
    }

    for (path, enabled) in folders {
        ui.horizontal(|ui| {
            let badge = if enabled { "Enabled" } else { "Disabled" };
            let badge_color = if enabled {
                ui.visuals().selection.bg_fill
            } else {
                ui.visuals().error_fg_color
            };
            ui.label(RichText::new(badge).color(badge_color).small());
            ui.add(
                egui::Label::new(path.display().to_string())
                    .selectable(true)
                    .truncate(),
            );

            let toggle_label = if enabled { "Disable" } else { "Enable" };
            if ui.button(toggle_label).clicked() {
                let _ = if enabled {
                    app.service.disable_folder(&path)
                } else {
                    app.service.enable_folder(&path)
                };
            }
            if ui.button("Remove").clicked() {
                let _ = app.service.remove_folder(&path);
            }
        });
    }

    ui.add_space(8.0);
    ui.weak("Changes are saved immediately.");
}

// ── Ignored Folders ────────────────────────────────────────────

fn ignored_folders_page(app: &mut GlintApp, ui: &mut egui::Ui) {
    page_header(ui, "Ignored Folders");

    let mut new_name = String::new();
    ui.horizontal(|ui| {
        let response =
            ui.add(egui::TextEdit::singleline(&mut new_name).hint_text("Folder name to ignore"));
        let submitted = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if ui.button("Add").clicked() || submitted {
            let name = new_name.trim().to_string();
            if !name.is_empty() {
                let _ = app.service.add_ignored_folder(name);
            }
        }
    });

    ui.add_space(8.0);

    let names: Vec<String> = app.service.ignored_folders().to_vec();
    if names.is_empty() {
        ui.weak("No ignored folder names.");
        return;
    }

    for name in names {
        ui.horizontal(|ui| {
            ui.label(&name);
            if ui.small_button("Remove").clicked() {
                let _ = app.service.remove_ignored_folder(&name);
            }
        });
    }

    ui.add_space(8.0);
    ui.weak("Changes are saved immediately.");
}

// ── Index ──────────────────────────────────────────────────────

fn index_page(app: &mut GlintApp, ui: &mut egui::Ui) {
    page_header(ui, "Index");

    let mut auto = app.service.config().index_on_startup;
    if ui
        .checkbox(&mut auto, "Index automatically at startup")
        .changed()
    {
        let _ = app.service.set_index_on_startup(auto);
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button("Index All").clicked() {
            app.start_index_job(IndexJob::IndexAll);
        }
        if ui.button("Rebuild Index").clicked() {
            app.start_index_job(IndexJob::Rebuild);
        }
        if ui.button("Clear Index").clicked() {
            app.start_index_job(IndexJob::Clear);
        }
    });

    ui.add_space(8.0);

    if app.service.is_indexing() || app.progress_active {
        ui.horizontal(|ui| {
            ui.spinner();
            if let Some(progress) = app.service.current_progress() {
                ui.label(format!("Processed: {} files", progress.files_processed));
            } else {
                ui.label("Indexing…");
            }
        });
    }

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(RichText::new("Statistics").strong());

    if let Some(stats) = &app.statistics {
        stat_row(ui, "Indexed Documents", stats.indexed_documents);
        stat_row(ui, "Configured Folders", stats.indexed_folders);
    }
    if let Some(progress) = &app.last_job_progress {
        stat_row(ui, "Files Indexed", progress.files_indexed);
        stat_row(ui, "Files Re-indexed", progress.files_reindexed);
        stat_row(ui, "Files Skipped", progress.files_unchanged);
        if progress.files_removed > 0 {
            stat_row(ui, "Files Removed", progress.files_removed);
        }
        if progress.files_failed > 0 {
            stat_row(ui, "Files Failed", progress.files_failed);
        }
        if progress.parser_errors > 0 {
            stat_row(ui, "Parser Errors", progress.parser_errors);
        }
        if progress.parser_panics > 0 {
            stat_row(ui, "Parser Panics", progress.parser_panics);
        }
    }
}

fn stat_row(ui: &mut egui::Ui, label: &str, value: u64) {
    ui.horizontal(|ui| {
        ui.weak(label);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(value.to_string());
        });
    });
}

// ── About ──────────────────────────────────────────────────────

fn about_page(ui: &mut egui::Ui) {
    page_header(ui, "About");
    info_row(ui, "Application", APP_NAME);
    info_row(ui, "Version", APP_VERSION);
    info_row(ui, "Description", APP_DESCRIPTION);
    info_row(ui, "Rust Version", RUST_VERSION);
    info_row(ui, "License", APP_LICENSE);
}

/// Opens settings on a specific page (used by the empty state).
pub fn open_at(app: &mut GlintApp, page: SettingsPage) {
    app.view = View::Settings;
    app.settings_page = page;
}
