//! Theme manager for loading and applying GTK CSS themes.
//!
//! Centralizes all CSS loading and application logic.
//! No other module should load CSS directly.
//!
//! The ThemeManager owns a single `CssProvider` for the entire
//! application lifetime. When the theme changes, CSS is reloaded
//! into the existing provider — no additional providers are created.
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

/// Manages theme loading, application, and switching.
///
/// The ThemeManager creates one `CssProvider` at startup and registers
/// it with the GTK Display. Theme changes reload CSS into the same
/// provider, avoiding provider stacking.
pub struct ThemeManager {
    provider: CssProvider,
}

impl ThemeManager {
    /// Creates a new ThemeManager, registers the CSS provider, and
    /// applies the given theme.
    pub fn new(theme: Theme) -> Self {
        let provider = CssProvider::new();

        // Register the provider once with the GTK Display.
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        } else {
            log::warn!("No GDK display available; theme will not be applied.");
        }

        let manager = Self { provider };
        manager.apply(theme);
        manager
    }

    /// Reloads CSS for the given theme into the existing provider.
    pub fn apply(&self, theme: Theme) {
        let css_content = match theme {
            Theme::Light => LIGHT_CSS,
            Theme::Dark => DARK_CSS,
            Theme::System => {
                if Self::is_dark_mode() {
                    DARK_CSS
                } else {
                    LIGHT_CSS
                }
            }
        };

        self.provider.load_from_data(css_content);
    }

    /// Returns `true` if the operating system prefers dark mode.
    fn is_dark_mode() -> bool {
        gtk::Settings::default()
            .map(|s| s.is_gtk_application_prefer_dark_theme())
            .unwrap_or(false)
    }
}
