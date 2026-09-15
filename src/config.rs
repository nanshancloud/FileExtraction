use crate::i18n::{Lang, Tr};
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
    /// Archive / compressed file formats
    Archive,
}

impl ExtCategory {
    pub const ALL: [ExtCategory; 3] = [
        ExtCategory::DefaultFormat,
        ExtCategory::ReportTemplate,
        ExtCategory::Archive,
    ];

    /// Stable identifier used for config keys and collapse-state ids.
    /// Independent of the display label, so switching language keeps state.
    pub fn id(self) -> &'static str {
        match self {
            ExtCategory::DefaultFormat => "cat_default",
            ExtCategory::ReportTemplate => "cat_report_template",
            ExtCategory::Archive => "cat_archive",
        }
    }

    /// Built-in extensions belonging to this category.
    /// Compound extensions such as "tar.gz" are supported as well.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            ExtCategory::DefaultFormat => &[
                "doc", "docx", "ppt", "pptx", "xls", "xlsx", "txt", "pdf", "md",
            ],
            ExtCategory::ReportTemplate => &["det2app"],
            ExtCategory::Archive => &["7z", "rar", "zip", "tar", "tar.gz", "tar.xz"],
        }
    }
}

/// A column of the result table and of the exported list
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColumnId {
    /// Row number
    No,
    Name,
    Path,
    /// Human readable file size
    Size,
    /// Last modification time
    Modified,
    /// Raw size in bytes
    Bytes,
}

impl ColumnId {
    /// Default column order; also used to complete a partial saved config
    pub const ALL: [ColumnId; 6] = [
        ColumnId::No,
        ColumnId::Name,
        ColumnId::Path,
        ColumnId::Size,
        ColumnId::Modified,
        ColumnId::Bytes,
    ];

    /// Display name of the column in the current language
    pub fn label(self, tr: &Tr) -> &'static str {
        match self {
            ColumnId::No => tr.col_no,
            ColumnId::Name => tr.col_name,
            ColumnId::Path => tr.col_path,
            ColumnId::Size => tr.col_size,
            ColumnId::Modified => tr.col_modified,
            ColumnId::Bytes => tr.excel_bytes,
        }
    }

    /// Whether the column is shown for a fresh configuration
    pub fn default_visible(self) -> bool {
        !matches!(self, ColumnId::Bytes)
    }

    /// Initial width of the column in the result table (in points)
    pub fn default_width(self) -> f32 {
        match self {
            ColumnId::No => 50.0,
            ColumnId::Name => 200.0,
            ColumnId::Path => 420.0,
            ColumnId::Size => 110.0,
            ColumnId::Modified => 160.0,
            ColumnId::Bytes => 110.0,
        }
    }

    /// Narrowest width the user can drag a column to
    pub fn min_width(self) -> f32 {
        match self {
            ColumnId::No => 36.0,
            _ => 60.0,
        }
    }
}

/// One managed column: which one it is, whether it is shown, and how wide it is
#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct ColumnDef {
    pub id: ColumnId,
    #[serde(default = "default_visible")]
    pub visible: bool,
    /// Width set by the user by dragging the column separator; `None` = default
    #[serde(default)]
    pub width: Option<f32>,
}

impl ColumnDef {
    /// Effective width: the user defined one, or the default for this column
    pub fn width(self) -> f32 {
        self.width.unwrap_or_else(|| self.id.default_width())
    }

    /// Remember a width; values outside the allowed range are ignored
    pub fn set_width(&mut self, width: f32) {
        let min = self.id.min_width();
        if width.is_finite() && width >= min {
            self.width = Some(width.max(min).min(2000.0));
        }
    }
}

/// Unit used to display file sizes. `Auto` picks the best fitting unit for
/// every single file, so the unit switches automatically with the file size.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SizeUnit {
    /// B / KB / MB ... chosen per file size
    #[default]
    Auto,
    B,
    Kb,
    Mb,
    Gb,
    Tb,
}

impl SizeUnit {
    pub const ALL: [SizeUnit; 6] = [
        SizeUnit::Auto,
        SizeUnit::B,
        SizeUnit::Kb,
        SizeUnit::Mb,
        SizeUnit::Gb,
        SizeUnit::Tb,
    ];

