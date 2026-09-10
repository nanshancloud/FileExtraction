use crate::i18n::Tr;
use crate::scanner::FileEntry;
use rust_xlsxwriter::{Format, FormatAlign, Workbook, XlsxError};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// 导出结果汇总（由 UI 层翻译为状态栏消息）
pub struct ExportResult {
    pub copied: usize,
    pub total: usize,
    /// 复制失败的源文件路径
    pub failed: Vec<String>,
    /// 清单文件保存路径（成功时）
    pub list_path: Option<PathBuf>,
    /// 清单导出失败原因
    pub list_error: Option<String>,
}

/// 将所有扫描到的文档复制到导出目录，并生成 Excel 清单。
/// keep_structure 为 true 时按原目录结构重建子目录，否则平铺导出（重名自动加序号）。
pub fn export_files(
    entries: &[FileEntry],
    scan_dir: &str,
    out_dir: &Path,
    keep_structure: bool,
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

    // 附加产物：在同一目录生成 Excel 清单
    let list_path = out_dir.join(tr.excel_filename);
    let excel_result = write_excel(entries, &list_path, tr);

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
    }
}

/// 按原目录结构导出：重建相对于扫描目录的路径，同名冲突时重命名
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

/// 平铺导出：所有文件放在同一层，重名冲突时生成 "名称 (2).ext" 形式的唯一文件名
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

/// 生成 Excel 清单：序号、文件名称、文件路径、文件大小(字节)、文件大小
fn write_excel(entries: &[FileEntry], path: &Path, tr: &Tr) -> Result<(), XlsxError> {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.set_name(tr.sheet_name)?;

    let header = Format::new()
        .set_bold()
        .set_background_color("#D9E1F2")
        .set_align(FormatAlign::Center);

    let headers = [tr.col_no, tr.col_name, tr.col_path, tr.excel_bytes, tr.excel_size];
    for (c, h) in headers.iter().enumerate() {
        sheet.write_with_format(0u32, c as u16, *h, &header)?;
    }

    for (i, e) in entries.iter().enumerate() {
        let row = (i + 1) as u32;
        sheet.write_number(row, 0, (i + 1) as f64)?;
        sheet.write_string(row, 1, &e.name)?;
        sheet.write_string(row, 2, &e.path)?;
        sheet.write_number(row, 3, e.size as f64)?;
        sheet.write_string(row, 4, human_size(e.size))?;
    }

    sheet.set_column_width(0, 8)?;
    sheet.set_column_width(1, 40)?;
    sheet.set_column_width(2, 70)?;
    sheet.set_column_width(3, 16)?;
    sheet.set_column_width(4, 12)?;
    sheet.set_freeze_panes(1, 0)?;

    workbook.save(path)?;
    Ok(())
}

/// 人类可读的文件大小（如 "1.25 MB"）
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut size = bytes as f64;
    let mut unit = 0usize;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{size:.2} {}", UNITS[unit])
    }
}
