//! Theme manager for loading and applying GTK CSS themes.
//!
//! Centralizes all CSS loading and application logic.
//! No other module should load CSS directly.
//!
//! The ThemeManager owns a single `CssProvider` for the entire
//! application lifetime. When the theme changes, CSS is reloaded
//! into the existing provider — no additional providers are created.

use gtk::CssProvider;

use glintindex_core::Theme;

/// Embedded default CSS used as a fallback when external CSS cannot be loaded.
const DEFAULT_CSS: &str = include_str!("../../resources/themes/light.css");

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
        // If no display is available (e.g., headless), skip registration.
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
    ///
    /// Loads CSS from the resources directory. Falls back to the
    /// embedded default CSS if the file cannot be loaded.
    pub fn apply(&self, theme: Theme) {
        let css_content = match theme {
            Theme::Light => Self::load_css("light.css"),
            Theme::Dark => Self::load_css("dark.css"),
            Theme::System => {
                // For System theme, detect the OS preference and
                // load the corresponding stylesheet.
                if Self::is_dark_mode() {
                    Self::load_css("dark.css")
                } else {
                    Self::load_css("light.css")
                }
            }
        };

        // Reload CSS into the existing provider (no new provider created)
        self.provider.load_from_data(&css_content);
    }

    /// Returns `true` if the operating system prefers dark mode.
    fn is_dark_mode() -> bool {
        gtk::Settings::default()
            .map(|s| s.is_gtk_application_prefer_dark_theme())
            .unwrap_or(false)
    }

    /// Loads CSS content from the themes directory.
    ///
    /// Falls back to the embedded default CSS if the file cannot be read.
    fn load_css(filename: &str) -> String {
        // Try to load from the themes directory relative to the executable
        let themes_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("resources/themes")))
            .unwrap_or_else(|| std::path::PathBuf::from("resources/themes"));

        let css_path = themes_dir.join(filename);
        match std::fs::read_to_string(&css_path) {
            Ok(content) => content,
            Err(e) => {
                log::warn!(
                    "Failed to load CSS from {}: {}. Using embedded default.",
                    css_path.display(),
                    e
                );
                DEFAULT_CSS.to_string()
            }
        }
    }
}
