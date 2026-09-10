use eframe::egui;

/// Look for a usable CJK font file in the Windows system font directory
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
    // Fallback: scan the Fonts directory and pick the first ttf/ttc/otf whose
    // name looks like a common CJK font
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

/// Register a CJK font at the end of the egui Proportional / Monospace font
/// fallback chain so that missing glyphs fall back automatically and CJK text
/// renders correctly
pub fn setup_cjk_fonts(ctx: &egui::Context) {
    let Some(bytes) = load_cjk_font_bytes() else {
        eprintln!("CJK font not found; CJK text may render as boxes");
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
