use std::{
    sync::{mpsc::TryRecvError, Arc},
    time::Duration,
};

use egui::{scroll_area::ScrollBarVisibility, Align, Color32, Frame, ScrollArea};
use itertools::zip;
use log::debug;
use rm_core::parser::{Level, Parser};
use serde::{self, Deserialize, Serialize};

use crate::built_info;

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Mapper {
    #[serde(skip)]
    parser: Parser,
    #[serde(skip)]
    log_buffer: Vec<String>,
    #[serde(skip)]
    scroll_to_bottom: bool,
    #[serde(skip)]
    seeds: Option<[u32; 3]>,
    #[serde(skip)]
    expedition: Option<Arc<Level>>,
}

impl Default for Mapper {
    fn default() -> Self {
        Self {
            parser: Parser::new(None),
            scroll_to_bottom: true,
            log_buffer: Default::default(),
            seeds: None,
            expedition: Default::default(),
        }
    }
}

impl Mapper {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut s: Mapper = Default::default();

        s.parser.start_watcher().unwrap();

        s
    }
}

impl eframe::App for Mapper {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(25));

        let data_msg = &self.parser.rx.as_ref().unwrap().try_recv();
        match data_msg {
            Ok(msg) => match msg {
                rm_core::parser::ParserMsg::NewFile => {
                    self.log_buffer.clear();
                    self.seeds = None;
                    self.expedition = None;
                }
                rm_core::parser::ParserMsg::LevelSeeds(build_seed, host_seed, session_seed) => {
                    self.seeds = Some([*build_seed, *host_seed, *session_seed]);
                }
                rm_core::parser::ParserMsg::LevelInit(level) => {
                    self.expedition = Some(level.to_owned());
                }
                // rm_core::parser::ParserMsg::Gatherable(_) => todo!(),
                // rm_core::parser::ParserMsg::LevelStart => todo!(),
                // rm_core::parser::ParserMsg::ZoneDoorOpened => todo!(),
                // rm_core::parser::ParserMsg::LevelFinish => todo!(),
                _ => {
                    debug!("{msg:?}");
                }
            },
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => debug!("Got disconnect from tail_data_rx"),
        }

        egui::TopBottomPanel::top("top_panel")
            .frame(Frame {
                fill: Color32::TRANSPARENT,
                ..Default::default()
            })
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(8.0);
                    ui.checkbox(&mut self.scroll_to_bottom, "Autoscroll to Bottom");
                });
            });

        egui::TopBottomPanel::bottom("btm_panel")
            .frame(Frame {
                fill: Color32::TRANSPARENT,
                ..Default::default()
            })
            .show(ctx, |ui| {
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

        egui::CentralPanel::default()
            .frame(Frame {
                fill: Color32::TRANSPARENT,
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.heading("Rusted Warden Mapper");

                ui.separator();

                if let Some(values) = &self.seeds {
                    ui.vertical(|ui| {
                        for (label, seed) in
                            zip(vec!["Build Seed", "Host Seed", "Session Seed"], values)
                        {
                            ui.label(format!("{label}: {seed}"));
                        }
                    });
                }

                if let Some(level) = &self.expedition {
                    ui.label(format!("Selected Expedition: {}", level.display_name()));
                }

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
                        if self.scroll_to_bottom {
                            ui.scroll_to_cursor(Some(Align::BOTTOM));
                        }
                    });
            });
    }
}
