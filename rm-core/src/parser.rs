use std::{
    path::{Path, PathBuf},
    sync::mpsc::{Receiver, Sender, TryRecvError, channel},
    thread,
    time::Duration,
};

use itertools::Itertools;
use jiff::civil::Time;
use log::{error, info};
use might_sleep::prelude::CpuLimiter;
use notify::{RecommendedWatcher, Watcher, event::CreateKind, recommended_watcher};
use walkdir::WalkDir;

use crate::{
    GatherItem, ItemIdentifier, Level, Rundown, Token, Zone, re,
    tail::{Tail, TailCmd, TailMsg},
};

#[derive(Debug)]
pub struct ParserMsg(pub Option<Time>, pub Token);

#[derive(Debug)]
pub struct Parser {
    watch_path: PathBuf,
    dir_watcher: Option<RecommendedWatcher>,
    tail_cmd: Option<Sender<TailCmd>>,
}

impl Parser {
    pub fn new(watch_path: Option<PathBuf>) -> Self {
        let watch = if let Some(path) = watch_path {
            path
        } else {
            Path::new(env!("USERPROFILE")).join("appdata\\locallow\\10 Chambers Collective\\GTFO")
        };

        Self {
            watch_path: watch,
            dir_watcher: None,
            tail_cmd: None,
        }
    }

    pub fn start_watcher(&mut self, parser_tx: Sender<ParserMsg>) -> anyhow::Result<()> {
        let (command_tx, command_rx) = channel::<TailCmd>();
        let (data_tx, data_rx) = channel::<TailMsg>();

        self.tail_cmd = Some(command_tx.clone());

        Tail::start_listen(command_rx, data_tx)?;

        thread::Builder::new()
            .name("parser".into())
            .spawn(|| Parser::parser(data_rx, parser_tx))?;

        for entry in WalkDir::new(self.watch_path.clone().as_path())
            .min_depth(1)
            .max_depth(1)
            .sort_by(|a, b| {
                b.metadata()
                    .unwrap()
                    .modified()
                    .unwrap()
                    .cmp(&a.metadata().unwrap().modified().unwrap())
            })
            .into_iter()
            .flatten()
        {
            info!("{:?}", entry);
            if entry
                .file_name()
                .to_str()
                .is_some_and(|v| v.contains("NICKNAME_NETSTATUS"))
            {
                command_tx.send(TailCmd::Open(entry.path().to_path_buf()))?;
                break;
            }
        }

        let mut watcher =
            recommended_watcher(move |res: Result<notify::Event, notify::Error>| match res {
                Ok(event) => {
                    info!("{:?} {:?} {:?}", event.kind, event.attrs, event.paths);
                    if let notify::EventKind::Create(CreateKind::Any) = event.kind {
                        if let Some(path) = event.paths.first() {
                            if let Some(filename) = path.file_name() {
                                if filename
                                    .to_str()
                                    .is_some_and(|v| v.contains("NICKNAME_NETSTATUS"))
                                {
                                    command_tx.send(TailCmd::Open(path.to_path_buf())).unwrap();
                                }
                            }
                        }
                    }
                }
                Err(e) => error!("{e:?}"),
            })?;

        watcher.watch(&self.watch_path, notify::RecursiveMode::NonRecursive)?;

        self.dir_watcher = Some(watcher);

        Ok(())
    }

    pub fn stop_tail(&mut self) -> anyhow::Result<()> {
        self.tail_cmd.clone().unwrap().send(TailCmd::Stop)?;

        Ok(())
    }

