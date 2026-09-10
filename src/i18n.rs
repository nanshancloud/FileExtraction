use serde::{Deserialize, Serialize};

/// Runtime formatting: replace each "{}" in the template with the arguments in order.
/// Single pass: placeholders are only searched in the not-yet-processed tail, so
/// arguments containing "{}" are never re-substituted. Once arguments run out the
/// remaining placeholders are kept as-is; extra arguments are ignored.
pub fn tf(template: &str, args: &[&dyn std::fmt::Display]) -> String {
    use std::fmt::Write as _;

    let mut out = String::with_capacity(template.len() + 32);
    let mut rest = template;
    let mut it = args.iter();

    while let Some(pos) = rest.find("{}") {
        let (head, tail) = rest.split_at(pos);
        out.push_str(head);
        match it.next() {
            Some(arg) => {
                let _ = write!(out, "{arg}");
            }
            None => out.push_str("{}"), // no arguments left: keep the placeholder as-is
        }
        rest = &tail["{}".len()..];
    }
    out.push_str(rest);
    out
}

/// Supported languages
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Lang {
    #[serde(rename = "en")]
    En,
    #[serde(rename = "zh")]
    Zh,
    #[serde(rename = "zh-tw")]
    ZhTw,
    #[serde(rename = "ja")]
    Ja,
    #[serde(rename = "es")]
    Es,
    #[serde(rename = "fr")]
    Fr,
}

/// The drop-down always shows each language in its native name
impl Lang {
    pub const ALL: [Lang; 6] = [
        Lang::En,
        Lang::Zh,
        Lang::ZhTw,
        Lang::Ja,
        Lang::Es,
        Lang::Fr,
    ];

    pub fn native_name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Zh => "简体中文",
            Lang::ZhTw => "繁體中文",
            Lang::Ja => "日本語",
            Lang::Es => "Español",
            Lang::Fr => "Français",
        }
    }

    pub fn tr(self) -> &'static Tr {
        match self {
            Lang::En => &EN,
            Lang::Zh => &ZH,
            Lang::ZhTw => &ZH_TW,
            Lang::Ja => &JA,
            Lang::Es => &ES,
            Lang::Fr => &FR,
        }
    }
}

/// All UI texts. Parameterized strings use "{}" placeholders filled via `tf()`.
pub struct Tr {
    pub app_title: &'static str,
    pub scan_dir_label: &'static str,
    pub dir_hint: &'static str,
    pub choose_dir: &'static str,
    pub start_scan: &'static str,
    pub export_all: &'static str,
    pub keep_structure: &'static str,
    pub pick_dir_title: &'static str,

    pub status_pick_dir: &'static str,
    pub status_invalid_dir: &'static str,
    pub status_scanning: &'static str,
    pub scanning_progress: &'static str,
    pub scan_done: &'static str,
    pub status_no_result: &'static str,
    pub status_scanning_wait: &'static str,

    pub invalid_ext: &'static str,
    pub ext_exists: &'static str,
    pub added_ext: &'static str,

    pub panel_title: &'static str,
    pub cat_default: &'static str,
    pub cat_report_template: &'static str,
    /// Category header with selection progress, e.g. "Default formats ({}/{})"
    pub category_progress: &'static str,
    pub select_all: &'static str,
    /// Shown instead of "select all" once every option is already selected
    pub deselect_all: &'static str,
    pub delete: &'static str,
    pub add_ext_hint: &'static str,
    pub add: &'static str,
    pub auto_save_hint: &'static str,
    pub selected_count: &'static str,

    pub col_no: &'static str,
    pub col_name: &'static str,
    pub col_path: &'static str,
    pub col_size: &'static str,
    /// Column header annotation: file name column (double-click to open file)
    pub col_name_dblclick: &'static str,
    /// Column header annotation: file path column (double-click to open folder)
    pub col_path_dblclick: &'static str,

    pub sheet_name: &'static str,
    pub excel_bytes: &'static str,
    pub excel_size: &'static str,
    pub excel_filename: &'static str,

    pub language: &'static str,
    pub lang_title: &'static str,
    pub lang_ok: &'static str,

    pub search_hint: &'static str,
    pub no_match: &'static str,

    pub mode_structure: &'static str,
    pub mode_flat: &'static str,
    pub exported_to: &'static str,
    pub list_ok: &'static str,
    pub list_fail: &'static str,
    pub copy_fail: &'static str,
    pub bytes: &'static str,
}

