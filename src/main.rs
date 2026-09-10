#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use egui_extras::{Column, TableBuilder};
use rust_xlsxwriter::{Format, FormatAlign, Workbook, XlsxError};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use walkdir::WalkDir;

const DEFAULT_EXTENSIONS: &[&str] = &["doc", "docx", "ppt", "pptx", "xls", "xlsx", "txt", "pdf", "md"];

#[derive(Clone)]
struct FileEntry {
    name: String,
    path: String,
    size: u64,
}

#[derive(Serialize, Deserialize, Default)]
struct AppConfig {
    custom_extensions: Vec<String>,
    selected_extensions: Vec<String>,
    #[serde(default)]
    keep_structure: bool,
}

struct ScanState {
    entries: Arc<Mutex<Vec<FileEntry>>>,
    done: Arc<AtomicBool>,
}

struct FileSearchApp {
    scan_dir: String,
    entries: Vec<FileEntry>,
    custom_extensions: Vec<String>,
    selected: HashSet<String>,
    new_ext: String,
    keep_structure: bool,
    status: String,
    scan: Option<ScanState>,
    config_path: PathBuf,
}

impl FileSearchApp {
    fn new() -> Self {
        // 配置文件保存在程序可执行文件相同目录下
        let config_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("config.json");

        let (custom_extensions, selected, keep_structure) = load_config(&config_path);

        Self {
            scan_dir: String::new(),
            entries: Vec::new(),
            custom_extensions,
            selected,
            new_ext: String::new(),
            keep_structure,
            status: "请选择要扫描的目录".to_string(),
            scan: None,
            config_path,
        }
    }

