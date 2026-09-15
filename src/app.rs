use crate::config::{self, ColumnDef, ColumnId, Config, ExtCategory, PathMode, SizeUnit};
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

/// Width of the label column of a settings row: every label is padded to this
/// width, so all drop-downs start at the same x position
const SETTINGS_LABEL_W: f32 = 150.0;
/// Width every drop-down of the settings (and of the language dialog) uses
const SETTINGS_COMBO_W: f32 = 170.0;

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
    /// Scan results, always kept in the default tree order (used for exporting)
    entries: Vec<FileEntry>,
    /// Result rows in the order they are displayed: indices into `entries`,
    /// rearranged when a column is sorted. Sorting therefore stays inside the
    /// results table and never touches the exported list.
    view: Vec<usize>,
    /// User-added extensions per category id
    custom_extensions: BTreeMap<String, Vec<String>>,
    selected: HashSet<String>,
    /// Text buffer of the "add extension" input of each category
    new_ext: BTreeMap<String, String>,
    keep_structure: bool,
    /// Column order and visibility of the result table and the exported list
    columns: Vec<ColumnDef>,
    /// Restore the last scan directory and rescan it when the app starts
    open_last_dir: bool,
    /// Directory used by the last scan
    last_dir: String,
    /// Unit used to display file sizes (Auto switches per file size)
    size_unit: SizeUnit,
    /// Whether paths are shown absolute or relative to the scan root
    path_mode: PathMode,
    /// Set on startup when the last directory has to be scanned automatically
    auto_scan_pending: bool,
    /// Sort column of the result table and whether it is sorted Z-A
    sort: Option<(ColumnId, bool)>,
    /// True while a column width is being changed but not saved yet
    width_dirty: bool,
    /// Whether the settings window is currently open
    show_settings: bool,
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

        // "Open last used directory": restore the directory and rescan it on the
        // first frame, so the previous results come back without any click
        let last_dir = cfg.last_dir.trim().to_string();
        let restore = !need_pick_lang && cfg.open_last_dir && PathBuf::from(&last_dir).is_dir();

        Self {
            lang,
            need_pick_lang,
            picker_lang: Lang::En,
            scan_dir: if restore { last_dir.clone() } else { String::new() },
            entries: Vec::new(),
            view: Vec::new(),
            custom_extensions: cfg.custom_extensions,
            selected: cfg.selected,
            new_ext: BTreeMap::new(),
            keep_structure: cfg.keep_structure,
            columns: cfg.columns,
            open_last_dir: cfg.open_last_dir,
            last_dir,
            size_unit: cfg.size_unit,
            path_mode: cfg.path_mode,
            auto_scan_pending: restore,
            sort: None,
            width_dirty: false,
            show_settings: false,
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
            ExtCategory::Archive => tr.cat_archive,
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
            columns: self.columns.clone(),
            open_last_dir: self.open_last_dir,
            last_dir: self.last_dir.clone(),
            size_unit: self.size_unit,
            path_mode: self.path_mode,
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
        // Letters, digits and dots are allowed so that compound extensions
        // such as "tar.gz" can be added by hand as well
        let valid = !ext.starts_with('.')
            && !ext.ends_with('.')
            && !ext.contains("..")
            && ext.chars().all(|c| c.is_ascii_alphanumeric() || c == '.');
        if !valid {
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
        // Remember the directory so the next start can restore it
        if self.last_dir != self.scan_dir {
            self.last_dir = self.scan_dir.clone();
            self.save_config();
        }
        let exts: HashSet<String> = self.selected.iter().cloned().collect();
        self.entries.clear();
        self.view.clear();
        self.status = tr.status_scanning.to_string();
        self.scan = Some(scanner::start_scan(dir, exts, ctx));
    }

    fn poll_scan(&mut self) {
        let tr = self.tr();
        let Some(scan) = &self.scan else { return };
        if scan.is_done() {
            let mut v = scan.take_result();
            self.status = tf(tr.scan_done, &[&v.len()]);
            // Root files first, then sub-directory by sub-directory, A-Z
            let root = PathBuf::from(&self.scan_dir);
            scanner::sort_entries(&mut v, &root);
            self.entries = v;
            self.scan = None;
            // The rows start in the tree order; a column sort chosen earlier is
            // kept for the new results
            self.apply_sort();
        } else {
            let cur = scan.current_len();
            self.status = tf(tr.scanning_progress, &[&cur]);
        }
    }

    /// Double-click on a column header: A-Z first, then Z-A, then back to the
    /// default order (files of the root first, then sub-directory by sub-directory).
    /// Only the results table is sorted, the exported list is always A-Z.
    fn toggle_sort(&mut self, id: ColumnId) {
        // Sorting by row number is the default order, nothing to do
        if id == ColumnId::No {
            return;
        }
        self.sort = match self.sort {
            Some((c, false)) if c == id => Some((c, true)),
            Some((c, true)) if c == id => None,
            _ => Some((id, false)),
        };
        self.apply_sort();
    }

    /// Rebuild `view` from `entries`: without a sort column the default tree
    /// order is restored, otherwise the rows are ordered by that column
    fn apply_sort(&mut self) {
        self.view = (0..self.entries.len()).collect();
        let Some((col, desc)) = self.sort else {
            return;
        };
        let entries = &self.entries;
        self.view.sort_by(|&a, &b| {
            use std::cmp::Ordering;
            let x = &entries[a];
            let y = &entries[b];
            let ord = match col {
                ColumnId::Name => x
                    .name
                    .to_lowercase()
                    .cmp(&y.name.to_lowercase())
                    .then_with(|| x.path.cmp(&y.path)),
                ColumnId::Path => {
                    let mode = self.path_mode;
                    shown_path(x, mode).to_lowercase().cmp(&shown_path(y, mode).to_lowercase())
                }
                ColumnId::Size | ColumnId::Bytes => x.size.cmp(&y.size),
                ColumnId::Modified => x.modified.cmp(&y.modified),
                ColumnId::No => Ordering::Equal,
            };
            if desc { ord.reverse() } else { ord }
        });
    }

    /// Ask the user where the export output (documents and/or list) should go
    fn pick_export_dir(&self, tr: &Tr) -> Option<PathBuf> {
        let mut dialog = rfd::FileDialog::new().set_title(tr.pick_dir_title);
        if !self.scan_dir.trim().is_empty() {
            let dir = PathBuf::from(&self.scan_dir);
            if dir.is_dir() {
                dialog = dialog.set_directory(dir.parent().unwrap_or(&dir));
            }
        }
        dialog.pick_folder()
    }

    /// Copy documents into the chosen directory and generate the Excel list
    fn export_files(&mut self) {
        let tr = self.tr();
        let Some(out_dir) = self.pick_export_dir(tr) else {
            return;
        };

        let result: ExportResult = exporter::export_files(
            &self.entries,
            &self.scan_dir,
            &out_dir,
            self.keep_structure,
            &self.columns,
            self.size_unit,
            self.path_mode,
            tr,
        );
        self.status = self.build_export_status(&result, &out_dir, tr);
    }

    /// Generate the Excel list only, without copying any document
    fn export_list_only(&mut self) {
        let tr = self.tr();
        let Some(out_dir) = self.pick_export_dir(tr) else {
            return;
        };

        let result = exporter::export_list_only(
            &self.entries,
            &self.scan_dir,
            &out_dir,
            &self.columns,
            self.size_unit,
            self.path_mode,
            tr,
        );
        self.status = self.build_export_status(&result, &out_dir, tr);
    }

    fn build_export_status(&self, result: &ExportResult, out_dir: &std::path::Path, tr: &Tr) -> String {
        let mut msg = if result.list_only {
            // List only: the message already carries the output path
            tf(tr.list_only_done, &[&result.total, &out_dir.display()])
        } else {
            let mode = if self.keep_structure {
                tr.mode_structure
            } else {
                tr.mode_flat
            };
            tf(
                tr.exported_to,
                &[&result.copied, &result.total, &mode, &out_dir.display()],
            )
        };
        if !result.list_only {
            if let Some(p) = &result.list_path {
                msg.push_str(&tf(tr.list_ok, &[&p.display()]));
            }
        }
        if let Some(e) = &result.list_error {
            msg.push_str(&tf(tr.list_fail, &[e]));
        }
        if !result.failed.is_empty() {
            msg.push_str(&tf(tr.copy_fail, &[&result.failed.len()]));
        }
        msg
    }

    /// Settings window: language switch plus column order / visibility
    fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }
        let tr = self.tr();
        let mut open = self.show_settings;
        let mut lang = self.lang;
        let mut unit = self.size_unit;
        let mut path_mode = self.path_mode;
        let mut col_changed = false;
        let mut general_changed = false;

        egui::Window::new(tr.settings)
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(360.0);

                settings_combo_row(
                    ui,
                    egui::RichText::new(tr.language),
                    egui::ComboBox::from_id_salt("lang_switch").selected_text(lang.native_name()),
                    |ui| {
                        for &l in Lang::ALL.iter() {
                            ui.selectable_value(&mut lang, l, l.native_name());
                        }
                    },
                );

                // File size unit: "Auto" switches B / KB / MB ... per file size
                settings_combo_row(
                    ui,
                    egui::RichText::new(tr.size_unit),
                    egui::ComboBox::from_id_salt("size_unit").selected_text(unit.label(tr)),
                    |ui| {
                        for &u in SizeUnit::ALL.iter() {
                            ui.selectable_value(&mut unit, u, u.label(tr));
                        }
                    },
                );

                // Path display: the full path, or the one relative to the scan root
                settings_combo_row(
                    ui,
                    egui::RichText::new(tr.path_display),
                    egui::ComboBox::from_id_salt("path_mode").selected_text(path_mode.label(tr)),
                    |ui| {
                        for &m in PathMode::ALL.iter() {
                            ui.selectable_value(&mut path_mode, m, m.label(tr));
                        }
                    },
                );
                ui.label(egui::RichText::new(tr.path_display_hint).weak().small());

                ui.separator();
                ui.strong(tr.col_manage);

                // Collect the actions first: the loop only borrows self immutably
                let len = self.columns.len();
                let mut swap: Option<(usize, usize)> = None;
                let mut toggles: Vec<(usize, bool)> = Vec::new();
                for i in 0..len {
                    let label = self.columns[i].id.label(tr);
                    let mut visible = self.columns[i].visible;
                    ui.horizontal(|ui| {
                        if ui.add_enabled(i > 0, egui::Button::new("↑")).clicked() {
                            swap = Some((i, i - 1));
                        }
                        if ui.add_enabled(i + 1 < len, egui::Button::new("↓")).clicked() {
                            swap = Some((i, i + 1));
                        }
                        if ui.checkbox(&mut visible, label).changed() {
                            toggles.push((i, visible));
                        }
                    });
                }
                if let Some((a, b)) = swap {
                    self.columns.swap(a, b);
                    col_changed = true;
                }
                for (i, v) in toggles {
                    if self.columns[i].visible != v {
                        self.columns[i].visible = v;
                        col_changed = true;
                    }
                }

                ui.separator();
                if ui
                    .checkbox(&mut self.open_last_dir, tr.open_last_dir)
                    .changed()
                {
                    general_changed = true;
                }
                ui.label(egui::RichText::new(tr.open_last_dir_hint).weak().small());
            });

        self.show_settings = open;
        if lang != self.lang {
            self.apply_language(ctx, lang);
        }
        if unit != self.size_unit {
            self.size_unit = unit;
            general_changed = true;
        }
        if path_mode != self.path_mode {
            self.path_mode = path_mode;
            general_changed = true;
            // A path sort has to follow the new text
            if matches!(self.sort, Some((ColumnId::Path, _))) {
                self.apply_sort();
            }
        }
        if col_changed || general_changed {
            self.save_config();
        }
    }
}

