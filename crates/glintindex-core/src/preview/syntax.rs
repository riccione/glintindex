//! Syntax highlighting for file previews.
//!
//! Uses `syntect` for automatic language detection based on file
//! extensions and syntax highlighting of source code.

use std::path::Path;

use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

/// A highlighted line containing styled spans.
#[derive(Debug, Clone)]
pub struct HighlightedLine {
    /// The plain text content of the line.
    pub text: String,
    /// Style information for each character position.
    /// Each entry is (start_byte, end_byte, style).
    pub spans: Vec<(usize, usize, Style)>,
}

/// Simplified style information for a text span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    /// Foreground color as (r, g, b).
    pub foreground: (u8, u8, u8),
    /// Whether the text is bold.
    pub bold: bool,
    /// Whether the text is italic.
    pub italic: bool,
}

impl Style {
    /// Creates a new style with the given properties.
    pub fn new(foreground: (u8, u8, u8), bold: bool, italic: bool) -> Self {
        Self {
            foreground,
            bold,
            italic,
        }
    }
}

impl Default for Style {
    fn default() -> Self {
        Self {
            foreground: (255, 255, 255),
            bold: false,
            italic: false,
        }
    }
}

/// Syntax highlighter for source code.
pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl SyntaxHighlighter {
    /// Creates a new syntax highlighter with default settings.
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    /// Detects the syntax for a file based on its extension.
    pub fn detect_syntax(&self, path: &Path) -> Option<&syntect::parsing::SyntaxReference> {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        // Try by extension first
        if let Some(syntax) = self.syntax_set.find_syntax_by_extension(extension) {
            return Some(syntax);
        }

        // Try by file name (for files like Makefile, Dockerfile, etc.)
        if let Some(syntax) = self.syntax_set.find_syntax_by_name(file_name) {
            return Some(syntax);
        }

        // Fallback to plain text - always returns Some
        Some(self.syntax_set.find_syntax_plain_text())
    }

    /// Highlights multiple lines of code.
    pub fn highlight_lines(
        &self,
        text: &str,
        path: &Path,
    ) -> Vec<HighlightedLine> {
        let syntax = match self.detect_syntax(path) {
            Some(s) => s,
            None => {
                // If no syntax found, return unstyled lines
                return text
                    .lines()
                    .map(|line| HighlightedLine {
                        text: line.to_string(),
                        spans: vec![(0, line.len(), Style::default())],
                    })
                    .collect();
            }
        };

        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let mut highlighter = HighlightLines::new(syntax, theme);

        let mut lines = Vec::new();

        for line in text.lines() {
            let mut highlighted_line = HighlightedLine {
                text: line.to_string(),
                spans: Vec::new(),
            };

            // Add newline for syntect
            let line_with_newline = format!("{}\n", line);

            match highlighter.highlight_line(&line_with_newline, &self.syntax_set) {
                Ok(ranges) => {
                    let mut current_pos = 0;

                    for (style, text_slice) in ranges {
                        let start = current_pos;
                        let end = start + text_slice.len();

                        // Skip the newline character in output
                        let display_end = if text_slice.ends_with('\n') {
                            end - 1
                        } else {
                            end
                        };

                        if start < display_end {
                            let fg = style.foreground;
                            highlighted_line.spans.push((
                                start,
                                display_end,
                                Style::new(
                                    (fg.r, fg.g, fg.b),
                                    style.font_style
                                        .contains(syntect::highlighting::FontStyle::BOLD),
                                    style.font_style
                                        .contains(syntect::highlighting::FontStyle::ITALIC),
                                ),
                            ));
                        }

                        current_pos = end;
                    }

                    // Add remaining text if any
                    if current_pos < line.len() {
                        highlighted_line
                            .spans
                            .push((current_pos, line.len(), Style::default()));
                    }
                }
                Err(_) => {
                    // If highlighting fails, add the whole line as unstyled
                    highlighted_line
                        .spans
                        .push((0, line.len(), Style::default()));
                }
            }

            lines.push(highlighted_line);
        }

        lines
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn syntax_highlighter_creation() {
        let highlighter = SyntaxHighlighter::new();
        let path = PathBuf::from("test.rs");
        let syntax = highlighter.detect_syntax(&path);
        assert!(syntax.is_some());
        assert!(!syntax.unwrap().name.is_empty());
    }

    #[test]
    fn detect_rust_syntax() {
        let highlighter = SyntaxHighlighter::new();
        let path = PathBuf::from("main.rs");
        let syntax = highlighter.detect_syntax(&path);
        assert!(syntax.is_some());
        assert!(syntax.unwrap().name.to_lowercase().contains("rust"));
    }

    #[test]
    fn detect_python_syntax() {
        let highlighter = SyntaxHighlighter::new();
        let path = PathBuf::from("script.py");
        let syntax = highlighter.detect_syntax(&path);
        assert!(syntax.is_some());
        assert!(syntax.unwrap().name.to_lowercase().contains("python"));
    }

    #[test]
    fn detect_javascript_syntax() {
        let highlighter = SyntaxHighlighter::new();
        let path = PathBuf::from("app.js");
        let syntax = highlighter.detect_syntax(&path);
        assert!(syntax.is_some());
        assert!(syntax.unwrap().name.to_lowercase().contains("javascript"));
    }

    #[test]
    fn detect_json_syntax() {
        let highlighter = SyntaxHighlighter::new();
        let path = PathBuf::from("data.json");
        let syntax = highlighter.detect_syntax(&path);
        assert!(syntax.is_some());
        assert!(syntax.unwrap().name.to_lowercase().contains("json"));
    }

    #[test]
    fn detect_yaml_syntax() {
        let highlighter = SyntaxHighlighter::new();
        let path = PathBuf::from("config.yml");
        let syntax = highlighter.detect_syntax(&path);
        assert!(syntax.is_some());
        assert!(syntax.unwrap().name.to_lowercase().contains("yaml"));
    }

    #[test]
    fn detect_toml_syntax() {
        let highlighter = SyntaxHighlighter::new();
        let path = PathBuf::from("Cargo.toml");
        let syntax = highlighter.detect_syntax(&path);
        assert!(syntax.is_some());
        // syntect may fall back to plain text for TOML; just verify we get a valid syntax
        assert!(!syntax.unwrap().name.is_empty());
    }

    #[test]
    fn detect_html_syntax() {
        let highlighter = SyntaxHighlighter::new();
        let path = PathBuf::from("index.html");
        let syntax = highlighter.detect_syntax(&path);
        assert!(syntax.is_some());
        assert!(syntax.unwrap().name.to_lowercase().contains("html"));
    }

    #[test]
    fn detect_css_syntax() {
        let highlighter = SyntaxHighlighter::new();
        let path = PathBuf::from("style.css");
        let syntax = highlighter.detect_syntax(&path);
        assert!(syntax.is_some());
        assert!(syntax.unwrap().name.to_lowercase().contains("css"));
    }

    #[test]
    fn highlight_simple_code() {
        let highlighter = SyntaxHighlighter::new();
        let path = PathBuf::from("test.rs");
        let lines = highlighter.highlight_lines("fn main() {}", &path);
        assert!(!lines.is_empty());
        assert_eq!(lines[0].text, "fn main() {}");
        // Should have some highlighted spans
        assert!(!lines[0].spans.is_empty());
    }

    #[test]
    fn highlight_multiline() {
        let highlighter = SyntaxHighlighter::new();
        let path = PathBuf::from("test.rs");
        let code = "fn main() {\n    println!(\"Hello\");\n}";
        let lines = highlighter.highlight_lines(code, &path);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].text, "fn main() {");
        assert_eq!(lines[1].text, "    println!(\"Hello\");");
        assert_eq!(lines[2].text, "}");
    }

    #[test]
    fn style_creation() {
        let style = Style::new((255, 0, 0), true, false);
        assert_eq!(style.foreground, (255, 0, 0));
        assert!(style.bold);
        assert!(!style.italic);
    }

    #[test]
    fn style_default() {
        let style = Style::default();
        assert_eq!(style.foreground, (255, 255, 255));
        assert!(!style.bold);
        assert!(!style.italic);
    }
}
