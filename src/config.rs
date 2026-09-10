use crate::i18n::Lang;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

/// Built-in extension categories shown in the settings panel
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExtCategory {
    /// Default formats: everyday office / plain text formats
    DefaultFormat,
    /// Report template formats
    ReportTemplate,
}

impl ExtCategory {
    pub const ALL: [ExtCategory; 2] = [ExtCategory::DefaultFormat, ExtCategory::ReportTemplate];

    /// Stable identifier used for config keys and collapse-state ids.
    /// Independent of the display label, so switching language keeps state.
    pub fn id(self) -> &'static str {
        match self {
            ExtCategory::DefaultFormat => "cat_default",
            ExtCategory::ReportTemplate => "cat_report_template",
        }
    }

    /// Built-in extensions belonging to this category
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            ExtCategory::DefaultFormat => &[
                "doc", "docx", "ppt", "pptx", "xls", "xlsx", "txt", "pdf", "md",
            ],
            ExtCategory::ReportTemplate => &["det2app"],
        }
    }
}

/// All built-in extensions across every category
pub fn default_extensions() -> Vec<&'static str> {
    ExtCategory::ALL
        .iter()
        .flat_map(|c| c.extensions().iter().copied())
        .collect()
}

/// Application configuration after loading
pub struct Config {
    /// User-added extensions per category id
    pub custom_extensions: BTreeMap<String, Vec<String>>,
    pub selected: HashSet<String>,
    pub keep_structure: bool,
    /// None means first run: the language picker dialog must be shown
    pub language: Option<Lang>,
}

/// Custom extensions as stored on disk, accepting both the legacy flat list and
/// the current per-category map so older config files keep working
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum CustomExts {
    Map(BTreeMap<String, Vec<String>>),
    Legacy(Vec<String>),
}

impl Default for CustomExts {
    fn default() -> Self {
        CustomExts::Map(BTreeMap::new())
    }
}

#[derive(Serialize, Deserialize, Default)]
struct AppConfig {
    #[serde(default)]
    custom_extensions: CustomExts,
    #[serde(default)]
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
/// (all built-in formats selected)
pub fn load(path: &Path) -> Config {
    let cfg = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<AppConfig>(&s).ok())
        .unwrap_or_default();

    let builtins: HashSet<String> = default_extensions().iter().map(|s| s.to_string()).collect();

    // Migrate the legacy flat list into the default-format category
    let mut custom: BTreeMap<String, Vec<String>> = match cfg.custom_extensions {
        CustomExts::Map(map) => map,
        CustomExts::Legacy(list) => {
            let mut map = BTreeMap::new();
            if !list.is_empty() {
                map.insert(ExtCategory::DefaultFormat.id().to_string(), list);
            }
            map
        }
    };

    // Drop duplicates of built-in formats and entries repeated across categories
    let mut seen: HashSet<String> = HashSet::new();
    custom.retain(|cat_id, list| {
        if !ExtCategory::ALL.iter().any(|c| c.id() == cat_id) {
            return false;
        }
        list.retain(|e| !e.is_empty() && !builtins.contains(e) && seen.insert(e.clone()));
        !list.is_empty()
    });

    let custom_all: HashSet<String> = custom.values().flatten().cloned().collect();
    let mut selected: HashSet<String> = cfg
        .selected_extensions
        .iter()
        .filter(|e| builtins.contains(*e) || custom_all.contains(*e))
        .cloned()
        .collect();
    if selected.is_empty() {
        selected = builtins.clone();
    }

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
        custom_extensions: CustomExts::Map(cfg.custom_extensions.clone()),
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