/// Column of the result table: starts at the configured width, every column can
/// be dragged wider or narrower (the separator responds to a double-click by
/// fitting the content)
fn table_column(id: ColumnId, width: f32) -> Column {
    Column::initial(width)
        .at_least(id.min_width())
        .at_most(2000.0)
        .resizable(true)
}

/// Rough number of lines a text needs inside a column of `width` points.
/// `mono` selects the (slightly wider) average glyph width of monospace text.
fn wrapped_lines(text: &str, width: f32, font_size: f32, mono: bool) -> usize {
    /// Never let a single row grow endlessly when a column is dragged tiny
    const MAX_LINES: usize = 6;
    let mut text_width = 0.0f32;
    for ch in text.chars() {
        text_width += if is_wide_char(ch) {
            font_size
        } else if mono {
            font_size * 0.60
        } else {
            font_size * 0.52
        };
    }
    let available = (width - 10.0).max(font_size);
    let lines = (text_width / available).ceil() as usize;
    lines.clamp(1, MAX_LINES)
}

/// Full width characters (CJK and friends) take about one font size per glyph
fn is_wide_char(ch: char) -> bool {
    matches!(
        ch,
        '\u{1100}'..='\u{115F}'
            | '\u{2E80}'..='\u{A4CF}'
            | '\u{AC00}'..='\u{D7A3}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{FE30}'..='\u{FE6F}'
            | '\u{FF00}'..='\u{FF60}'
            | '\u{FFE0}'..='\u{FFE6}'
            | '\u{20000}'..='\u{3FFFD}'
    )
}

