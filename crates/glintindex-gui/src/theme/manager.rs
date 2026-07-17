//! Theme manager for loading and applying GTK CSS themes.
//!
//! Centralizes all CSS loading and application logic.
//! No other module should load CSS directly.
//!
//! The ThemeManager owns a single `CssProvider` for the entire
//! application lifetime. When the theme or font size changes,
//! CSS is regenerated and reloaded into the existing provider —
//! no additional providers are created.
//!
//! All CSS files are embedded in the binary via `include_str!()` so
//! the application always has access to the stylesheets regardless
//! of how it is run (cargo run, installed binary, etc.).

use gtk::CssProvider;

use glintindex_core::Theme;

/// Embedded light theme CSS.
const LIGHT_CSS: &str = include_str!("../../resources/themes/light.css");

/// Embedded dark theme CSS.
const DARK_CSS: &str = include_str!("../../resources/themes/dark.css");

/// Generates the complete CSS by combining theme styles with font rules.
fn generate_css(theme_css: &str, font_size: u32) -> String {
    format!("{theme_css}\n* {{ font-size: {font_size}pt; }}")
}

/// Returns the embedded CSS for the given theme.
fn theme_css(theme: Theme) -> &'static str {
    match theme {
        Theme::Light => LIGHT_CSS,
        Theme::Dark => DARK_CSS,
        Theme::System => {
            if is_dark_mode() {
                DARK_CSS
            } else {
                LIGHT_CSS
            }
        }
    }
}

/// Returns `true` if the operating system prefers dark mode.
fn is_dark_mode() -> bool {
    gtk::Settings::default()
        .map(|s| s.is_gtk_application_prefer_dark_theme())
        .unwrap_or(false)
}

/// Manages theme loading, application, and switching.
///
/// The ThemeManager creates one `CssProvider` at startup and registers
/// it with the GTK Display. Theme and font size changes regenerate CSS
/// and reload it into the same provider, avoiding provider stacking.
pub struct ThemeManager {
    provider: CssProvider,
    #[allow(dead_code)]
    current_theme: Theme,
}

impl ThemeManager {
    /// Creates a new ThemeManager, registers the CSS provider, and
    /// applies the given theme with the specified font size.
    pub fn new(theme: Theme, font_size: u32) -> Self {
        let provider = CssProvider::new();

        // Register the provider once with the GTK Display.
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        } else {
            tracing::warn!(
                target: "glintindex::gui",
                "no GDK display available; theme will not be applied"
            );
        }

        let manager = Self {
            provider,
            current_theme: theme,
        };
        manager.apply(theme, font_size);
        manager
    }

    /// Regenerates and reloads CSS for the given theme and font size.
    ///
    /// Combines the theme stylesheet with font size rules into a single
    /// CSS payload and loads it into the existing provider.
    pub fn apply(&self, theme: Theme, font_size: u32) {
        // 1. Force GTK's built-in components (like titlebars/searchbars) to prefer dark mode
        if let Some(settings) = gtk::Settings::default() {
            let prefer_dark = match theme {
                Theme::Dark => true,
                Theme::Light => false,
                Theme::System => is_dark_mode(),
            };
            settings.set_gtk_application_prefer_dark_theme(prefer_dark);
        }

        let css = generate_css(theme_css(theme), font_size);
        self.provider.load_from_data(&css);
    }

    /// Returns the currently applied theme.
    #[allow(dead_code)]
    pub fn current_theme(&self) -> Theme {
        self.current_theme
    }
}
