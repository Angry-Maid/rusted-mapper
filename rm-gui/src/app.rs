use std::{sync::mpsc::TryRecvError, time::Duration};

use egui::{scroll_area::ScrollBarVisibility, Align, ScrollArea};
use log::debug;
use rm_core::parser::Parser;
use serde::{self, Deserialize, Serialize};

use crate::built_info;

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Mapper {
    #[serde(skip)]
    parser: Parser,
    #[serde(skip)]
    log_buffer: Vec<String>,
}

impl Default for Mapper {
    fn default() -> Self {
        Self {
            parser: Parser::new(None),
            log_buffer: Default::default(),
        }
    }
}

impl Mapper {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut s: Mapper = Default::default();

        s.parser.start_watcher();

        s
    }
}

impl eframe::App for Mapper {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(100));

        let data = &self.parser.tail_data_rx.as_ref().unwrap().try_recv();
        match data {
            Ok(d) => self.log_buffer.push(d.to_owned()),
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => debug!("Got disconnect from tail_data_rx"),
        }

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

            ScrollArea::vertical()
                .auto_shrink(false)
                .scroll_bar_visibility(ScrollBarVisibility::VisibleWhenNeeded)
                .show(ui, |ui| {
                    ui.with_layout(
                        egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                        |ui| {
                            for line in &self.log_buffer {
                                ui.label(line.to_owned());
                            }
                        },
                    );
                    ui.scroll_to_cursor(Some(Align::BOTTOM));
                });

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    egui::warn_if_debug_build(ui);
                    ui.label(format!(
                        "{}{}",
                        built_info::GIT_VERSION.unwrap(),
                        if built_info::GIT_DIRTY.unwrap() {
                            "(dirty)"
                        } else {
                            ""
                        },
                    ));
                });
            });
        });
    }
}
