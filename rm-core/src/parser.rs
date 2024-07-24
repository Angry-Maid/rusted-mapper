use std::{
    marker,
    path::{Path, PathBuf},
    sync::mpsc::{channel, Receiver, Sender, TryRecvError},
    thread,
    time::Duration,
};

use log::{error, info};
use might_sleep::cpu_limiter::CpuLimiter;
use notify::{
    event::CreateKind, recommended_watcher, Error, Event, RecommendedWatcher, RecursiveMode,
    Watcher,
};
use walkdir::WalkDir;

use crate::{
    data::{GatherItem, ItemIdentifier, Level, Rundown, TimerEntry, Zone},
    re,
    tail::{Tail, TailCmd, TailMsg},
};

#[derive(Debug)]
pub struct Parser {
    watch_path: PathBuf,
    dir_watcher: Option<RecommendedWatcher>,
    pub tail_cmd_tx: Option<Sender<TailCmd>>,
    pub rx: Option<Receiver<ParserMsg>>,
}

#[derive(Debug)]
pub enum ParserMsg {
    LevelSeeds(u32, u32, u32),
    LevelInit(Level),
    GeneratedZone(TimerEntry),
    Gatherable(GatherItem),
    LevelStart,
    ZoneDoorOpened,
    LevelFinish,

    NewFile,
}

#[derive(Debug, Default)]
enum ParserState {
    #[default]
    LevelSeeds,
    LevelSelected,
    LevelGeneration,
    ItemGeneration,
    ElevatorDropFinish,
    LevelFinish,
    NotInLevel,
}

#[derive(Debug)]
struct ParserManager {
    pub buffer: String,
    pub pos: usize,
    pub state: ParserState,
}

impl Default for ParserManager {
    fn default() -> Self {
        Self {
            buffer: "".into(),
            pos: 0,
            state: Default::default(),
        }
    }
}

impl Parser {
    pub fn new(watch_path: Option<PathBuf>) -> Self {
        let profile_path = if let Some(path) = watch_path {
            path
        } else {
            Path::new(env!("USERPROFILE")).join("appdata\\locallow\\10 Chambers Collective\\GTFO")
        };

        Parser {
            watch_path: profile_path,
            dir_watcher: None,
            tail_cmd_tx: None,
            rx: None,
        }
    }

    pub fn start_watcher(&mut self) -> anyhow::Result<()> {
        let (command_tx, data_rx): (Sender<TailCmd>, Receiver<TailMsg>) = Tail::start_listen()?;

        let (parser_tx, parser_rx) = channel::<ParserMsg>();

        self.tail_cmd_tx = Some(command_tx.clone());
        self.rx = Some(parser_rx);

        thread::Builder::new()
            .name("parser".into())
            .spawn(|| Parser::parser(data_rx, parser_tx))?;

        // We first look for `NICKNAME_NETSTATUS` file in case
        // rusted-mapper was opened after the game was open.
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
            info!("{:?}", entry.file_name());
            if entry
                .file_name()
                .to_str()
                .map_or(false, |v| v.contains("NICKNAME_NETSTATUS"))
            {
                command_tx.send(TailCmd::Open(entry.path().to_path_buf()))?;
                break;
            }
        }

        let mut watcher = recommended_watcher(move |res: Result<Event, Error>| match res {
            Ok(event) => {
                info!("{:?} {:?} {:?}", event.kind, event.attrs, event.paths);
                if let notify::EventKind::Create(CreateKind::Any) = event.kind {
                    if let Some(path) = event.paths.first() {
                        if let Some(filename) = path.file_name() {
                            if filename
                                .to_str()
                                .map_or(false, |v| v.contains("NICKNAME_NETSTATUS"))
                            {
                                command_tx.send(TailCmd::Open(path.to_path_buf())).unwrap();
                            }
                        }
                    }
                }
            }
            Err(e) => error!("{e:?}"),
        })?;

        watcher.watch(self.watch_path.as_path(), RecursiveMode::NonRecursive)?;

        self.dir_watcher = Some(watcher);

