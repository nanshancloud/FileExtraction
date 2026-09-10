use crate::config::{self, Config};
use crate::exporter::{self, ExportResult};
use crate::i18n::{tf, Lang, Tr};
use crate::scanner::{self, FileEntry, ScanState};
use eframe::egui;
use egui_extras::{Column, TableBuilder};
use std::collections::HashSet;
use std::path::PathBuf;

/// 应用主状态
pub struct FileSearchApp {
    lang: Lang,
    /// 首次运行时的语言选择弹窗
    need_pick_lang: bool,
    picker_lang: Lang,
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
    pub fn new() -> Self {
        let config_path = config::config_path();
        let cfg: Config = config::load(&config_path);
        // 默认英文；仅在已有配置时跳过语言选择
        let lang = cfg.language.unwrap_or(Lang::En);
        let need_pick_lang = cfg.language.is_none();

        Self {
            lang,
            need_pick_lang,
            picker_lang: Lang::En,
            scan_dir: String::new(),
            entries: Vec::new(),
            custom_extensions: cfg.custom_extensions,
            selected: cfg.selected,
            new_ext: String::new(),
            keep_structure: cfg.keep_structure,
            status: lang.tr().status_pick_dir.to_string(),
            scan: None,
            config_path,
        }
    }

    fn tr(&self) -> &'static Tr {
        self.lang.tr()
    }

    fn current_config(&self) -> Config {
        Config {
            custom_extensions: self.custom_extensions.clone(),
            selected: self.selected.clone(),
            keep_structure: self.keep_structure,
            language: Some(self.lang),
        }
    }

    fn save_config(&self) {
        config::save(&self.config_path, &self.current_config());
    }

    fn apply_language(&mut self, ctx: &egui::Context, lang: Lang) {
        self.lang = lang;
        self.status = lang.tr().status_pick_dir.to_string();
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(lang.tr().app_title.to_string()));
        self.save_config();
    }

    fn add_extension(&mut self) {
        let tr = self.tr();
        let ext = self
            .new_ext
            .trim()
            .trim_start_matches('.')
            .to_lowercase();
        if ext.is_empty() {
            return;
        }
        if !ext.chars().all(|c| c.is_ascii_alphanumeric()) {
            self.status = tf(tr.invalid_ext, &[&ext]);
            return;
        }
        let exists_default = config::DEFAULT_EXTENSIONS.iter().any(|d| *d == ext);
        let exists_custom = self.custom_extensions.iter().any(|c| *c == ext);
        if exists_default || exists_custom {
            self.status = tf(tr.ext_exists, &[&ext]);
            self.new_ext.clear();
            return;
        }
        self.custom_extensions.push(ext.clone());
        self.selected.insert(ext);
        self.new_ext.clear();
        self.save_config();
        self.status = tr.added_ext.to_string();
    }

    fn start_scan(&mut self, ctx: &egui::Context) {
        let tr = self.tr();
        let dir = PathBuf::from(&self.scan_dir);
        if !dir.is_dir() {
            self.status = tr.status_invalid_dir.to_string();
            return;
        }
        let exts: HashSet<String> = self.selected.iter().cloned().collect();
        self.entries.clear();
        self.status = tr.status_scanning.to_string();
        self.scan = Some(scanner::start_scan(dir, exts, ctx));
    }

    fn poll_scan(&mut self) {
        let tr = self.tr();
        let Some(scan) = &self.scan else { return };
        if scan.is_done() {
            let v = scan.take_result();
            self.status = tf(tr.scan_done, &[&v.len()]);
            self.entries = v;
            self.scan = None;
        } else {
            let cur = scan.current_len();
            self.status = tf(tr.scanning_progress, &[&cur]);
        }
    }

    fn export_files(&mut self) {
        let tr = self.tr();
        // 1. 选择导出目录（文档将复制到该目录，清单也生成在该目录）
        let mut dialog = rfd::FileDialog::new().set_title(tr.pick_dir_title);
        if !self.scan_dir.trim().is_empty() {
            let dir = PathBuf::from(&self.scan_dir);
            if dir.is_dir() {
                dialog = dialog.set_directory(dir.parent().unwrap_or(&dir));
            }
        }
        let Some(out_dir) = dialog.pick_folder() else {
            return;
        };

        // 2. 复制文档 + 生成清单
        let result: ExportResult =
            exporter::export_files(&self.entries, &self.scan_dir, &out_dir, self.keep_structure, tr);
        self.status = self.build_export_status(&result, &out_dir, tr);
    }

    fn build_export_status(&self, result: &ExportResult, out_dir: &std::path::Path, tr: &Tr) -> String {
        let mode = if self.keep_structure {
            tr.mode_structure
        } else {
            tr.mode_flat
        };
        let mut msg = tf(
            tr.exported_to,
            &[&result.copied, &result.total, &mode, &out_dir.display()],
        );
        if let Some(p) = &result.list_path {
            msg.push_str(&tf(tr.list_ok, &[&p.display()]));
        }
        if let Some(e) = &result.list_error {
            msg.push_str(&tf(tr.list_fail, &[e]));
        }
        if !result.failed.is_empty() {
            msg.push_str(&tf(tr.copy_fail, &[&result.failed.len()]));
        }
        msg
    }
}

