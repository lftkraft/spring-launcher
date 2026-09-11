use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub launcher_dir: String,
    pub default_memory: u32,
    pub default_java: Option<String>,
    pub theme: String,
    pub language: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            launcher_dir: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("SpringLauncher")
                .to_string_lossy()
                .to_string(),
            default_memory: 2048,
            default_java: None,
            theme: "dark".to_string(),
            language: "hu".to_string(),
        }
    }
}