use crate::config::{self, Config, ExtCategory};
use crate::exporter::{self, ExportResult};
use crate::i18n::{tf, Lang, Tr};
use crate::icon;
use crate::scanner::{self, FileEntry, ScanState};
use eframe::egui;
use egui_extras::{Column, TableBuilder};
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

/// Copyright notice shown at the bottom-right corner
const COPYRIGHT: &str = "POWER  BY Eric Yeung";

/// Open a file with the system default associated program (double-click action)
fn open_file(path: &str) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer").arg(path).spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
}

/// Open the containing folder of a file (on Windows, locate and select the file)
fn open_containing_folder(path: &str) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer")
            .arg(format!("/select,{path}"))
            .spawn();
    }
    #[cfg(not(windows))]
    {
        if let Some(parent) = PathBuf::from(path).parent() {
            let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
        }
    }
}

/// Main application state
pub struct FileExtractionApp {
    lang: Lang,
    /// Whether to show the language picker dialog on first run
    need_pick_lang: bool,
    picker_lang: Lang,
    scan_dir: String,
    entries: Vec<FileEntry>,
    /// User-added extensions per category id
    custom_extensions: BTreeMap<String, Vec<String>>,
    selected: HashSet<String>,
    /// Text buffer of the "add extension" input of each category
    new_ext: BTreeMap<String, String>,
    keep_structure: bool,
    /// File name search keyword (filters the table in real time)
    search: String,
    status: String,
    scan: Option<ScanState>,
    config_path: PathBuf,
    /// Cached logo texture for the UI (loaded lazily on the first frame)
    logo_tex: Option<egui::TextureHandle>,
}