static EN: Tr = Tr {
    app_title: "FileExtraction",
    scan_dir_label: "Scan directory:",
    dir_hint: "Click the button on the right to choose a directory",
    choose_dir: "Browse...",
    start_scan: "Start Scan",
    export_all: "Export All Files",
    keep_structure: "Export with original folder structure",
    pick_dir_title: "Choose export directory",

    status_pick_dir: "Please select a directory to scan",
    status_invalid_dir: "Invalid directory, please choose again",
    status_scanning: "Scanning...",
    scanning_progress: "Scanning, {} files found so far...",
    scan_done: "Scan complete, {} files found",
    status_no_result: "No results yet. Select a directory and click \"Start Scan\"",
    status_scanning_wait: "Scanning, please wait...",

    invalid_ext: "Invalid file extension: {}",
    ext_exists: "Extension {} already exists",
    added_ext: "Custom extension added and saved",

    panel_title: "File Format Settings",
    cat_default: "Default formats",
    cat_report_template: "Report templates",
    category_progress: "{} ({}/{})",
    select_all: "Select all",
    deselect_all: "Deselect all",
    delete: "Delete",
    add_ext_hint: "e.g. csv",
    add: "Add",
    auto_save_hint: "Multi-select supported. Settings are saved to:",
    selected_count: "Selected formats: {}",

    col_no: "No.",
    col_name: "File Name",
    col_path: "File Path",
    col_size: "Size",

    col_name_dblclick: "double-click to open",
    col_path_dblclick: "double-click to open folder",

    sheet_name: "Scan List",
    excel_bytes: "Size (bytes)",
    excel_size: "Size",
    excel_filename: "FileScanList.xlsx",

    language: "Language:",
    lang_title: "Select Language",
    lang_ok: "Start",

    search_hint: "Search file name...",
    no_match: "No files matching \"{}\"",

    mode_structure: " (folder structure)",
    mode_flat: " (flat)",
    exported_to: "Exported {}/{} documents{} to {}",
    list_ok: "; list: {}",
    list_fail: "; failed to export list: {}",
    copy_fail: "; {} file(s) failed to copy",
    bytes: "{} bytes",
};

static ZH: Tr = Tr {
    app_title: "FileExtraction",
    scan_dir_label: "扫描目录:",
    dir_hint: "点击右侧按钮选择目录",
    choose_dir: "选择目录...",
    start_scan: "开始扫描",
    export_all: "导出所有文件",
    keep_structure: "按原目录结构导出",
    pick_dir_title: "选择导出目录",

    status_pick_dir: "请选择要扫描的目录",
    status_invalid_dir: "目录无效，请重新选择",
    status_scanning: "正在扫描...",
    scanning_progress: "正在扫描，已找到 {} 个文件...",
    scan_done: "扫描完成，共找到 {} 个文件",
    status_no_result: "暂无扫描结果，请选择目录并点击\"开始扫描\"",
    status_scanning_wait: "正在扫描，请稍候...",

    invalid_ext: "无效的文件后缀: {}",
    ext_exists: "后缀 {} 已存在",
    added_ext: "已添加并保存自定义后缀",

    panel_title: "文件格式配置",
    cat_default: "默认格式",
    cat_report_template: "报告模板",
    category_progress: "{}（{}/{}）",
    select_all: "全选",
    deselect_all: "反选",
    delete: "删除",
    add_ext_hint: "如 csv",
    add: "添加",
    auto_save_hint: "支持多选，配置自动保存到：",
    selected_count: "已选格式: {} 个",

    col_no: "序号",
    col_name: "文件名称",
    col_path: "文件路径",
    col_size: "文件大小",

    col_name_dblclick: "双击打开",
    col_path_dblclick: "双击打开所在目录",

    sheet_name: "扫描清单",
    excel_bytes: "文件大小(字节)",
    excel_size: "文件大小",
    excel_filename: "文件扫描清单.xlsx",

    language: "语言:",
    lang_title: "请选择语言",
    lang_ok: "开始使用",

    search_hint: "搜索文件名...",
    no_match: "没有匹配 \"{}\" 的文件",

    mode_structure: "（按原目录结构）",
    mode_flat: "（平铺）",
    exported_to: "已导出 {}/{} 个文档{}到 {}",
    list_ok: "；清单: {}",
    list_fail: "；清单导出失败: {}",
    copy_fail: "；{} 个文件复制失败",
    bytes: "{} 字节",
};

