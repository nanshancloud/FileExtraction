use crate::config::{ColumnDef, ColumnId, PathMode, SizeUnit};
use crate::i18n::{tf, Tr};
use crate::scanner::FileEntry;
use rust_xlsxwriter::{Format, FormatAlign, Workbook, XlsxError};
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

/// Summary of an export run (the UI layer turns it into a status bar message)
pub struct ExportResult {
    pub copied: usize,
    pub total: usize,
    /// Source paths that failed to copy
    pub failed: Vec<String>,
    /// Path of the generated list file (on success)
    pub list_path: Option<PathBuf>,
    /// Reason why the Excel list failed to export
    pub list_error: Option<String>,
    /// True when only the Excel list was generated and no document was copied
    pub list_only: bool,
}

/// Copy all scanned documents into the export directory and generate the Excel
/// list. When `keep_structure` is true, sub-directories are recreated following
/// the original layout; otherwise files are exported flat with automatic
/// renaming on name conflicts.
pub fn export_files(
    entries: &[FileEntry],
    scan_dir: &str,
    out_dir: &Path,
    keep_structure: bool,
    columns: &[ColumnDef],
    unit: SizeUnit,
    path_mode: PathMode,
    tr: &Tr,
) -> ExportResult {
    let scan_root = PathBuf::from(scan_dir);
    let mut used_names: HashSet<String> = HashSet::new();
    let mut copied = 0usize;
    let mut failed: Vec<String> = Vec::new();

    for e in entries {
        let src = PathBuf::from(&e.path);
        let file_name = src.file_name().map(|n| n.to_string_lossy().to_string());
        let Some(file_name) = file_name else {
            failed.push(e.path.clone());
            continue;
        };

        let dest = if keep_structure {
            dest_with_structure(&src, &scan_root, out_dir, &file_name)
        } else {
            dest_flat(&src, out_dir, &file_name, &mut used_names)
        };

        match std::fs::copy(&src, &dest) {
            Ok(_) => copied += 1,
            Err(_) => failed.push(e.path.clone()),
        }
    }

    // Extra artifact: generate the Excel list in the same directory
    let list_path = out_dir.join(tr.excel_filename);
    let excel_result = write_excel(entries, &scan_root, &list_path, columns, unit, path_mode, tr);

    let (list_path, list_error) = match excel_result {
        Ok(()) => (Some(list_path), None),
        Err(e) => (None, Some(e.to_string())),
    };

    ExportResult {
        copied,
        total: entries.len(),
        failed,
        list_path,
        list_error,
        list_only: false,
    }
}

/// Generate the Excel list only, without copying any document.
/// The list is written into `out_dir` and carries the auto detected directory
/// levels of every file.
pub fn export_list_only(
    entries: &[FileEntry],
    scan_dir: &str,
    out_dir: &Path,
    columns: &[ColumnDef],
    unit: SizeUnit,
    path_mode: PathMode,
    tr: &Tr,
) -> ExportResult {
    let scan_root = PathBuf::from(scan_dir);
    let list_path = out_dir.join(tr.excel_filename);
    let excel_result = write_excel(entries, &scan_root, &list_path, columns, unit, path_mode, tr);

    let (list_path, list_error) = match excel_result {
        Ok(()) => (Some(list_path), None),
        Err(e) => (None, Some(e.to_string())),
    };

    ExportResult {
        copied: 0,
        total: entries.len(),
        failed: Vec::new(),
        list_path,
        list_error,
        list_only: true,
    }
}

/// Structured export: rebuild the path relative to the scan root, renaming on
/// name conflicts
fn dest_with_structure(src: &Path, scan_root: &Path, out_dir: &Path, file_name: &str) -> PathBuf {
    let rel = src
        .strip_prefix(scan_root)
        .map(|r| r.to_path_buf())
        .unwrap_or_else(|_| PathBuf::from(file_name));
    let dest = out_dir.join(&rel);
    if let Some(parent) = dest.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut d = dest.clone();
    let stem = src
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = src
        .extension()
        .map(|x| format!(".{}", x.to_string_lossy()))
        .unwrap_or_default();
    let mut n = 2usize;
    while d.exists() {
        d = dest
            .parent()
            .map(|p| p.join(format!("{stem} ({n}){ext}")))
            .unwrap_or_else(|| PathBuf::from(format!("{stem} ({n}){ext}")));
        n += 1;
    }
    d
}

