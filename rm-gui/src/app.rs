use std::{iter::zip, sync::mpsc::TryRecvError, time::Duration};

use egui::{scroll_area::ScrollBarVisibility, Align, Color32, Frame, ScrollArea};
use log::debug;
use rm_core::{
    data::{GatherItem, Level, TimerEntry},
    parser::{Parser, ParserMsg},
};
use serde::{self, Deserialize, Serialize};

use crate::built_info;

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Mapper {
    #[serde(skip)]
    parser: Parser,
    #[serde(skip)]
    scroll_to_bottom: bool,
    #[serde(skip)]
    seeds: Option<[u32; 3]>,
    #[serde(skip)]
    expedition: Option<Level>,
    #[serde(skip)]
    gatherables: Vec<GatherItem>,
}

impl Default for Mapper {
    fn default() -> Self {
        Self {
            parser: Parser::new(None),
            scroll_to_bottom: true,
            seeds: None,
            expedition: Default::default(),
            gatherables: Default::default(),
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
                ParserMsg::NewFile => {
                    self.gatherables.clear();
                    self.seeds = None;
                    self.expedition = None;
                }
                ParserMsg::LevelSeeds(build_seed, host_seed, session_seed) => {
                    self.seeds = Some([*build_seed, *host_seed, *session_seed]);
                }
                ParserMsg::LevelInit(level) => {
                    self.expedition = Some(level.to_owned());
                }
                ParserMsg::GeneratedZone(zone) => {
                    self.expedition
                        .as_mut()
                        .unwrap()
                        .timer_zones
                        .push(zone.to_owned());
                    if let TimerEntry::Zone(z) = zone {
                        self.expedition.as_mut().unwrap().zones.push(z.to_owned())
                    }
                }
                ParserMsg::Gatherable(gatherable) => {
                    self.gatherables.push(gatherable.to_owned());
                }
                // ParserMsg::LevelStart => todo!(),
                // ParserMsg::ZoneDoorOpened => todo!(),
                // ParserMsg::LevelFinish => todo!(),
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

                ScrollArea::vertical()
                    .auto_shrink(false)
                    .scroll_bar_visibility(ScrollBarVisibility::VisibleWhenNeeded)
                    .show(ui, |ui| {
                        ui.with_layout(
                            egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                            |ui| {
                                if let Some(level) = &self.expedition {
                                    ui.label(format!("Selected Expedition: {}", level));
                                    for zone in &level.timer_zones {
                                        match zone {
                                            TimerEntry::Start => {
                                                ui.label("Start");
                                            }
                                            TimerEntry::Zone(z) => {
                                                ui.label(format!(
                                                    "ZONE_{} {} {}",
                                                    z.alias, z.layer, z.dimension
                                                ));
                                            }
                                            TimerEntry::Custom(s) => {
                                                ui.label(s);
                                            }
                                            TimerEntry::Invariance(_, _) => {}
                                            TimerEntry::End => {
                                                ui.label("End");
                                            }
                                        }
                                    }
                                    for gatherable in &self.gatherables {
                                        match gatherable {
                                            GatherItem::Key(_, dim, alias, _) => {
                                                ui.label(format!(
                                                    "{} {:?}",
                                                    level[(alias.to_owned(), dim.to_owned())],
                                                    gatherable
                                                ));
                                            }
                                            GatherItem::Seeded(container, seed) => {
                                                ui.label(format!("{} {}", container, seed));
                                            }
                                            other => {
                                                ui.label(format!("{other:?}"));
                                            }
                                        }
                                    }
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
