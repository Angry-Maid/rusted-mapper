use std::{
    sync::mpsc::{Receiver, TryRecvError, channel},
    time::Duration,
};

use egui::{Color32, Frame, ScrollArea};
use itertools::Itertools;
use log::{debug, info};
use rm_core::{
    GatherItem, Level, Token,
    parser::{Parser, ParserMsg},
};

pub struct Mapper {
    parser: Parser,
    seeds: Option<[u32; 3]>,
    level: Level,
    scroll_to_bottom: bool,
    parser_rx: Option<Receiver<ParserMsg>>,
}

impl Default for Mapper {
    fn default() -> Self {
        Self {
            parser: Parser::new(None),
            seeds: None,
            level: Level::default(),
            scroll_to_bottom: true,
            parser_rx: None,
        }
    }
}

impl Mapper {
    pub fn new(_: &eframe::CreationContext<'_>) -> Self {
        let mut s: Mapper = Default::default();

        let (parser_tx, parser_rx) = channel::<ParserMsg>();

        s.parser_rx = Some(parser_rx);

        s.parser.start_watcher(parser_tx).unwrap();

        s
    }
}

impl eframe::App for Mapper {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(25));

        match &self.parser_rx.as_ref().unwrap().try_recv() {
            Ok(ParserMsg(time, token)) => {
                info!("{token:?}");
                match token {
                    Token::Seeds(build_seed, host_seed, session_seed) => {
                        self.seeds.replace([
                            build_seed.to_owned(),
                            host_seed.to_owned(),
                            session_seed.to_owned(),
                        ]);
                    }
                    Token::Expedition(rundown, tier, exp) => {
                        self.level.rundown = Some(rundown.to_owned());
                        self.level.tier = Some(tier.to_owned());
                        self.level.exp = Some(exp.to_owned());
                    }
                    Token::Zone(zone) => {
                        self.level.zones.push(zone.to_owned());
                    }
                    Token::Start => todo!(),
                    Token::Split => todo!(),
                    Token::End => todo!(),
                    Token::Gatherable(Some(local_idx), Some(dim), gather_item) => {
                        self.level.gathatable_items.insert(
                            self.level[(local_idx.to_owned(), dim.to_owned())].clone(),
                            gather_item.to_owned(),
                        );
                    }
                    Token::Gatherable(None, None, gather_item) => {
                        self.level.gatherables.push(gather_item.to_owned());
                    }
                    Token::Uncategorized(item_identifier, _) => {
                        self.level.uncategorized.push(item_identifier.to_owned());
                    }
                    Token::Reset => {
                        // TODO: Save level before clearing it.

                        self.seeds = None;
                        self.level.zones.clear();
                        self.level.gathatable_items.clear();
                        self.level.gatherables.clear();
                    }
                    _ => {}
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => debug!("Got disconnect from parser channel"),
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
                    ui.checkbox(&mut self.scroll_to_bottom, "Autoscroll to Bottom")
                })
            });

        egui::TopBottomPanel::bottom("bottom_panel")
            .frame(Frame {
                fill: Color32::TRANSPARENT,
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.horizontal(|ui| {
                        egui::warn_if_debug_build(ui);
                        ui.label(env!("CARGO_PKG_VERSION"));
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

                ScrollArea::vertical()
                    .auto_shrink(false)
                    .scroll_bar_visibility(
                        egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                    )
                    .show(ui, |ui| {
                        ui.with_layout(
                            egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                            |ui| {
                                ui.label(format!("{}", self.level));

                                ui.separator();

                                if let Some(seeds) = &self.seeds {
                                    for seed in seeds {
                                        ui.label(format!("{seed}"));
                                    }
                                    ui.separator();
                                }

                                for (zone, gatherable) in
                                    self.level.gathatable_items.iter().sorted()
                                {
                                    match gatherable {
                                        GatherItem::Key(name, _, zone_alias, ri) => {
                                            ui.label(format!("{name} - ID {ri}"))
                                        }
                                        GatherItem::Seeded(container, seed) => {
                                            ui.label(format!("{container} {seed}"))
                                        }
                                        other => ui.label(format!("{other:?}")),
                                    };
                                }
                                for gatherable in &self.level.gatherables {
                                    match gatherable {
                                        GatherItem::Key(name, _, zone_alias, ri) => {
                                            ui.label(format!("{name} - {ri}"))
                                        }
                                        GatherItem::Seeded(container, seed) => {
                                            ui.label(format!("{container} {seed}"))
                                        }
                                        GatherItem::HSU(id) => ui.label(format!("HSU - ID {id}")),
                                        other => ui.label(format!("{other:?}")),
                                    };
                                }
                            },
                        )
                    })
            });
    }
}