    fn save_config(&self) {
        let cfg = AppConfig {
            custom_extensions: self.custom_extensions.clone(),
            selected_extensions: self.selected.iter().cloned().collect(),
            keep_structure: self.keep_structure,
        };
        if let Some(parent) = self.config_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&cfg) {
            let _ = std::fs::write(&self.config_path, json);
        }
    }

    fn add_extension(&mut self) {
        let ext = self
            .new_ext
            .trim()
            .trim_start_matches('.')
            .to_lowercase();
        if ext.is_empty() {
            return;
        }
        if !ext.chars().all(|c| c.is_ascii_alphanumeric()) {
            self.status = format!("无效的文件后缀: {ext}");
            return;
        }
        let exists_default = DEFAULT_EXTENSIONS.iter().any(|d| *d == ext);
        let exists_custom = self.custom_extensions.iter().any(|c| *c == ext);
        if exists_default || exists_custom {
            self.status = format!("后缀 {ext} 已存在");
            self.new_ext.clear();
            return;
        }
        self.custom_extensions.push(ext.clone());
        self.selected.insert(ext);
        self.new_ext.clear();
        self.save_config();
        self.status = "已添加并保存自定义后缀".to_string();
    }

    fn start_scan(&mut self, ctx: &egui::Context) {
        let dir = PathBuf::from(&self.scan_dir);
        if !dir.is_dir() {
            self.status = "目录无效，请重新选择".to_string();
            return;
        }
        let exts: HashSet<String> = self.selected.iter().cloned().collect();
        let entries = Arc::new(Mutex::new(Vec::new()));
        let done = Arc::new(AtomicBool::new(false));

        let e = Arc::clone(&entries);
        let d = Arc::clone(&done);
        let ctx = ctx.clone();

        self.entries.clear();
        self.status = "正在扫描...".to_string();

        thread::spawn(move || {
            for entry in WalkDir::new(&dir)
                .min_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if !entry.file_type().is_file() {
                    continue;
                }
                let path = entry.path();
                let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
                    continue;
                };
                if exts.contains(&ext.to_lowercase()) {
                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    let fe = FileEntry {
                        name: entry.file_name().to_string_lossy().to_string(),
                        path: path.to_string_lossy().to_string(),
                        size,
                    };
                    e.lock().unwrap().push(fe);
                }
            }
            d.store(true, Ordering::Relaxed);
            ctx.request_repaint();
        });

        self.scan = Some(ScanState { entries, done });
    }

    fn poll_scan(&mut self) {
        let Some(scan) = &self.scan else { return };
        if scan.done.load(Ordering::Relaxed) {
            let v = std::mem::take(&mut *scan.entries.lock().unwrap());
            self.status = format!("扫描完成，共找到 {} 个文件", v.len());
            self.entries = v;
            self.scan = None;
        } else {
            let cur = scan.entries.lock().unwrap().len();
            self.status = format!("正在扫描，已找到 {cur} 个文件...");
        }
    }

    fn export_files(&mut self) {
        // 1. 选择导出目录（文档将复制到该目录，清单也生成在该目录）
        let mut dialog = rfd::FileDialog::new().set_title("选择导出目录");
        if !self.scan_dir.trim().is_empty() {
            let dir = PathBuf::from(&self.scan_dir);
            if dir.is_dir() {
                dialog = dialog.set_directory(dir.parent().unwrap_or(&dir));
            }
        }
        let Some(out_dir) = dialog.pick_folder() else {
            return;
        };

        // 2. 复制所有扫描到的文档
        let scan_root = PathBuf::from(&self.scan_dir);
        let mut used_names: HashSet<String> = HashSet::new();
        let mut copied = 0usize;
        let mut failed: Vec<String> = Vec::new();

        for e in &self.entries {
            let src = PathBuf::from(&e.path);
            let file_name = src.file_name().map(|n| n.to_string_lossy().to_string());
            let Some(file_name) = file_name else {
                failed.push(e.path.clone());
                continue;
            };

            let dest = if self.keep_structure {
                // 按原目录结构导出：重建相对于扫描目录的路径
                let rel = src
                    .strip_prefix(&scan_root)
                    .map(|r| r.to_path_buf())
                    .unwrap_or_else(|_| PathBuf::from(&file_name));
                let dest = out_dir.join(&rel);
                if let Some(parent) = dest.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                // 同名冲突（源目录不同但相对路径映射到同一目标，或目标已存在）时重命名
                let mut d = dest.clone();
                let stem = src.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                let ext = src.extension().map(|x| format!(".{}", x.to_string_lossy())).unwrap_or_default();
                let mut n = 2usize;
                while d.exists() {
                    d = dest.parent().map(|p| {
                        p.join(format!("{stem} ({n}){ext}"))
                    }).unwrap_or_else(|| PathBuf::from(format!("{stem} ({n}){ext}")));
                    n += 1;
                }
                d
            } else {
                // 平铺导出：重名冲突时生成 "名称 (2).ext" 形式的唯一文件名
                let stem = src.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                let ext = src.extension().map(|x| format!(".{}", x.to_string_lossy())).unwrap_or_default();
                let mut dest_name = file_name.clone();
                let mut n = 2usize;
                while used_names.contains(&dest_name)
                    || out_dir.join(&dest_name).exists()
                {
                    dest_name = format!("{stem} ({n}){ext}");
                    n += 1;
                }
                used_names.insert(dest_name.clone());
                out_dir.join(&dest_name)
            };

            match std::fs::copy(&src, &dest) {
                Ok(_) => copied += 1,
                Err(_) => failed.push(e.path.clone()),
            }
        }

        // 3. 附加产物：在同一目录生成 Excel 清单
        let list_path = out_dir.join("文件扫描清单.xlsx");
        let excel_result = write_excel(&self.entries, &list_path)
            .map(|_| list_path.display().to_string());

        let mode = if self.keep_structure { "（按原目录结构）" } else { "（平铺）" };
        let mut msg = format!("已导出 {copied}/{} 个文档{mode}到 {}", self.entries.len(), out_dir.display());
        if let Ok(p) = &excel_result {
            msg.push_str(&format!("；清单: {p}"));
        }
        if let Err(e) = excel_result {
            msg.push_str(&format!("；清单导出失败: {e}"));
        }
        if !failed.is_empty() {
            msg.push_str(&format!("；{} 个文件复制失败", failed.len()));
        }
        self.status = msg;
    }
}