    /// Unit name shown in the settings and appended to the column header
    pub fn suffix(self) -> &'static str {
        match self {
            SizeUnit::Auto => "",
            SizeUnit::B => "B",
            SizeUnit::Kb => "KB",
            SizeUnit::Mb => "MB",
            SizeUnit::Gb => "GB",
            SizeUnit::Tb => "TB",
        }
    }

    /// Display name in the current language ("Auto" is translated, units are not)
    pub fn label(self, tr: &Tr) -> &'static str {
        match self {
            SizeUnit::Auto => tr.size_auto,
            other => other.suffix(),
        }
    }

    /// Divisor applied to a byte count; `Auto` is handled per file instead
    pub fn divisor(self) -> f64 {
        match self {
            SizeUnit::Auto | SizeUnit::B => 1.0,
            SizeUnit::Kb => 1024.0,
            SizeUnit::Mb => 1024.0 * 1024.0,
            SizeUnit::Gb => 1024.0 * 1024.0 * 1024.0,
            SizeUnit::Tb => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        }
    }
}

/// How the file path is shown in the result table and in the exported list
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PathMode {
    /// Full path as the file system reports it
    #[default]
    Absolute,
    /// Path relative to the scan root, the root itself written as "/"
    Relative,
}

impl PathMode {
    pub const ALL: [PathMode; 2] = [PathMode::Absolute, PathMode::Relative];

    /// Display name in the current language
    pub fn label(self, tr: &Tr) -> &'static str {
        match self {
            PathMode::Absolute => tr.path_absolute,
            PathMode::Relative => tr.path_relative,
        }
    }
}

fn default_visible() -> bool {
    true
}

/// Default column setup of the result table and the exported list
fn default_columns() -> Vec<ColumnDef> {
    ColumnId::ALL
        .iter()
        .map(|id| ColumnDef {
            id: *id,
            visible: id.default_visible(),
            width: None,
        })
        .collect()
}

/// Drop unknown/duplicate entries and append columns that are missing, so that
/// older or hand-edited config files keep working
pub fn migrate_columns(columns: Vec<ColumnDef>) -> Vec<ColumnDef> {
    let mut out: Vec<ColumnDef> = Vec::with_capacity(ColumnId::ALL.len());
    for c in columns {
        if !out.iter().any(|x| x.id == c.id) {
            out.push(c);
        }
    }
    for id in ColumnId::ALL.iter() {
        if !out.iter().any(|c| c.id == *id) {
            out.push(ColumnDef {
                id: *id,
                visible: id.default_visible(),
                width: None,
            });
        }
    }
    out
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
    /// Column order and visibility of the result table and the exported list
    pub columns: Vec<ColumnDef>,
    /// Restore the last scan directory (and rescan it) when the app starts
    pub open_last_dir: bool,
    /// Directory used by the last scan; empty when the app has never scanned
    pub last_dir: String,
    /// Unit used to display file sizes (Auto switches per file size)
    pub size_unit: SizeUnit,
    /// Whether paths are shown absolute or relative to the scan root
    pub path_mode: PathMode,
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
    #[serde(default = "default_columns")]
    columns: Vec<ColumnDef>,
    #[serde(default = "default_true")]
    open_last_dir: bool,
    #[serde(default)]
    last_dir: String,
    #[serde(default)]
    size_unit: SizeUnit,
    #[serde(default)]
    path_mode: PathMode,
}

fn default_true() -> bool {
    true
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
        columns: migrate_columns(cfg.columns),
        open_last_dir: cfg.open_last_dir,
        last_dir: cfg.last_dir,
        size_unit: cfg.size_unit,
        path_mode: cfg.path_mode,
    }
}

/// Save config to a json file
pub fn save(path: &Path, cfg: &Config) {
    let out = AppConfig {
        custom_extensions: CustomExts::Map(cfg.custom_extensions.clone()),
        selected_extensions: cfg.selected.iter().cloned().collect(),
        keep_structure: cfg.keep_structure,
        language: cfg.language,
        columns: cfg.columns.clone(),
        open_last_dir: cfg.open_last_dir,
        last_dir: cfg.last_dir.clone(),
        size_unit: cfg.size_unit,
        path_mode: cfg.path_mode,
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&out) {
        let _ = std::fs::write(path, json);
    }
}
