#![feature(duration_constructors)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

use rm_gui::built_info;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([330.0, 550.0])
            .with_min_inner_size([330.0, 550.0])
            .with_position([1.0, 300.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_window_level(egui::WindowLevel::AlwaysOnTop),
        ..Default::default()
    };

    eframe::run_native(
        built_info::PKG_NAME,
        native_options,
        Box::new(|cc| Box::new(rm_gui::Mapper::new(cc))),
    )
}