/// Flat export: all files in one level; on name conflicts a unique name of the
/// form "name (2).ext" is generated
fn dest_flat(src: &Path, out_dir: &Path, file_name: &str, used_names: &mut HashSet<String>) -> PathBuf {
    let stem = src
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = src
        .extension()
        .map(|x| format!(".{}", x.to_string_lossy()))
        .unwrap_or_default();
    let mut dest_name = file_name.to_string();
    let mut n = 2usize;
    while used_names.contains(&dest_name) || out_dir.join(&dest_name).exists() {
        dest_name = format!("{stem} ({n}){ext}");
        n += 1;
    }
    used_names.insert(dest_name.clone());
    out_dir.join(&dest_name)
}

/// One row of the Excel list: the file plus the directory levels it lives in
struct LeveledRow<'a> {
    entry: &'a FileEntry,
    /// Directory names from the scan root down to the file's parent directory
    levels: Vec<String>,
}

/// Split the parent directory of a file into levels relative to the scan root.
/// A file sitting directly in the scan root has no level at all; a file outside
/// of the scan root also yields no level (its path column still shows the truth).
fn dir_levels(path: &str, scan_root: &Path) -> Vec<String> {
    let p = PathBuf::from(path);
    let parent = match p.parent() {
        Some(parent) => parent,
        None => return Vec::new(),
    };
    let rel = parent.strip_prefix(scan_root).unwrap_or(Path::new(""));
    rel.components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().to_string()),
            _ => None,
        })
        .collect()
}

/// Build the list rows. The list is always sorted A-Z by file name, whatever
/// order the app shows the results in: sorting a column only rearranges the table
/// of the scan results, never the exported list. Files that stay next to each
/// other and share a directory level get their level cell merged below.
fn build_rows<'a>(entries: &'a [FileEntry], scan_root: &Path) -> Vec<LeveledRow<'a>> {
    let mut rows: Vec<LeveledRow<'a>> = entries
        .iter()
        .map(|e| LeveledRow {
            entry: e,
            levels: dir_levels(&e.path, scan_root),
        })
        .collect();
    rows.sort_by(|a, b| {
        a.entry
            .name
            .to_lowercase()
            .cmp(&b.entry.name.to_lowercase())
            .then_with(|| a.entry.path.cmp(&b.entry.path))
    });
    rows
}