static JA: Tr = Tr {
    app_title: "FileExtraction",
    scan_dir_label: "スキャンディレクトリ:",
    dir_hint: "右のボタンをクリックしてディレクトリを選択",
    choose_dir: "参照...",
    start_scan: "スキャン開始",
    export_all: "すべてのファイルをエクスポート",
    keep_structure: "元のフォルダ構造でエクスポート",
    pick_dir_title: "エクスポート先を選択",

    status_pick_dir: "スキャンするディレクトリを選択してください",
    status_invalid_dir: "ディレクトリが無効です。再選択してください",
    status_scanning: "スキャン中...",
    scanning_progress: "スキャン中、現在 {} 個のファイル...",
    scan_done: "スキャン完了、{} 個のファイルが見つかりました",
    status_no_result: "結果がありません。ディレクトリを選択して「スキャン開始」をクリックしてください",
    status_scanning_wait: "スキャン中です。お待ちください...",

    invalid_ext: "無効な拡張子: {}",
    ext_exists: "拡張子 {} は既に存在します",
    added_ext: "カスタム拡張子を追加して保存しました",

    panel_title: "ファイル形式設定",
    cat_default: "デフォルト形式",
    cat_report_template: "レポートテンプレート",
    category_progress: "{}（{}/{}）",
    select_all: "すべて選択",
    deselect_all: "選択を解除",
    delete: "削除",
    add_ext_hint: "例: csv",
    add: "追加",
    auto_save_hint: "複数選択可能。設定は次に保存されます:",
    selected_count: "選択された形式: {} 個",

    col_no: "番号",
    col_name: "ファイル名",
    col_path: "ファイルパス",
    col_size: "サイズ",

    col_name_dblclick: "ダブルクリックで開く",
    col_path_dblclick: "ダブルクリックでフォルダを開く",

    sheet_name: "スキャン一覧",
    excel_bytes: "サイズ（バイト）",
    excel_size: "サイズ",
    excel_filename: "スキャン一覧.xlsx",

    language: "言語:",
    lang_title: "言語を選択してください",
    lang_ok: "開始",

    search_hint: "ファイル名を検索...",
    no_match: "\"{}\" に一致するファイルはありません",

    mode_structure: "（元のフォルダ構造）",
    mode_flat: "（フラット）",
    exported_to: "{} / {} 個のドキュメント{}を {} にエクスポートしました",
    list_ok: "；一覧: {}",
    list_fail: "；一覧のエクスポートに失敗: {}",
    copy_fail: "；{} 個のファイルのコピーに失敗",
    bytes: "{} バイト",
};

static ES: Tr = Tr {
    app_title: "FileExtraction",
    scan_dir_label: "Directorio de escaneo:",
    dir_hint: "Haga clic en el botón de la derecha para elegir un directorio",
    choose_dir: "Examinar...",
    start_scan: "Iniciar escaneo",
    export_all: "Exportar todos los archivos",
    keep_structure: "Exportar con la estructura de carpetas original",
    pick_dir_title: "Elegir directorio de exportación",

    status_pick_dir: "Seleccione un directorio para escanear",
    status_invalid_dir: "Directorio no válido, elija otro",
    status_scanning: "Escaneando...",
    scanning_progress: "Escaneando, {} archivos encontrados...",
    scan_done: "Escaneo completado, {} archivos encontrados",
    status_no_result: "Sin resultados. Seleccione un directorio y haga clic en \"Iniciar escaneo\"",
    status_scanning_wait: "Escaneando, espere...",

    invalid_ext: "Extensión no válida: {}",
    ext_exists: "La extensión {} ya existe",
    added_ext: "Extensión personalizada añadida y guardada",

    panel_title: "Configuración de formatos",
    cat_default: "Formatos predeterminados",
    cat_report_template: "Plantillas de informes",
    category_progress: "{} ({}/{})",
    select_all: "Seleccionar todo",
    deselect_all: "Deseleccionar todo",
    delete: "Eliminar",
    add_ext_hint: "p. ej. csv",
    add: "Añadir",
    auto_save_hint: "Selección múltiple. La configuración se guarda en:",
    selected_count: "Formatos seleccionados: {}",

    col_no: "N.º",
    col_name: "Nombre de archivo",
    col_path: "Ruta",
    col_size: "Tamaño",

    col_name_dblclick: "doble clic para abrir",
    col_path_dblclick: "doble clic para abrir carpeta",

    sheet_name: "Lista de escaneo",
    excel_bytes: "Tamaño (bytes)",
    excel_size: "Tamaño",
    excel_filename: "ListaEscaneo.xlsx",

    language: "Idioma:",
    lang_title: "Seleccionar idioma",
    lang_ok: "Comenzar",

    search_hint: "Buscar por nombre de archivo...",
    no_match: "Ningún archivo coincide con \"{}\"",

    mode_structure: " (estructura original)",
    mode_flat: " (plano)",
    exported_to: "Exportados {}/{} documentos{} a {}",
    list_ok: "; lista: {}",
    list_fail: "; fallo al exportar la lista: {}",
    copy_fail: "; {} archivo(s) no se pudieron copiar",
    bytes: "{} bytes",
};

