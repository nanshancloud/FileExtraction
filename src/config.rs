use crate::i18n::Lang;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// File extensions scanned by default
pub const DEFAULT_EXTENSIONS: &[&str] =
    &["doc", "docx", "ppt", "pptx", "xls", "xlsx", "txt", "pdf", "md"];

/// Application configuration after loading
pub struct Config {
    pub custom_extensions: Vec<String>,
    pub selected: HashSet<String>,
    pub keep_structure: bool,
    /// None means first run: the language picker dialog must be shown
    pub language: Option<Lang>,
}

#[derive(Serialize, Deserialize, Default)]
struct AppConfig {
    custom_extensions: Vec<String>,
    selected_extensions: Vec<String>,
    #[serde(default)]
    keep_structure: bool,
    #[serde(default)]
    language: Option<Lang>,
}

/// Config file path: stored in the same directory as the executable
pub fn config_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("config.json")
}

/// Load config; falls back to defaults when the file is missing or corrupt
/// (all default formats selected)
pub fn load(path: &Path) -> Config {
    let cfg = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<AppConfig>(&s).ok())
        .unwrap_or_default();

    let defaults: Vec<String> = DEFAULT_EXTENSIONS.iter().map(|s| s.to_string()).collect();
    let mut selected: HashSet<String> = cfg
        .selected_extensions
        .iter()
        .filter(|e| defaults.contains(e) || cfg.custom_extensions.contains(e))
        .cloned()
        .collect();
    if selected.is_empty() {
        selected.extend(defaults.iter().cloned());
    }
    let mut custom = cfg.custom_extensions.clone();
    custom.retain(|c| !defaults.contains(c));

    Config {
        custom_extensions: custom,
        selected,
        keep_structure: cfg.keep_structure,
        language: cfg.language,
    }
}

/// Save config to a json file
pub fn save(path: &Path, cfg: &Config) {
    let out = AppConfig {
        custom_extensions: cfg.custom_extensions.clone(),
        selected_extensions: cfg.selected.iter().cloned().collect(),
        keep_structure: cfg.keep_structure,
        language: cfg.language,
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&out) {
        let _ = std::fs::write(path, json);
    }
}