        Ok(())
    }

    pub fn stop_tail(&mut self) -> anyhow::Result<()> {
        self.tail_cmd_tx.clone().unwrap().send(TailCmd::Stop)?;

        Ok(())
    }

    pub fn parser(data_rx: Receiver<TailMsg>, parser_tx: Sender<ParserMsg>) -> anyhow::Result<()> {
        let mut limiter = CpuLimiter::new(Duration::from_millis(250));
        let mut parser_manager = ParserManager::default();

        loop {
            match data_rx.try_recv() {
                Ok(val) => {
                    // For now we get the message and propagate it back
                    match val {
                        TailMsg::Content(s) => {
                            parser_manager.buffer.push_str(s.as_str());
                        }
                        TailMsg::NewFile => {
                            parser_manager.buffer.clear();
                            parser_manager.state = ParserState::LevelSeeds;
                            parser_tx.send(ParserMsg::NewFile)?;
                        }
                        TailMsg::Stop => break,
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    error!("Got disconnect from data channel");
                    break;
                }
            }

            // Check for level end trigger/level de-init.
            if false {
                todo!();
                continue;
            }

            match parser_manager.state {
                ParserState::LevelSeeds => {
                    if let Some(ref cap) = re::BUILDER_LEVEL_SEEDS
                        .captures_iter(&parser_manager.buffer)
                        .last()
                    {
                        let (_, [build_seed, host_seed, session_seed]) = cap.extract();
                        parser_manager.pos = cap.get(0).unwrap().end();

                        let build_seed = build_seed.parse::<u32>()?;
                        let host_seed = host_seed.parse::<u32>()?;
                        let session_seed = session_seed.parse::<u32>()?;

                        parser_tx.send(ParserMsg::LevelSeeds(
                            build_seed,
                            host_seed,
                            session_seed,
                        ))?;

                        parser_manager.state = ParserState::LevelSelected;
                    }
                }
                ParserState::LevelSelected => {
                    if let Some(cap) = re::DROP_SERVER_MANAGER_NEW_SESSION
                        .captures_iter(&parser_manager.buffer)
                        .last()
                    {
                        let (_, [rundown_idx, rundown_exp]) = cap.extract();
                        parser_manager.pos = cap.get(0).unwrap().end();

                        let rundown_idx = rundown_idx.parse::<u16>()?;
                        let rundown_exp = rundown_exp.to_string();

                        let level = Level {
                            rundown: Rundown::from_repr(rundown_idx).unwrap_or(Rundown::Modded),
                            exp_name: rundown_exp.clone(),
                            ..Default::default()
                        };

                        parser_tx.send(ParserMsg::LevelInit(level))?;

                        parser_manager.state = ParserState::LevelGeneration;
                    }
                }
                ParserState::LevelGeneration => {
                    // TODO: add check if level already exists as file and load zones from file

                    let (batch_start, batch_end) = (
                        re::SETUP_FLOOR_BATCH_START
                            .captures_iter(&parser_manager.buffer)
                            .last()
                            .and_then(|c| c.get(0))
                            .map(|m| m.start()),
                        re::SETUP_FLOOR_BATCH_END
                            .captures_iter(&parser_manager.buffer)
                            .last()
                            .and_then(|c| c.get(0))
                            .map(|m| m.end()),
                    );

                    if let (Some(start), Some(end)) = (batch_start, batch_end) {
                        parser_tx.send(ParserMsg::GeneratedZone(TimerEntry::Start))?;
                        for cap in
                            re::ZONE_CREATED.captures_iter(&parser_manager.buffer[start..end])
                        {
                            let (_, [alias, local, dim, layer]) = cap.extract();
                            parser_tx.send(ParserMsg::GeneratedZone(TimerEntry::Zone(Zone {
                                alias: alias.parse::<u32>()?,
                                local: local.parse::<u32>()?,
                                dimension: dim.to_string(),
                                layer: layer.to_string(),
                                area: None,
                            })))?;
                        }

                        parser_tx.send(ParserMsg::GeneratedZone(TimerEntry::End))?;

                        parser_manager.state = ParserState::ItemGeneration;
                    }
                }
                ParserState::ItemGeneration => {
                    // TODO: Biggest state yet
                    // General work that we need to do here:
                    // - Parse for gatherable items (any _other_ gatherable that we can encounter) and record their zones and count
                    // - Parse the information for mappable items like keys - 1st Variant
                    // - Parse the information for mappable items that have item seed - 2nd Variant
                    // - Parse the information for generators if we have generator objective - 3rd Variant

                    let (
                        distribution_batch_start,
                        distribution_batch_end,
                        marker_batch_start,
                        marker_batch_end,
                    ) = (
                        re::DISTRIBUTION_BATCH_START
                            .captures_iter(&parser_manager.buffer)
                            .last()
                            .and_then(|c| c.get(0))
                            .map(|m| m.start()),
                        re::DISTRIBUTION_BATCH_END
                            .captures_iter(&parser_manager.buffer)
                            .last()
                            .and_then(|c| c.get(0))
                            .map(|m| m.end()),
                        re::FUNCTION_MARKERS_BATCH_START
                            .captures_iter(&parser_manager.buffer)
                            .last()
                            .and_then(|c| c.get(0))
                            .map(|m| m.start()),
                        re::FUNCTION_MARKERS_BATCH_END
                            .captures_iter(&parser_manager.buffer)
                            .last()
                            .and_then(|c| c.get(0))
                            .map(|m| m.end()),
                    );

                    if let (
                        Some(distribution_start),
                        Some(distribution_end),
                        Some(marker_start),
                        Some(marker_end),
                    ) = (
                        distribution_batch_start,
                        distribution_batch_end,
                        marker_batch_start,
                        marker_batch_end,
                    ) {
                        let distribution_segment =
                            &parser_manager.buffer[distribution_start..distribution_end];
                        let marker_segment = &parser_manager.buffer[marker_start..marker_end];

                        // Keys
                        for cap in
                            re::CREATE_KEY_ITEM_DISTRIBUTION.captures_iter(distribution_segment)
                        {
                            let (_, [key, dim, _, alias, ri]) = cap.extract();
                            parser_tx.send(ParserMsg::Gatherable(GatherItem::Key(
                                key.into(),
                                dim.into(),
                                alias.parse()?,
                                ri.parse()?,
                            )))?;
                        }

                        let mut collectibles: Vec<ItemIdentifier> = vec![];

                        for cap in
                            re::DISTRIBUTE_WARDEN_OBJECTIVE.captures_iter(distribution_segment)
                        {
                            let (_, [alias, idx, item]) = cap.extract();

                            collectibles.push(
                                match ItemIdentifier::from_repr(item.parse()?).unwrap() {
                                    ItemIdentifier::DataCube | ItemIdentifier::DataCubeR8 => {
                                        ItemIdentifier::DataCube
                                    }
                                    other => other,
                                },
                            );
                        }

                        let mut seeded_collectibles = collectibles.iter().filter(|x| {
                            matches!(
                                x,
                                ItemIdentifier::ID
                                    | ItemIdentifier::PD
                                    | ItemIdentifier::GLP1
                                    | ItemIdentifier::OSIP
                                    | ItemIdentifier::PlantSample
                                    | ItemIdentifier::DataCube
                                    | ItemIdentifier::DataCubeR8
                                    | ItemIdentifier::GLP2
                            )
                        });

                        for cap in re::GENERIC_SMALL_PICKUP_ITEM.captures_iter(marker_segment) {
                            let (_, [container, seed]) = cap.extract();
                            let seed = seed.parse::<u32>()?;

                            let item = seeded_collectibles.next();

                            let collectible = match item {
                                Some(item) => match item {
                                    ItemIdentifier::ID => GatherItem::ID(container.into(), seed),
                                    ItemIdentifier::PD => GatherItem::PD(container.into(), seed),
                                    ItemIdentifier::GLP1 => {
                                        GatherItem::GLP1(container.into(), seed)
                                    }
                                    ItemIdentifier::OSIP => {
                                        GatherItem::OSIP(container.into(), seed)
                                    }
                                    ItemIdentifier::PlantSample => {
                                        GatherItem::PlantSample(container.into(), seed)
                                    }
                                    ItemIdentifier::DataCube | ItemIdentifier::DataCubeR8 => {
                                        GatherItem::DataCube(container.into(), seed)
                                    }
                                    ItemIdentifier::GLP2 => {
                                        GatherItem::GLP2(container.into(), seed)
                                    }
                                    _ => GatherItem::Seeded(container.into(), seed),
                                },
                                None => GatherItem::Seeded(container.into(), seed),
                            };

                            parser_tx.send(ParserMsg::Gatherable(collectible))?;
                        }

                        for (i, cap) in re::WARDEN_OBJECTIVE_MANAGER
                            .captures_iter(marker_segment)
                            .enumerate()
                        {
                            dbg!(cap);
                        }

                        parser_manager.state = ParserState::ElevatorDropFinish;
                    }
                }
                ParserState::ElevatorDropFinish => {
                    // TODO: ;_;
                }
                ParserState::LevelFinish => {}
                ParserState::NotInLevel => {}
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