/// One row of the settings window: a label padded to a fixed width followed by
/// a drop-down of a fixed width, so the drop-downs of all rows line up
fn settings_combo_row(
    ui: &mut egui::Ui,
    label: egui::RichText,
    combo: egui::ComboBox,
    show: impl FnOnce(&mut egui::Ui),
) {
    ui.horizontal(|ui| {
        let h = ui.text_style_height(&egui::TextStyle::Body);
        ui.add_sized([SETTINGS_LABEL_W, h], egui::Label::new(label).truncate());
        let _ = combo.width(SETTINGS_COMBO_W).show_ui(ui, show);
    });
}

/// Path of a file as it is currently displayed: the full one or the one relative
/// to the scan root, depending on the setting in the settings window
fn shown_path<'a>(e: &'a FileEntry, mode: PathMode) -> &'a str {
    match mode {
        PathMode::Absolute => e.path.as_str(),
        PathMode::Relative => e.rel_path.as_str(),
    }
}

/// Height of one result row: the tallest cell decides, and a cell becomes taller
/// as soon as its text has to wrap inside the current column width
fn row_height(
    e: &FileEntry,
    columns: &[ColumnDef],
    widths: &[f32],
    line_height: f32,
    font_size: f32,
    path_mode: PathMode,
) -> f32 {
    let mut lines = 1usize;
    for (c, w) in columns.iter().zip(widths) {
        // Only the name and the path are long enough to be worth wrapping
        let text = match c.id {
            ColumnId::Name => Some((e.name.as_str(), false)),
            ColumnId::Path => Some((shown_path(e, path_mode), true)),
            _ => None,
        };
        if let Some((text, mono)) = text {
            lines = lines.max(wrapped_lines(text, *w, font_size, mono));
        }
    }
    (lines as f32 * line_height + 6.0).max(22.0)
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
                            // Center the row inside the dialog
                            let pad = (ui.available_width()
                                - (SETTINGS_LABEL_W + SETTINGS_COMBO_W))
                                .max(0.0)
                                / 2.0;
                            ui.add_space(pad);
                            // Combo box options always show each language's native name
                            settings_combo_row(
                                ui,
                                egui::RichText::new(pt.language).size(14.0),
                                egui::ComboBox::from_id_salt("lang_picker")
                                    .selected_text(self.picker_lang.native_name()),
                                |ui| {
                                    for &l in Lang::ALL.iter() {
                                        ui.selectable_value(
                                            &mut self.picker_lang,
                                            l,
                                            l.native_name(),
                                        );
                                    }
                                },
                            );
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

        // "Open last used directory": rescan the restored directory once the UI
        // is running, so the previous results reappear right away
        if self.auto_scan_pending {
            self.auto_scan_pending = false;
            self.start_scan(ctx);
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
                        if self.last_dir != self.scan_dir {
                            self.last_dir = self.scan_dir.clone();
                            self.save_config();
                        }
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
                    .add_enabled(can_export, egui::Button::new(tr.export_list_only))
                    .clicked()
                {
                    self.export_list_only();
                }
                if ui
                    .checkbox(&mut self.keep_structure, tr.keep_structure)
                    .changed()
                {
                    self.save_config();
                }
                if ui.button(tr.settings).clicked() {
                    self.show_settings = !self.show_settings;
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

            // Columns follow the configured order, hidden ones are skipped
            let visible: Vec<ColumnDef> =
                self.columns.iter().copied().filter(|c| c.visible).collect();
            let unit = self.size_unit;
            let path_mode = self.path_mode;
            let sort = self.sort;
            let line_height = ui.text_style_height(&egui::TextStyle::Body).max(14.0);
            let font_size = egui::TextStyle::Body.resolve(ui.style()).size;
            let widths: Vec<f32> = visible.iter().map(|c| c.width()).collect();
            // egui_extras names the drag handles of the column separators like this
            let resize_ids: Vec<egui::Id> = (0..visible.len())
                .map(|i| ui.id().with("resize_column").with(i))
                .collect();

            // Filled by the table: real column widths and the header that was
            // double-clicked. Both are applied after the table, because the rows
            // keep borrowing `self.entries` while the table is built.
            let mut measured: Vec<f32> = Vec::new();
            let mut sort_click: Option<ColumnId> = None;
            let mut no_match = false;
            {
                // Rows are drawn in the order of `view`, i.e. the order produced
                // by the column sort; `entries` itself keeps the export order
                let filtered: Vec<&FileEntry> = self
                    .view
                    .iter()
                    .filter_map(|i| self.entries.get(*i))
                    .filter(|e| keyword.is_empty() || e.name.to_lowercase().contains(&keyword))
                    .collect();

                if filtered.is_empty() {
                    no_match = true;
                } else {
                    let mut table = TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center));
                    for c in &visible {
                        table = table.column(table_column(c.id, c.width()));
                    }
                    table
                        .header(28.0, |mut header| {
                            // Column headers annotate the double-click actions for name/path columns
                            let name_h = format!("{} ({})", tr.col_name, tr.col_name_dblclick);
                            let path_h = format!("{} ({})", tr.col_path, tr.col_path_dblclick);
                            let size_h = exporter::size_header(tr, unit);
                            let hint = format!("{}\n{}", tr.sort_hint, tr.width_hint);
                            for c in &visible {
                                let base: &str = match c.id {
                                    ColumnId::Name => name_h.as_str(),
                                    ColumnId::Path => path_h.as_str(),
                                    ColumnId::Size => size_h.as_str(),
                                    _ => c.id.label(tr),
                                };
                                // An arrow marks the sorted column and its direction
                                let title = match sort {
                                    Some((sc, desc)) if sc == c.id => {
                                        format!("{} {}", base, if desc { "▼" } else { "▲" })
                                    }
                                    _ => base.to_string(),
                                };
                                let mut width = 0.0f32;
                                header.col(|ui| {
                                    width = ui.max_rect().width();
                                    let r = ui.add(
                                        egui::Label::new(egui::RichText::new(&title).strong())
                                            .truncate()
                                            .sense(egui::Sense::click()),
                                    );
                                    // Double-click on the header sorts by this column
                                    if r.double_clicked() {
                                        sort_click = Some(c.id);
                                    }
                                    r.on_hover_text(&hint);
                                });
                                measured.push(width);
                            }
                        })
                        .body(|body| {
                            // Rows grow when a narrowed column makes the text wrap
                            let heights = filtered
                                .iter()
                                .map(|e| {
                                    row_height(e, &visible, &widths, line_height, font_size, path_mode)
                                });
                            body.heterogeneous_rows(heights, |mut row| {
                                let i = row.index();
                                let e = filtered[i];
                                for c in &visible {
                                    row.col(|ui| match c.id {
                                        ColumnId::No => {
                                            ui.monospace(format!("{}", i + 1));
                                        }
                                        // Double-click opens the file with its default program
                                        ColumnId::Name => {
                                            let r = ui.add(
                                                egui::Label::new(e.name.as_str())
                                                    .wrap()
                                                    .sense(egui::Sense::click()),
                                            );
                                            if r.double_clicked() {
                                                open_file(&e.path);
                                            }
                                        }
                                        // Absolute or relative to the scan root,
                                        // depending on the setting; the other
                                        // variant is shown while hovering
                                        ColumnId::Path => {
                                            let shown = shown_path(e, path_mode);
                                            let other = match path_mode {
                                                PathMode::Absolute => e.rel_path.as_str(),
                                                PathMode::Relative => e.path.as_str(),
                                            };
                                            let r = ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(shown).monospace(),
                                                )
                                                .wrap()
                                                .sense(egui::Sense::click()),
                                            );
                                            if r.double_clicked() {
                                                open_containing_folder(&e.path);
                                            }
                                            r.on_hover_text(other);
                                        }
                                        ColumnId::Size => {
                                            ui.label(exporter::format_size(e.size, unit))
                                                .on_hover_text(tf(tr.bytes, &[&e.size]));
                                        }
                                        ColumnId::Bytes => {
                                            ui.monospace(e.size.to_string());
                                        }
                                        ColumnId::Modified => {
                                            ui.label(exporter::format_time(e.modified));
                                        }
                                    });
                                }
                            });
                        });
                }
            }

            if no_match {
                ui.centered_and_justified(|ui| {
                    ui.label(egui::RichText::new(tf(tr.no_match, &[&self.search])).weak().size(16.0));
                });
                return;
            }

            // Sort the results by the column whose header was double-clicked
            if let Some(id) = sort_click {
                self.toggle_sort(id);
            }

            // Remember the widths the user dragged; saved when the drag ends
            let resizing = resize_ids
                .iter()
                .any(|id| ui.ctx().read_response(*id).is_some_and(|r| r.dragged()));
            if resizing || self.width_dirty {
                for (c, w) in visible.iter().zip(&measured) {
                    if let Some(cd) = self.columns.iter_mut().find(|x| x.id == c.id) {
                        cd.set_width(*w);
                    }
                }
                self.width_dirty = resizing;
                if !resizing {
                    self.save_config();
                }
            }
        });

        // Settings window: language switch and column management
        self.settings_window(ctx);
    }
}
