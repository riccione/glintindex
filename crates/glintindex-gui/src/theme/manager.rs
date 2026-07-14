//! Theme manager for loading and applying GTK CSS themes.
//!
//! Centralizes all CSS loading and application logic.
//! No other module should load CSS directly.

use gtk::CssProvider;

use glintindex_core::Theme;

/// Embedded default CSS used as a fallback when external CSS cannot be loaded.
const DEFAULT_CSS: &str = include_str!("../../resources/themes/light.css");

/// Manages theme loading, application, and switching.
///
/// The ThemeManager loads CSS files from the resources directory and
/// applies them to the GTK application. It handles fallback gracefully
/// if CSS files are missing or contain errors.
pub struct ThemeManager {
    provider: CssProvider,
}

impl ThemeManager {
    /// Creates a new ThemeManager and applies the given theme.
    pub fn new(theme: Theme) -> Self {
        let provider = CssProvider::new();
        let manager = Self { provider };
        manager.apply(theme);
        manager
    }

    /// Applies the given theme to the application.
    ///
    /// Loads CSS from the resources directory. Falls back to the
    /// embedded default CSS if the file cannot be loaded.
    pub fn apply(&self, theme: Theme) {
        let css_content = match theme {
            Theme::Light => Self::load_css("light.css"),
            Theme::Dark => Self::load_css("dark.css"),
            Theme::System => {
                // For System theme, try to detect the system preference.
                // GTK4 provides this via the settings manager, but for
                // simplicity we fall back to the light theme.
                Self::load_css("light.css")
            }
        };

        self.apply_css(&css_content);
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

    /// Applies CSS content to the application.
    fn apply_css(&self, css_content: &str) {
        self.provider.load_from_data(css_content);

        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().expect("Could not connect to a display"),
            &self.provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
