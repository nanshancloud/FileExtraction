#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod exporter;
mod fonts;
mod i18n;
mod icon;
mod scanner;

use crate::app::FileExtractionApp;
use crate::fonts::setup_cjk_fonts;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1150.0, 660.0])
        .with_title(i18n::Lang::En.tr().app_title);

    // Window icon: shown in the taskbar and the window title bar
    if let Some(logo) = icon::load_logo() {
        viewport = viewport.with_icon(egui::IconData {
            rgba: logo.rgba,
            width: logo.width,
            height: logo.height,
        });
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "FileExtraction",
        options,
        Box::new(|cc| {
            setup_cjk_fonts(&cc.egui_ctx);
            Ok(Box::new(FileExtractionApp::new()))
        }),
    )
}