impl FileExtractionApp {
    pub fn new() -> Self {
        let config_path = config::config_path();
        let cfg: Config = config::load(&config_path);
        // English by default; skip language picker only when a saved config exists
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
            new_ext: BTreeMap::new(),
            keep_structure: cfg.keep_structure,
            search: String::new(),
            status: lang.tr().status_pick_dir.to_string(),
            scan: None,
            config_path,
            logo_tex: None,
        }
    }

    /// UI text of a built-in extension category
    fn category_label(&self, cat: ExtCategory) -> &'static str {
        let tr = self.tr();
        match cat {
            ExtCategory::DefaultFormat => tr.cat_default,
            ExtCategory::ReportTemplate => tr.cat_report_template,
        }
    }

    /// User-added extensions of one category
    fn custom_of(&self, cat: ExtCategory) -> Vec<String> {
        self.custom_extensions
            .get(cat.id())
            .cloned()
            .unwrap_or_default()
    }

    /// Render one collapsible category block: header row with a hover-only
    /// "select all" button, extension checkboxes and an "add extension" row.
    fn category_section(
        &mut self,
        ui: &mut egui::Ui,
        cat: ExtCategory,
        tr: &Tr,
        changed: &mut bool,
    ) {
        let ctx = ui.ctx().clone();
        let open_id = ui.make_persistent_id((cat.id(), "open"));
        let hover_id = ui.make_persistent_id((cat.id(), "hover"));

        // Collapse state and last frame's hover state are kept in egui memory
        let mut open = ctx.data_mut(|d| d.get_temp::<bool>(open_id)).unwrap_or(true);
        let hovered_prev = ctx.data(|d| d.get_temp::<bool>(hover_id)).unwrap_or(false);

        let ext_list = cat.extensions();
        let custom_list = self.custom_of(cat);
        let total = ext_list.len() + custom_list.len();
        let selected_count = ext_list
            .iter()
            .filter(|e| self.selected.contains(**e))
            .count()
            + custom_list
                .iter()
                .filter(|e| self.selected.contains(e.as_str()))
                .count();

        let title = tf(
            tr.category_progress,
            &[&self.category_label(cat), &selected_count, &total],
        );

        // Once everything is selected the button flips to "deselect all"
        let all_selected = total > 0 && selected_count == total;

        // Header row: disclosure arrow + title on the left, the select/deselect
        // button appears at the end of the row only while the pointer hovers it
        let row = ui.horizontal(|ui| {
            let arrow = if open { "▼" } else { "▶" };
            if ui.small_button(arrow).clicked() {
                open = !open;
            }
            ui.label(egui::RichText::new(title).strong());

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if hovered_prev {
                    let label = if all_selected {
                        tr.deselect_all
                    } else {
                        tr.select_all
                    };
                    if ui.small_button(label).clicked() {
                        for e in ext_list {
                            if all_selected {
                                self.selected.remove(*e);
                            } else {
                                self.selected.insert(e.to_string());
                            }
                        }
                        for e in &custom_list {
                            if all_selected {
                                self.selected.remove(e.as_str());
                            } else {
                                self.selected.insert(e.clone());
                            }
                        }
                        *changed = true;
                    }
                } else {
                    // Reserve the same space so the row does not jump
                    ui.allocate_space(egui::vec2(80.0, 0.0));
                }
            });
        });

        // Remember hover for the next frame (avoids layout jitter)
        let hovered_now = ctx
            .pointer_hover_pos()
            .map_or(false, |p| row.response.rect.contains(p));
        ctx.data_mut(|d| d.insert_temp(hover_id, hovered_now));
        ctx.data_mut(|d| d.insert_temp(open_id, open));

        if !open {
            return;
        }

        ui.indent(cat.id(), |ui| {
            // Built-in extensions of this category
            for ext in ext_list.iter().map(|s| s.to_string()).collect::<Vec<_>>() {
                let mut sel = self.selected.contains(&ext);
                if ui.checkbox(&mut sel, &ext).changed() {
                    if sel {
                        self.selected.insert(ext.clone());
                    } else {
                        self.selected.remove(&ext);
                    }
                    *changed = true;
                }
            }

            // User-added extensions of this category (removable)
            for ext in &custom_list {
                ui.horizontal(|ui| {
                    let mut sel = self.selected.contains(ext);
                    if ui.checkbox(&mut sel, ext).changed() {
                        if sel {
                            self.selected.insert(ext.clone());
                        } else {
                            self.selected.remove(ext);
                        }
                        *changed = true;
                    }
                    if ui.small_button(tr.delete).clicked() {
                        if let Some(list) = self.custom_extensions.get_mut(cat.id()) {
                            list.retain(|e| e != ext);
                        }
                        self.selected.remove(ext);
                        *changed = true;
                    }
                });
            }

            // Add-extension row at the end of the category
            ui.horizontal(|ui| {
                let buf = self.new_ext.entry(cat.id().to_string()).or_default();
                ui.add(
                    egui::TextEdit::singleline(buf)
                        .desired_width(90.0)
                        .hint_text(tr.add_ext_hint),
                );
                if ui.button(tr.add).clicked() {
                    self.add_extension(cat);
                }
            });
        });
    }

    /// Load the embedded logo into an egui texture once, then reuse it
    fn logo_texture(&mut self, ctx: &egui::Context) -> Option<egui::TextureHandle> {
        if self.logo_tex.is_none() {
            let logo = icon::load_logo()?;
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [logo.width as usize, logo.height as usize],
                &logo.rgba,
            );
            self.logo_tex = Some(ctx.load_texture("app_logo", image, Default::default()));
        }
        self.logo_tex.clone()
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

    /// Clear the "add extension" input of one category
    fn clear_ext_input(&mut self, cat: ExtCategory) {
        self.new_ext.insert(cat.id().to_string(), String::new());
    }

    /// Validate and add a user extension into the given category, then select it
    fn add_extension(&mut self, cat: ExtCategory) {
        let tr = self.tr();
        let raw = self
            .new_ext
            .get(cat.id())
            .cloned()
            .unwrap_or_default();
        let ext = raw.trim().trim_start_matches('.').to_lowercase();

        if ext.is_empty() {
            return;
        }
        if !ext.chars().all(|c| c.is_ascii_alphanumeric()) {
            self.status = tf(tr.invalid_ext, &[&ext]);
            return;
        }
        let exists_builtin = config::default_extensions().iter().any(|d| *d == ext.as_str());
        let exists_custom = self
            .custom_extensions
            .values()
            .any(|list| list.iter().any(|e| *e == ext));
        if exists_builtin || exists_custom {
            self.status = tf(tr.ext_exists, &[&ext]);
            self.clear_ext_input(cat);
            return;
        }

        self.custom_extensions
            .entry(cat.id().to_string())
            .or_default()
            .push(ext.clone());
        self.selected.insert(ext);
        self.clear_ext_input(cat);
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
        // 1. Choose export directory (documents are copied here, list is generated here too)
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

        // 2. Copy documents + generate the Excel list
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

impl eframe::App for FileExtractionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // First run: language picker dialog (UI texts switch live with the selection)
        if self.need_pick_lang {
            let pt = self.picker_lang.tr();

            // Semi-transparent overlay: blocks interaction with the main UI for a modal effect
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
                            // Combo box options always show each language's native name
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

        // Top toolbar
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                // Application logo
                if let Some(tex) = self.logo_texture(ctx) {
                    ui.add(
                        egui::Image::new(&tex)
                            .fit_to_exact_size(egui::vec2(22.0, 22.0)),
                    );
                }
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

                // Top-right corner: file name search box, filters the table in real time
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let clear_button = ui.add_enabled(
                        !self.search.is_empty(),
                        egui::Button::new("✕").small(),
                    );
                    if clear_button.clicked() {
                        self.search.clear();
                        clear_button.surrender_focus();
                    }
                    ui.add(
                        egui::TextEdit::singleline(&mut self.search)
                            .desired_width(200.0)
                            .hint_text(tr.search_hint)
                            .clip_text(true),
                    );
                });
            });
            ui.add_space(4.0);
        });

        // Bottom status bar: status text on the left, copyright on the right
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.separator();
            ui.horizontal(|ui| {
                if self.scan.is_some() {
                    ui.spinner();
                }
                ui.label(self.status.clone());

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(COPYRIGHT).weak().small());
                });
            });
        });

        // Left panel: file format settings
        egui::SidePanel::left("ext_panel")
            .default_width(230.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.heading(tr.panel_title);
                ui.separator();

                let mut changed = false;

                // Top-level collapsible blocks: one per category
                for cat in ExtCategory::ALL.iter() {
                    self.category_section(ui, *cat, tr, &mut changed);
                    ui.separator();
                }

                ui.label(egui::RichText::new(tr.auto_save_hint).weak().small());
                ui.label(egui::RichText::new(self.config_path.display().to_string()).small().weak());

                ui.separator();
                ui.label(tf(tr.selected_count, &[&self.selected.len()]));

                // Language switch (can be changed at any time while running)
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

        // Central results table (filtered by search keyword on file name)
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

            let keyword = self.search.trim().to_lowercase();
            let filtered: Vec<&FileEntry> = self
                .entries
                .iter()
                .filter(|e| {
                    keyword.is_empty() || e.name.to_lowercase().contains(&keyword)
                })
                .collect();

            if filtered.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label(egui::RichText::new(tf(tr.no_match, &[&self.search])).weak().size(16.0));
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
                    // Column headers annotate the double-click actions for name/path columns
                    let name_h = format!("{} ({})", tr.col_name, tr.col_name_dblclick);
                    let path_h = format!("{} ({})", tr.col_path, tr.col_path_dblclick);
                    for t in [tr.col_no, name_h.as_str(), path_h.as_str(), tr.col_size] {
                        header.col(|ui| {
                            ui.strong(t);
                        });
                    }
                })
                .body(|mut body| {
                    for (i, e) in filtered.iter().enumerate() {
                        body.row(22.0, |mut row| {
                            row.col(|ui| {
                                ui.monospace(format!("{}", i + 1));
                            });
                            row.col(|ui| {
                                // Double-click opens the file with its default program
                                let r = ui.label(&e.name);
                                if r.double_clicked() {
                                    open_file(&e.path);
                                }
                            });
                            row.col(|ui| {
                                // Double-click opens the containing folder (file gets selected)
                                let r = ui.monospace(&e.path);
                                if r.double_clicked() {
                                    open_containing_folder(&e.path);
                                }
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