impl eframe::App for FileSearchApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 首次运行：语言选择弹窗（界面文案跟随当前选择的语言实时切换）
        if self.need_pick_lang {
            let pt = self.picker_lang.tr();

            // 半透明遮罩：拦截主界面交互，使弹窗具有模态效果
            let screen_rect = ctx.screen_rect();
            egui::Area::new(egui::Id::new("lang_picker_blocker"))
                .order(egui::Order::Middle)
                .interactable(true)
                .fixed_pos(screen_rect.min)
                .show(ctx, |ui| {
                    let r = ui.available_rect_before_wrap();
                    let _ = ui.allocate_rect(r, egui::Sense::click_and_drag());
                    ui.painter()
                        .rect_filled(r, 0.0, egui::Color32::from_black_alpha(150));
                });

            egui::Window::new("language_picker")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .resizable(false)
                .collapsible(false)
                .title_bar(false)
                .fixed_size([440.0, 240.0])
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(28.0);
                        ui.heading(egui::RichText::new(pt.lang_title).size(22.0));
                        ui.add_space(24.0);
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            ui.label(egui::RichText::new(pt.language).size(14.0));
                            // 下拉框选项始终显示各语言自身的名称
                            egui::ComboBox::from_id_salt("lang_picker")
                                .width(200.0)
                                .selected_text(self.picker_lang.native_name())
                                .show_ui(ui, |ui| {
                                    for &l in Lang::ALL.iter() {
                                        ui.selectable_value(
                                            &mut self.picker_lang,
                                            l,
                                            l.native_name(),
                                        );
                                    }
                                });
                        });
                        ui.add_space(28.0);
                        if ui
                            .add_sized(
                                [120.0, 32.0],
                                egui::Button::new(egui::RichText::new(pt.lang_ok).size(14.0)),
                            )
                            .clicked()
                        {
                            self.need_pick_lang = false;
                            self.apply_language(ctx, self.picker_lang);
                        }
                    });
                });
            return;
        }

        let tr = self.tr();
        self.poll_scan();

        // 顶部工具栏
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.strong(tr.scan_dir_label);
                ui.add(
                    egui::TextEdit::singleline(&mut self.scan_dir)
                        .desired_width(420.0)
                        .hint_text(tr.dir_hint),
                );
                if ui.button(tr.choose_dir).clicked() {
                    if let Some(p) = rfd::FileDialog::new().pick_folder() {
                        self.scan_dir = p.display().to_string();
                    }
                }
                let scanning = self.scan.is_some();
                let can_scan =
                    !self.scan_dir.trim().is_empty() && !self.selected.is_empty() && !scanning;
                if ui
                    .add_enabled(can_scan, egui::Button::new(tr.start_scan))
                    .clicked()
                {
                    self.start_scan(ctx);
                }
                let can_export = !self.entries.is_empty() && !scanning;
                if ui
                    .add_enabled(can_export, egui::Button::new(tr.export_all))
                    .clicked()
                {
                    self.export_files();
                }
                if ui
                    .checkbox(&mut self.keep_structure, tr.keep_structure)
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
                ui.heading(tr.panel_title);
                ui.separator();

                let mut changed = false;

                ui.label(egui::RichText::new(tr.default_formats).strong());
                for ext in config::DEFAULT_EXTENSIONS.iter() {
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
                ui.label(egui::RichText::new(tr.custom_formats).strong());
                if self.custom_extensions.is_empty() {
                    ui.label(egui::RichText::new(tr.no_custom).weak().small());
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
                        if ui.small_button(tr.delete).clicked() {
                            self.custom_extensions.retain(|e| *e != ext);
                            self.selected.remove(&ext);
                            changed = true;
                        }
                    });
                }

                ui.separator();
                ui.label(egui::RichText::new(tr.add_ext).strong());
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_ext)
                            .desired_width(90.0)
                            .hint_text(tr.add_ext_hint),
                    );
                    if ui.button(tr.add).clicked() {
                        self.add_extension();
                    }
                });
                ui.label(egui::RichText::new(tr.auto_save_hint).weak().small());
                ui.label(egui::RichText::new(self.config_path.display().to_string()).small().weak());

                ui.separator();
                ui.label(tf(tr.selected_count, &[&self.selected.len()]));

                // 语言切换（运行中随时可改）
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(tr.language);
                    let before = self.lang;
                    egui::ComboBox::from_id_salt("lang_switch")
                        .width(140.0)
                        .selected_text(self.lang.native_name())
                        .show_ui(ui, |ui| {
                            for &l in Lang::ALL.iter() {
                                ui.selectable_value(&mut self.lang, l, l.native_name());
                            }
                        });
                    if before != self.lang {
                        self.apply_language(ctx, self.lang);
                    }
                });

                if changed {
                    self.save_config();
                }
            });

        // 中间结果表格
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.entries.is_empty() {
                let msg = if self.scan.is_some() {
                    tr.status_scanning_wait
                } else {
                    tr.status_no_result
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
                    for t in [tr.col_no, tr.col_name, tr.col_path, tr.col_size] {
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
                                ui.label(exporter::human_size(e.size))
                                    .on_hover_text(tf(tr.bytes, &[&e.size]));
                            });
                        });
                    }
                });
        });
    }
}