fn load_config(path: &Path) -> (Vec<String>, HashSet<String>, bool) {
    let cfg = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<AppConfig>(&s).ok())
        .unwrap_or_default();

    let defaults: Vec<String> = DEFAULT_EXTENSIONS.iter().map(|s| s.to_string()).collect();
    let mut selected: HashSet<String> = cfg
        .selected_extensions
        .into_iter()
        .filter(|e| defaults.contains(e) || cfg.custom_extensions.contains(e))
        .collect();
    if selected.is_empty() {
        selected.extend(defaults.iter().cloned());
    }
    let mut custom = cfg.custom_extensions;
    custom.retain(|c| !defaults.contains(c));
    (custom, selected, cfg.keep_structure)
}

fn write_excel(entries: &[FileEntry], path: &Path) -> Result<(), XlsxError> {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.set_name("扫描清单")?;

    let header = Format::new()
        .set_bold()
        .set_background_color("#D9E1F2")
        .set_align(FormatAlign::Center);

    let headers = ["序号", "文件名称", "文件路径", "文件大小(字节)", "文件大小"];
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

fn human_size(bytes: u64) -> String {
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

impl eframe::App for FileSearchApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_scan();

        // 顶部工具栏
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.strong("扫描目录:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.scan_dir)
                        .desired_width(420.0)
                        .hint_text("点击右侧按钮选择目录"),
                );
                if ui.button("选择目录...").clicked() {
                    if let Some(p) = rfd::FileDialog::new().pick_folder() {
                        self.scan_dir = p.display().to_string();
                    }
                }
                let scanning = self.scan.is_some();
                let can_scan =
                    !self.scan_dir.trim().is_empty() && !self.selected.is_empty() && !scanning;
                if ui
                    .add_enabled(can_scan, egui::Button::new("开始扫描"))
                    .clicked()
                {
                    self.start_scan(ctx);
                }
                let can_export = !self.entries.is_empty() && !scanning;
                if ui
                    .add_enabled(can_export, egui::Button::new("导出所有文件"))
                    .clicked()
                {
                    self.export_files();
                }
                if ui
                    .checkbox(&mut self.keep_structure, "按原目录结构导出")
                    .changed()
                {
                    self.save_config();
                }
            });
            ui.add_space(4.0);
        });

        // 底部状态栏
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.separator();
            ui.horizontal(|ui| {
                if self.scan.is_some() {
                    ui.spinner();
                }
                ui.label(self.status.clone());
            });
        });

        // 左侧格式配置面板
        egui::SidePanel::left("ext_panel")
            .default_width(230.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.heading("文件格式配置");
                ui.separator();

                let mut changed = false;

                ui.label(egui::RichText::new("默认格式").strong());
                for ext in DEFAULT_EXTENSIONS.iter() {
                    let ext = ext.to_string();
                    let mut sel = self.selected.contains(&ext);
                    if ui.checkbox(&mut sel, &ext).changed() {
                        if sel {
                            self.selected.insert(ext.clone());
                        } else {
                            self.selected.remove(&ext);
                        }
                        changed = true;
                    }
                }

                ui.separator();
                ui.label(egui::RichText::new("自定义格式").strong());
                if self.custom_extensions.is_empty() {
                    ui.label(
                        egui::RichText::new("（暂无，可在下方添加）")
                            .weak()
                            .small(),
                    );
                }
                let customs = self.custom_extensions.clone();
                for ext in customs {
                    ui.horizontal(|ui| {
                        let mut sel = self.selected.contains(&ext);
                        if ui.checkbox(&mut sel, &ext).changed() {
                            if sel {
                                self.selected.insert(ext.clone());
                            } else {
                                self.selected.remove(&ext);
                            }
                            changed = true;
                        }
                        if ui.small_button("删除").clicked() {
                            self.custom_extensions.retain(|e| *e != ext);
                            self.selected.remove(&ext);
                            changed = true;
                        }
                    });
                }

                ui.separator();
                ui.label(egui::RichText::new("添加后缀").strong());
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_ext)
                            .desired_width(90.0)
                            .hint_text("如 csv"),
                    );
                    if ui.button("添加").clicked() {
                        self.add_extension();
                    }
                });
                ui.label(
                    egui::RichText::new("支持多选，配置自动保存到：")
                        .weak()
                        .small(),
                );
                ui.label(egui::RichText::new(self.config_path.display().to_string()).small().weak());

                ui.separator();
                ui.label(format!("已选格式: {} 个", self.selected.len()));

                if changed {
                    self.save_config();
                }
            });

        // 中间结果表格
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.entries.is_empty() {
                let msg = if self.scan.is_some() {
                    "正在扫描，请稍候..."
                } else {
                    "暂无扫描结果，请选择目录并点击\"开始扫描\""
                };
                ui.centered_and_justified(|ui| {
                    ui.label(egui::RichText::new(msg).weak().size(16.0));
                });
                return;
            }

            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(50.0))
                .column(Column::initial(200.0).resizable(true))
                .column(Column::initial(420.0).resizable(true))
                .column(Column::exact(110.0))
                .header(26.0, |mut header| {
                    for t in ["序号", "文件名称", "文件路径", "文件大小"] {
                        header.col(|ui| {
                            ui.strong(t);
                        });
                    }
                })
                .body(|mut body| {
                    for (i, e) in self.entries.iter().enumerate() {
                        body.row(22.0, |mut row| {
                            row.col(|ui| {
                                ui.monospace(format!("{}", i + 1));
                            });
                            row.col(|ui| {
                                ui.label(&e.name);
                            });
                            row.col(|ui| {
                                ui.monospace(&e.path);
                            });
                            row.col(|ui| {
                                ui.label(human_size(e.size))
                                    .on_hover_text(format!("{} 字节", e.size));
                            });
                        });
                    }
                });
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1150.0, 660.0])
            .with_title("文件扫描清单工具"),
        ..Default::default()
    };
    eframe::run_native(
        "FileSearch",
        options,
        Box::new(|cc| {
            setup_cjk_fonts(&cc.egui_ctx);
            Ok(Box::new(FileSearchApp::new()))
        }),
    )
}