    pub fn parser(data_rx: Receiver<TailMsg>, parser_tx: Sender<ParserMsg>) -> anyhow::Result<()> {
        let mut limiter = CpuLimiter::new(Duration::from_millis(250));

        loop {
            match data_rx.try_recv() {
                Ok(val) => {
                    match val {
                        TailMsg::Content(s) => {
                            let mut lines = s.lines().peekable();
                            while lines.peek().is_some() {
                                let line = lines.next().unwrap();
                                // Check for End Level
                                if line.ends_with("OnApplicationQuit")
                                    || ["ExpeditionAbort", "AfterLevel", "Lobby", "NoLobby"]
                                        .iter()
                                        .any(|e| {
                                            *e == re::GAMESTATE_MANAGER
                                                .captures(line)
                                                .map_or("", |c| {
                                                    c.name("new_state").unwrap().as_str()
                                                })
                                        })
                                {
                                    parser_tx.send(ParserMsg(None, Token::Reset))?;
                                }

                                // Level Seeds
                                if let Some(cap) = re::BUILDER_LEVEL_SEEDS.captures(line) {
                                    let (_, [time, build_seed, host_seed, session_seed]) =
                                        cap.extract();

                                    parser_tx.send(ParserMsg(
                                        time.parse::<Time>().ok(),
                                        Token::Seeds(
                                            build_seed.parse::<u32>()?,
                                            host_seed.parse::<u32>()?,
                                            session_seed.parse::<u32>()?,
                                        ),
                                    ))?;
                                }

                                // Rundown and Level
                                if let Some(cap) =
                                    re::DROP_SERVER_MANAGER_NEW_SESSION.captures(line)
                                {
                                    info!("{line}");

                                    let (_, [time, rundown_idx, tier, exp_idx]) = cap.extract();

                                    let rundown = Rundown::from_repr(rundown_idx.parse()?)
                                        .unwrap_or(Rundown::Modded);
                                    let tier = tier.to_string();
                                    let exp: usize = exp_idx.parse()?;

                                    parser_tx.send(ParserMsg(
                                        time.parse::<Time>().ok(),
                                        Token::Expedition(
                                            rundown.clone(),
                                            tier.clone(),
                                            if (rundown == Rundown::R8
                                                && ["A", "C", "D", "E"].contains(&tier.as_str())
                                                && exp == 2)
                                            {
                                                exp
                                            } else {
                                                exp + 1
                                            },
                                        ),
                                    ))?;
                                }

                                // Zones
                                if line.contains("LG_Floor.CreateZone") {
                                    let zone = format!("{}\n{}", line, lines.next().unwrap());
                                    if let Some(cap) = re::ZONE_CREATED.captures(zone.as_str()) {
                                        let (_, [alias, local, dim, layer]) = cap.extract();
                                        parser_tx.send(ParserMsg(
                                            None,
                                            Token::Zone(Zone {
                                                alias: alias.parse::<u32>()?,
                                                local: local.parse::<u32>()?,
                                                dimension: dim.to_string(),
                                                layer: layer.to_string(),
                                                area: None,
                                            }),
                                        ))?;
                                    }
                                }

                                // Keys
                                if line.contains("CreateKeyItemDistribution") {
                                    let key = format!(
                                        "{}\n{}",
                                        line,
                                        lines
                                            .by_ref()
                                            .take_while_inclusive(|l| {
                                                !l.contains(
                                            "TryGetExistingGenericFunctionDistributionForSession",
                                        )
                                            })
                                            .join("\n")
                                    );

                                    if let Some(cap) =
                                        re::CREATE_KEY_ITEM_DISTRIBUTION.captures(key.as_str())
                                    {
                                        let (_, [key_name, dim, local, alias, ri]) = cap.extract();

                                        let key = GatherItem::Key(
                                            key_name.parse()?,
                                            dim.parse()?,
                                            alias.parse()?,
                                            ri.parse()?,
                                        );

                                        parser_tx.send(ParserMsg(
                                            None,
                                            Token::Gatherable(
                                                Some(alias.parse()?),
                                                Some(dim.parse()?),
                                                key,
                                            ),
                                        ))?;
                                    }
                                }

                                // HSU
                                if line.contains("HydroStatisUnit for wardenObjectiveType") {
                                    if let Some(cap) = re::DISTRIBUTE_HSU.captures(line) {
                                        let (_, [alias, id, area]) = cap.extract();

                                        let hsu = GatherItem::HSU(id.parse()?);

                                        parser_tx.send(ParserMsg(
                                            None,
                                            Token::Gatherable(None, None, hsu),
                                        ))?;
                                    }
                                }

                                // Other gatherables: GLPS, IDs, PDs
                            }
                        }
                        TailMsg::NewFile => parser_tx.send(ParserMsg(None, Token::Reset))?,
                        TailMsg::Stop => todo!(),
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    error!("data channel was disconnected");
                    break;
                }
            }

            limiter.might_sleep();
        }

        Ok(())
    }
}

impl Drop for Parser {
    fn drop(&mut self) {
        self.stop_tail().unwrap();
    }
}
