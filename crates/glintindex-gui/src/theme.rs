//! Theme application for the egui frontend.
//!
//! Maps [`glintindex_core::Theme`] to egui `Visuals` (light/dark) and
//! applies the configured application font size. `Theme::System` follows
//! the OS preference reported by the egui context each frame.
//!
//! egui 0.36 keeps separate light/dark styles on the context, so both
//! are configured up front and the active theme is picked by
//! [`egui::Context::set_theme`] semantics (System preference).

use eframe::egui::{self, FontFamily, FontId, TextStyle};

use glintindex_core::Theme;

/// Warm-yellow match-highlight background — readable on light and dark
/// visuals (same tone the GTK frontend used).
pub const MATCH_BACKGROUND: egui::Color32 = egui::Color32::from_rgb(255, 214, 51);

/// Resolves `Theme::System` against the OS preference of the context.
pub fn resolve_theme(ctx: &egui::Context, theme: Theme) -> Theme {
    match theme {
        Theme::System => {
            if ctx.system_theme() == Some(egui::Theme::Dark) {
                Theme::Dark
            } else {
                Theme::Light
            }
        }
        other => other,
    }
}

/// Applies the given theme and font size to the egui context.
pub fn apply_theme(ctx: &egui::Context, theme: Theme, font_size: u32) {
    let resolved = resolve_theme(ctx, theme);
    ctx.set_theme(match resolved {
        Theme::Dark => egui::Theme::Dark,
        _ => egui::Theme::Light,
    });

    let visuals = match resolved {
        Theme::Dark => egui::Visuals::dark(),
        _ => egui::Visuals::light(),
    };
    // Configure both slots so switching themes later needs no re-apply.
    ctx.set_visuals_of(egui::Theme::Dark, visuals.clone());
    ctx.set_visuals_of(egui::Theme::Light, visuals);

    let size = font_size as f32;
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        ctx.style_mut_of(theme, |style| {
            style
                .text_styles
                .insert(TextStyle::Body, FontId::new(size, FontFamily::Proportional));
            style.text_styles.insert(
                TextStyle::Small,
                FontId::new(size * 0.875, FontFamily::Proportional),
            );
            style.text_styles.insert(
                TextStyle::Heading,
                FontId::new(size * 1.35, FontFamily::Proportional),
            );
            style.text_styles.insert(
                TextStyle::Monospace,
                FontId::new(size, FontFamily::Monospace),
            );
        });
    }
}