/// 在 Windows 系统字体目录中寻找可用的中文字体文件
fn load_cjk_font_bytes() -> Option<Vec<u8>> {
    const PREFERRED: &[&str] = &[
        r"C:\Windows\Fonts\simhei.ttf",
        r"C:\Windows\Fonts\simfang.ttf",
        r"C:\Windows\Fonts\simkai.ttf",
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\msyhbd.ttc",
        r"C:\Windows\Fonts\simsun.ttc",
    ];
    for p in PREFERRED {
        if let Ok(bytes) = std::fs::read(p) {
            return Some(bytes);
        }
    }
    // 兜底：扫描 Fonts 目录，挑第一个含中文/常见 CJK 字体名的 ttf/ttc/otf
    if let Ok(rd) = std::fs::read_dir(r"C:\Windows\Fonts") {
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            let lower_name = name.as_str();
            let is_zh = lower_name.contains("sim")
                || lower_name.contains("yahei")
                || lower_name.contains("msyh")
                || lower_name.contains("noto")
                || lower_name.contains("cjk")
                || lower_name.contains("hei");
            let ext_ok = lower_name.ends_with(".ttf")
                || lower_name.ends_with(".ttc")
                || lower_name.ends_with(".otf");
            if is_zh && ext_ok {
                if let Ok(bytes) = std::fs::read(entry.path()) {
                    return Some(bytes);
                }
            }
        }
    }
    None
}

/// 将中文字体注册到 egui 的 Proportional / Monospace 字体回退链
fn setup_cjk_fonts(ctx: &egui::Context) {
    let Some(bytes) = load_cjk_font_bytes() else {
        eprintln!("未找到系统中文字体，UI 可能仍显示方框");
        return;
    };
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("cjk".to_owned(), egui::FontData::from_owned(bytes));
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        if let Some(list) = fonts.families.get_mut(&family) {
            list.push("cjk".to_owned());
        }
    }
    ctx.set_fonts(fonts);
}
