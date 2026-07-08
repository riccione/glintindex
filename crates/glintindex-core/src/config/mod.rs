use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub index_path: String,
    pub watched_paths: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            index_path: "~/.local/share/glintindex/index".into(),
            watched_paths: Vec::new(),
        }
    }
}
