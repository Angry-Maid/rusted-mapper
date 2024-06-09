use serde::{self, Deserialize, Serialize};

use crate::built_info;

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Mapper {}

impl Default for Mapper {
    fn default() -> Self {
        Self {}
    }
}

impl Mapper {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Default::default()
    }
}

impl eframe::App for Mapper {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.add_space(16.0);
                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Rusted Warden Mapper");

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    egui::warn_if_debug_build(ui);
                    ui.label(format!(
                        "{} {}{}",
                        built_info::PKG_VERSION,
                        built_info::GIT_COMMIT_HASH_SHORT.unwrap(),
                        if built_info::GIT_DIRTY.unwrap() {
                            "(dirty)"
                        } else {
                            ""
                        },
                    ));
                });
                ui.separator();
            });
        });
    }
}