/// Generate the Excel list. The layout follows the configured columns (hidden
/// ones are skipped) and adapts to the real directory depth: the level columns
/// are inserted right behind the row number, and each directory name is written
/// once and merged over the rows that share it.
fn write_excel(
    entries: &[FileEntry],
    scan_root: &Path,
    path: &Path,
    columns: &[ColumnDef],
    unit: SizeUnit,
    path_mode: PathMode,
    tr: &Tr,
) -> Result<(), XlsxError> {
    let rows = build_rows(entries, scan_root);
    let max_level = rows.iter().map(|r| r.levels.len()).max().unwrap_or(0);

    // Visible columns in the configured order
    let managed: Vec<ColumnDef> = columns.iter().copied().filter(|c| c.visible).collect();
    // Directory levels sit behind the row number, or in front when it is hidden
    let level_at = managed
        .iter()
        .position(|c| c.id == ColumnId::No)
        .map(|i| i + 1)
        .unwrap_or(0);
    let column_at = |i: usize| -> u16 {
        if i < level_at {
            i as u16
        } else {
            (i + max_level) as u16
        }
    };

    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.set_name(tr.sheet_name)?;

    let header = Format::new()
        .set_bold()
        .set_background_color("#D9E1F2")
        .set_align(FormatAlign::Center);

    // Level cells are merged, so the text must be vertically centered
    let level_fmt = Format::new()
        .set_align(FormatAlign::VerticalCenter)
        .set_align(FormatAlign::Left);

    // The size column carries its unit once a fixed unit is configured
    let size_header = format!("{}{}", tr.col_size, unit_header(unit));
    for (i, c) in managed.iter().enumerate() {
        let title = match c.id {
            ColumnId::Size => size_header.as_str(),
            _ => c.id.label(tr),
        };
        sheet.write_with_format(0u32, column_at(i), title, &header)?;
    }
    for lvl in 0..max_level {
        let title = tf(tr.level_header, &[&(lvl + 1)]);
        sheet.write_with_format(0u32, (level_at + lvl) as u16, &title, &header)?;
    }

    // File rows: every visible column in the configured order
    for (i, r) in rows.iter().enumerate() {
        let row = (i + 1) as u32;
        for (j, c) in managed.iter().enumerate() {
            let col = column_at(j);
            match c.id {
                ColumnId::No => sheet.write_number(row, col, (i + 1) as f64)?,
                ColumnId::Name => sheet.write_string(row, col, &r.entry.name)?,
                // Same path display as the results table: absolute or relative
                ColumnId::Path => {
                    let shown = match path_mode {
                        PathMode::Absolute => r.entry.path.as_str(),
                        PathMode::Relative => r.entry.rel_path.as_str(),
                    };
                    sheet.write_string(row, col, shown)?
                }
                ColumnId::Size => {
                    sheet.write_string(row, col, &format_size(r.entry.size, unit))?
                }
                ColumnId::Bytes => sheet.write_number(row, col, r.entry.size as f64)?,
                ColumnId::Modified => {
                    sheet.write_string(row, col, &format_time(r.entry.modified))?
                }
            };
        }
    }

    // Directory levels: a name is written once and merged over every following
    // row that belongs to the same directory
    for lvl in 0..max_level {
        let col = (level_at + lvl) as u16;
        let mut start = 0usize;
        while start < rows.len() {
            let Some(name) = rows[start].levels.get(lvl) else {
                start += 1;
                continue;
            };
            let mut end = start;
            while end + 1 < rows.len() && rows[end + 1].levels.get(lvl) == Some(name) {
                end += 1;
            }
            let first = (start + 1) as u32;
            let last = (end + 1) as u32;
            if last > first {
                sheet.merge_range(first, col, last, col, name.as_str(), &level_fmt)?;
            } else {
                sheet.write_with_format(first, col, name.as_str(), &level_fmt)?;
            }
            start = end + 1;
        }
    }

    // Levels the file does not reach (a file of the scan root has none at all)
    // stay empty, so a centered "/" marks "nothing below this level"
    let slash_fmt = Format::new()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter);
    for (i, r) in rows.iter().enumerate() {
        for lvl in r.levels.len()..max_level {
            sheet.write_with_format((i + 1) as u32, (level_at + lvl) as u16, "/", &slash_fmt)?;
        }
    }

    for (i, c) in managed.iter().enumerate() {
        sheet.set_column_width(column_at(i), excel_width(c.id))?;
    }
    for lvl in 0..max_level {
        sheet.set_column_width((level_at + lvl) as u16, 20)?;
    }
    sheet.set_freeze_panes(1, 0)?;

    workbook.save(path)?;
    Ok(())
}

/// Column width used in the Excel list
fn excel_width(id: ColumnId) -> u16 {
    match id {
        ColumnId::No => 8,
        ColumnId::Name => 40,
        ColumnId::Path => 70,
        ColumnId::Size => 14,
        ColumnId::Modified => 20,
        ColumnId::Bytes => 16,
    }
}

const SIZE_UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];

/// Format a file size in the configured unit. `Auto` switches the unit with the
/// file size (B → KB → MB → ...), a fixed unit keeps every row comparable.
pub fn format_size(bytes: u64, unit: SizeUnit) -> String {
    match unit {
        SizeUnit::Auto => {
            let mut size = bytes as f64;
            let mut idx = 0usize;
            while size >= 1024.0 && idx < SIZE_UNITS.len() - 1 {
                size /= 1024.0;
                idx += 1;
            }
            if idx == 0 {
                format!("{bytes} B")
            } else {
                format!("{size:.2} {}", SIZE_UNITS[idx])
            }
        }
        SizeUnit::B => format!("{bytes} B"),
        other => format!("{:.2} {}", bytes as f64 / other.divisor(), other.suffix()),
    }
}

/// Unit annotation appended to the size column header, empty in auto mode
fn unit_header(unit: SizeUnit) -> String {
    match unit {
        SizeUnit::Auto => String::new(),
        other => format!(" ({})", other.suffix()),
    }
}

/// Header text of the size column, carrying the unit when one is fixed
pub fn size_header(tr: &Tr, unit: SizeUnit) -> String {
    format!("{}{}", tr.col_size, unit_header(unit))
}

/// Format a Unix timestamp (seconds) as "YYYY-MM-DD HH:MM:SS" in local time.
/// Returns "-" when the modification time is unknown.
pub fn format_time(secs: u64) -> String {
    if secs == 0 {
        return "-".to_string();
    }
    match chrono::DateTime::<chrono::Utc>::from_timestamp(secs as i64, 0) {
        Some(dt) => dt
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
        None => "-".to_string(),
    }
}
