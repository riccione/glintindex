use std::collections::HashSet;
use std::path::Path;

/// Default directory names that should always be ignored during scanning.
const DEFAULT_IGNORED_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".idea",
    ".vscode",
    ".cache",
    "dist",
    "build",
];

/// Supported file extensions for text file indexing.
const SUPPORTED_EXTENSIONS: &[&str] = &[
    // Plain text formats
    "txt", "md", "log", "json", "yaml", "yml", "toml", "xml", "csv", // Source code
    "rs", "c", "cpp", "h", "hpp", "py", "go", "java", "kt", "js", "jsx", "ts", "tsx", "html", "css",
    "scss", "sql", "sh", // Document formats
    "pdf", "docx", "docm", "xlsx", "xlsm", "xlsb", "xls", "pptx", "pptm", "rtf", "odt",
];

/// Decides whether a path should be skipped during filesystem scanning.
///
/// Combines a set of default ignored directory names with optionally
/// provided custom ignore patterns. The check is purely logical —
/// no filesystem access is performed.
#[derive(Debug, Clone)]
pub struct IgnoreRules {
    ignored_dirs: HashSet<String>,
}

impl IgnoreRules {
    /// Creates `IgnoreRules` with the default ignored directories.
    pub fn new() -> Self {
        Self::with_custom(&[])
    }

    /// Creates `IgnoreRules` with the default directories plus custom names.
    ///
    /// Custom names are merged with the defaults, so passing a name that
    /// already exists in the defaults is harmless.
    pub fn with_custom(custom: &[String]) -> Self {
        let mut ignored_dirs: HashSet<String> =
            DEFAULT_IGNORED_DIRS.iter().map(|s| s.to_string()).collect();
        for name in custom {
            ignored_dirs.insert(name.clone());
        }
        Self { ignored_dirs }
    }

    /// Returns `true` if the given directory name should be skipped.
    pub fn should_ignore_dir(&self, name: &str) -> bool {
        self.ignored_dirs.contains(name)
    }

    /// Returns `true` if the file at `path` has a supported extension.
    pub fn is_supported_file(path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext))
            .unwrap_or(false)
    }
}

impl Default for IgnoreRules {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_rules_contain_common_dirs() {
        let rules = IgnoreRules::new();
        assert!(rules.should_ignore_dir(".git"));
        assert!(rules.should_ignore_dir("target"));
        assert!(rules.should_ignore_dir("node_modules"));
        assert!(rules.should_ignore_dir(".idea"));
        assert!(rules.should_ignore_dir(".vscode"));
        assert!(rules.should_ignore_dir(".cache"));
        assert!(rules.should_ignore_dir("dist"));
        assert!(rules.should_ignore_dir("build"));
    }

    #[test]
    fn default_rules_do_not_ignore_src() {
        let rules = IgnoreRules::new();
        assert!(!rules.should_ignore_dir("src"));
        assert!(!rules.should_ignore_dir("crates"));
    }

    #[test]
    fn custom_rules_merge() {
        let custom = vec!["my_custom_dir".to_string(), ".git".to_string()];
        let rules = IgnoreRules::with_custom(&custom);
        assert!(rules.should_ignore_dir("my_custom_dir"));
        assert!(rules.should_ignore_dir(".git"));
    }

    #[test]
    fn supported_extensions() {
        assert!(IgnoreRules::is_supported_file(Path::new("main.rs")));
        assert!(IgnoreRules::is_supported_file(Path::new("index.html")));
        assert!(IgnoreRules::is_supported_file(Path::new("style.css")));
        assert!(IgnoreRules::is_supported_file(Path::new("readme.md")));
        assert!(IgnoreRules::is_supported_file(Path::new("data.json")));
    }

    #[test]
    fn unsupported_extensions() {
        assert!(!IgnoreRules::is_supported_file(Path::new("image.png")));
        assert!(!IgnoreRules::is_supported_file(Path::new("binary")));
        assert!(!IgnoreRules::is_supported_file(Path::new("archive.zip")));
    }
}