static FR: Tr = Tr {
    app_title: "FileExtraction",
    scan_dir_label: "Répertoire de scan :",
    dir_hint: "Cliquez sur le bouton à droite pour choisir un répertoire",
    choose_dir: "Parcourir...",
    start_scan: "Lancer le scan",
    export_all: "Exporter tous les fichiers",
    keep_structure: "Exporter avec la structure de dossiers d'origine",
    pick_dir_title: "Choisir le répertoire d'exportation",

    status_pick_dir: "Veuillez sélectionner un répertoire à scanner",
    status_invalid_dir: "Répertoire invalide, veuillez recommencer",
    status_scanning: "Scan en cours...",
    scanning_progress: "Scan en cours, {} fichiers trouvés...",
    scan_done: "Scan terminé, {} fichiers trouvés",
    status_no_result: "Aucun résultat. Sélectionnez un répertoire et cliquez sur « Lancer le scan »",
    status_scanning_wait: "Scan en cours, veuillez patienter...",

    invalid_ext: "Extension non valide : {}",
    ext_exists: "L'extension {} existe déjà",
    added_ext: "Extension personnalisée ajoutée et enregistrée",

    panel_title: "Configuration des formats",
    cat_default: "Formats par défaut",
    cat_report_template: "Modèles de rapport",
    category_progress: "{} ({}/{})",
    select_all: "Tout sélectionner",
    deselect_all: "Tout désélectionner",
    delete: "Supprimer",
    add_ext_hint: "ex. csv",
    add: "Ajouter",
    auto_save_hint: "Sélection multiple. Configuration enregistrée dans :",
    selected_count: "Formats sélectionnés : {}",

    col_no: "N°",
    col_name: "Nom du fichier",
    col_path: "Chemin",
    col_size: "Taille",

    col_name_dblclick: "double-clic pour ouvrir",
    col_path_dblclick: "double-clic pour ouvrir le dossier",

    sheet_name: "Liste de scan",
    excel_bytes: "Taille (octets)",
    excel_size: "Taille",
    excel_filename: "ListeScan.xlsx",

    language: "Langue :",
    lang_title: "Sélectionner la langue",
    lang_ok: "Démarrer",

    search_hint: "Rechercher un nom de fichier...",
    no_match: "Aucun fichier correspondant à \"{}\"",

    mode_structure: " (structure d'origine)",
    mode_flat: " (à plat)",
    exported_to: "{}/{} documents exportés{} vers {}",
    list_ok: " ; liste : {}",
    list_fail: " ; échec de l'export de la liste : {}",
    copy_fail: " ; {} fichier(s) non copié(s)",
    bytes: "{} octets",
};

static ZH_TW: Tr = Tr {
    app_title: "FileExtraction",
    scan_dir_label: "掃描目錄:",
    dir_hint: "點擊右側按鈕選擇目錄",
    choose_dir: "選擇目錄...",
    start_scan: "開始掃描",
    export_all: "匯出所有檔案",
    keep_structure: "按原目錄結構匯出",
    pick_dir_title: "選擇匯出目錄",

    status_pick_dir: "請選擇要掃描的目錄",
    status_invalid_dir: "目錄無效，請重新選擇",
    status_scanning: "正在掃描...",
    scanning_progress: "正在掃描，已找到 {} 個檔案...",
    scan_done: "掃描完成，共找到 {} 個檔案",
    status_no_result: "暫無掃描結果，請選擇目錄並點擊「開始掃描」",
    status_scanning_wait: "正在掃描，請稍候...",

    invalid_ext: "無效的檔案副檔名: {}",
    ext_exists: "副檔名 {} 已存在",
    added_ext: "已新增並儲存自訂副檔名",

    panel_title: "檔案格式設定",
    cat_default: "預設格式",
    cat_report_template: "報告範本",
    category_progress: "{}（{}/{}）",
    select_all: "全選",
    deselect_all: "反選",
    delete: "刪除",
    add_ext_hint: "如 csv",
    add: "新增",
    auto_save_hint: "支援多選，設定自動儲存至：",
    selected_count: "已選格式: {} 個",

    col_no: "序號",
    col_name: "檔案名稱",
    col_path: "檔案路徑",
    col_size: "檔案大小",

    col_name_dblclick: "雙擊開啟",
    col_path_dblclick: "雙擊開啟所在目錄",

    sheet_name: "掃描清單",
    excel_bytes: "檔案大小(位元組)",
    excel_size: "檔案大小",
    excel_filename: "檔案掃描清單.xlsx",

    language: "語言:",
    lang_title: "請選擇語言",
    lang_ok: "開始使用",

    search_hint: "搜尋檔案名稱...",
    no_match: "沒有符合 \"{}\" 的檔案",

    mode_structure: "（按原目錄結構）",
    mode_flat: "（平鋪）",
    exported_to: "已匯出 {}/{} 個檔案{}至 {}",
    list_ok: "；清單: {}",
    list_fail: "；清單匯出失敗: {}",
    copy_fail: "；{} 個檔案複製失敗",
    bytes: "{} 位元組",
};
