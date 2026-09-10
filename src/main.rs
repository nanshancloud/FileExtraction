#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod exporter;
mod fonts;
mod i18n;
mod scanner;

use crate::app::FileSearchApp;
use crate::fonts::setup_cjk_fonts;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1150.0, 660.0])
            .with_title(i18n::Lang::En.tr().app_title),
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
