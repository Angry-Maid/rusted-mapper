use std::{
    fmt::Display,
    path::{Path, PathBuf},
    sync::{
        mpsc::{channel, Receiver, Sender, TryRecvError},
        Arc,
    },
    thread,
    time::Duration,
};

use chrono::NaiveTime;
use glam::Vec2;
use log::{error, info};
use might_sleep::cpu_limiter::CpuLimiter;
use notify::{
    event::CreateKind, recommended_watcher, Error, Event, RecommendedWatcher, RecursiveMode,
    Watcher,
};
use serde::{Deserialize, Serialize};
use strum::FromRepr;
use walkdir::WalkDir;

use crate::tail::{Tail, TailCmd, TailMsg};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Zone {
    pub alias: u32,
    pub local: u32,
    pub dimension: String,
    pub layer: String,
    pub area: Option<char>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Record {
    pub time: NaiveTime,
    pub item: Option<GatherItem>,
    pub zone: Option<Zone>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TimerEntry {
    Start,
    Zone(Zone),
    Invariance(Vec<Zone>, InvarianceMethod),
    End,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub enum InvarianceMethod {
    #[default]
    All,
    /// Number of Zones, filter Item, max num of Item
    ///
    /// Filter by Item(if item was provided) N zones
    /// Max N of items only to have a treshold
    Any(u32, Option<ItemIdentifier>, Option<u32>),
    ByGatherable(ItemIdentifier),
}

/// Values are corelated to the R8 live build
#[derive(FromRepr, Debug, Default, Serialize, Deserialize, Clone)]
#[repr(u8)]
pub enum Rundown {
    #[default]
    Modded,
    R7 = 31,
    R1 = 32,
    R2 = 33,
    R3 = 34,
    R8 = 35,
    R4 = 37,
    R5 = 38,
    R6 = 41,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Level {
    pub rundown: Rundown,
    pub exp_name: String,
    pub gathatable_items: Vec<GatherItem>,
    pub zones: Vec<TimerEntry>,
    pub maps: Vec<GatherableMap>,
}

impl Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let expedition = if self.exp_name != "E3" {
            &self.exp_name
        } else {
            &"E2".to_string()
        };
        write!(f, "{:?}{}", self.rundown, expedition)
    }
}

impl Level {
    // TODO: impl fn on Level to load level from file
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GatherableMap {
    pub outline_poly: Vec<Vec2>,
    pub blockouts: Vec<[Vec2; 4]>,
}

/// Main enum which keeps list of all gatherable items in game and related data to them
/// Keys and Bulkhead Keys and HSU don't have item ID and/or have separate algorithm of
/// generating and are dependant on some internal datablocks(?).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GatherItem {
    /// Zone, Name
    Key(Zone, String),
    /// Zone, Name
    BulkheadKey(Zone, String),
    /// Zone, Local Area ID, Local Area Name
    HSU(Zone, u32, char),
    /// Zone, Name, item idx, idx
    Generator(Zone, String, u8, u8),
    /// Zone, Item Seed
    ID(Zone, u32),
    /// Zone, Item Seed
    PD(Zone, u32),
    /// Zone, Spawn Zone idx
    Cell(Zone, u8),
    /// Zone, Name
    FogTurbine(Zone, String),
    /// Zone, Name - R2E1 only level for this
    Neonate(Zone, String),
    /// Zone, Name
    Cryo(Zone, String),
    /// Zone, Item Seed
    GLP1(Zone, u32),
    /// Zone, Item Seed
    OSIP(Zone, u32),
    /// Zone, Spawn Zone idx
    Datasphere(Zone, u8),
    /// Zone, Item Seed
    PlantSample(Zone, u32),
    /// Zone, Name
    HiSec(Zone, String),
    /// Zone, Item Seed
    DataCube(Zone, u32),
    /// Zone, Item Seed
    GLP2(Zone, u32),
    /// Zone, Name
    Cargo(Zone, String),
}

#[derive(FromRepr, Debug, Serialize, Deserialize, Clone)]
#[repr(u8)]
pub enum ItemIdentifier {
    ID = 128,
    PD = 129,
    Cell = 131,
    FogTurbine = 133,
    Neonate = 137,
    Cryo = 148,
    GLP1 = 149,
    OSIP = 150,
    Datasphere = 151,
    PlantSample = 153,
    HiSec = 154,
    DataCubeR8 = 165,
    DataCube = 168,
    GLP2 = 169,
    Cargo = 176,
}

pub mod re {
    use regex::Regex;
    use std::sync::LazyLock;

    /// At the start of level gen - get the seed info
    pub static BUILDER_LEVEL_SEEDS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?m)^.*Builder\.Build.*buildSeed:\s(?<build>\d+)\shostIDSeed:\s(?<hostId>\d+)\ssessionSeed:\s(?<session>\d+).*$").unwrap()
    });

    /// At the start of level gen - get the level info
    pub static DROP_SERVER_MANAGER_NEW_SESSION: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?m)^.*ServerManager:\s'new\ssession.*?rundown:\sLocal_(?<rundown_idx>\d+),\sexpedition:\s(?<rundown_exp>\w\d).*$").unwrap()
    });

    /// SetupFloor batch start
    pub static SETUP_FLOOR_BATCH_START: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?m)^Next\sBatch:\sSetupFloor.*$").unwrap());

    /// SetupFloor batch end
    pub static SETUP_FLOOR_BATCH_END: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?m)^.*Last\sBatch:\sSetupFloor.*$").unwrap());

    /// Zone info inside SetupFloor batch
    pub static ZONE_CREATED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?m)^.*?Alias: (?<alias>\d+).*aliasOffset: \w+_(?<local>\d+).*\s.*?Zone\sCreated.*?in\s(?<dim>\w+)\s(?<layer>\w+).*$"
        )
        .unwrap()
    });

    pub static BUILD_END: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^.*BUILDER\s:\sBuildDone$").unwrap());

    pub static SPLIT_TIME: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?<h>\d{2}):(?<m>\d{2}):(?<s>\d{2})\.(?<millis>\d{3}).*$").unwrap()
    });

    pub static CREATE_KEY_ITEM_DISTRIBUTION: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(concat!(
            r"^.*?PublicName:\s(?<key>[A-Za-z0-9_]+).*?DimensionIndex:\s(?<dim>\w+)\sLocalIndex:\s\w+_(?<local>\d+).*?", // CreateKeyItemDistribution
            r"(?:\s|.*)*?",                                             // Discard
            r"TryGetExisting.*?ZONE(?<alias>\d+).*?ri:\s(?<ri>\d+).*$", // TryGetExistingGenericFunctionDistributionForSession
        ))
        .unwrap()
    });

    pub static DISTRIBUTE_HSU: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^.*zone:\s(?<alias>\d+),\sArea:\s(?<id>\d+)_\w+\s(?<area>\w+).*$").unwrap()
    });

    pub static DISTRIBUTE_WARDEN_OBJECTIVE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^.*?zone\sZONE(?<alias>\d+).*?Index:\s(?<idx>\d+).*\n.*?itemID:\s(?<item>\d+).*$",
        )
        .unwrap()
    });

    pub static WARDEN_OBJECTIVE_MANAGER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^(?<gen>.*LG_PowerGenerator_Graphics.OnSyncStatusChanged.*)?\s?(?:.*?Collection\s(?<id>\d+)\s.*?\s(?<name>\w+_\d+))$"
        )
        .unwrap()
    });
}

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
    GeneratedZone(Zone),
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
                                        let (_, [build_seed, host_seed, session_seed]) =
                                            cap.extract();
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

                                        let rundown_idx = rundown_idx.parse::<u8>()?;
                                        let rundown_exp = rundown_exp.to_string();

                                        let level = Level {
                                            rundown: Rundown::from_repr(rundown_idx)
                                                .unwrap_or(Rundown::Modded),
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
                                        for cap in re::ZONE_CREATED
                                            .captures_iter(&parser_manager.buffer[start..end])
                                        {
                                            let (_, [alias, local, dim, layer]) = cap.extract();
                                            let zone = Zone {
                                                alias: alias.parse::<u32>()?,
                                                local: local.parse::<u32>()?,
                                                dimension: dim.to_string(),
                                                layer: layer.to_string(),
                                                area: None,
                                            };
                                            parser_tx.send(ParserMsg::GeneratedZone(zone))?;
                                        }
                                        parser_manager.state = ParserState::ItemGeneration;
                                    }
                                }
                                ParserState::ItemGeneration => {}
                                ParserState::ElevatorDropFinish => {
                                    // TODO: Lel
                                }
                                ParserState::LevelFinish => {}
                                ParserState::NotInLevel => {}
                            }
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

// TODO: remove this/rewrite
// fn split_time(s: String) -> Option<NaiveTime> {
//     //! 11:11:11.111 - other text -> (11:11:11.111, "other text")
//     if let Some(caps) = re::SPLIT_TIME.captures(s.as_str()) {
//         Some(
//             NaiveTime::from_hms_milli_opt(
//                 caps.name("h")
//                     .map_or(0u32, |v| u32::from_str_radix(v.as_str(), 10).unwrap()),
//                 caps.name("m")
//                     .map_or(0u32, |v| u32::from_str_radix(v.as_str(), 10).unwrap()),
//                 caps.name("s")
//                     .map_or(0u32, |v| u32::from_str_radix(v.as_str(), 10).unwrap()),
//                 caps.name("millis")
//                     .map_or(0u32, |v| u32::from_str_radix(v.as_str(), 10).unwrap()),
//             )
//             .unwrap(),
//         )
//     } else {
//         None
//     }
// }

// fn create_zone(s: String) -> Option<(u32, u32)> {
//     //! <color=#C84800>>>>>>>>>------------->>>>>>>>>>>> LG_Floor.CreateZone, Alias: 410 with BuildFromZoneAlias410 zoneAliasStart: 410 aliasOffset: Zone_0</color>
//     if let Some(caps) = re::CREATE_ZONE.captures(s.as_str()) {
//         Some((
//             caps.name("alias")
//                 .map_or(0u32, |v| u32::from_str_radix(v.as_str(), 10).unwrap()),
//             caps.name("local")
//                 .map_or(0u32, |v| u32::from_str_radix(v.as_str(), 10).unwrap()),
//         ))
//     } else {
//         None
//     }
// }

// fn zone_created(s: String) -> Option<(String, String)> {
//     //! <b>Zone Created</b> (New Game Object) in Reality MainLayer with
//     if let Some(caps) = re::ZONE_CREATED.captures(s.as_str()) {
//         Some((
//             caps.name("dim").unwrap().as_str().to_owned(),
//             caps.name("layer").unwrap().as_str().to_owned(),
//         ))
//     } else {
//         None
//     }
// }

// // Start of key generation. We can get name of the key and zone(local_name)
// fn create_key_item_distribution(s: String) -> Option<(String, String, u32)> {
//     //! <color=purple>CreateKeyItemDistribution, keyItem: PublicName: KEY_WHITE_584 SpawnedItem: KeyItemPickup_Core(Clone)_GateKeyItem:KEY_WHITE_584_terminalKey: KEY_WHITE_584 (KeyItemPickup_Core) placementData: DimensionIndex: Reality LocalIndex: Zone_1 ZonePlacementWeights, Start: 0 Middle: 2500 End: 10000</color>
//     if let Some(caps) = re::CREATE_KEY_ITEM_DISTRIBUTION.captures(s.as_str()) {
//         Some((
//             caps.name("key").unwrap().as_str().to_owned(),
//             caps.name("dim").unwrap().as_str().to_owned(),
//             caps.name("local")
//                 .map_or(0u32, |v| u32::from_str_radix(v.as_str(), 10).unwrap()),
//         ))
//     } else {
//         None
//     }
// }

// // End of key generation. `ri` can be used to give key id
// // which will be used to map key to it's spawn zone and place in game.
// fn create_key_get_distribution_function(s: String) -> Option<(u32, u32)> {
//     //! <color=#C84800>TryGetExistingGenericFunctionDistributionForSession, foundDist in zone: ZONE50 function: ResourceContainerWeak available: 58 randomValue: 0.8431178 ri: 54 had weight: 10001</color>
//     if let Some(caps) = re::CREATE_KEY_GET_DISTRIBUTION.captures(s.as_str()) {
//         Some((
//             caps.name("alias")
//                 .map_or(0u32, |v| u32::from_str_radix(v.as_str(), 10).unwrap()),
//             caps.name("ri")
//                 .map_or(0u32, |v| u32::from_str_radix(v.as_str(), 10).unwrap()),
//         ))
//     } else {
//         None
//     }
// }

// // Funky Area/Subzone which is consistent with in-game generation. Can be used to identify area(as in A, B, C, E, etc. not 1-1 mapping)
// pub fn distribute_warden_objective_hydro_stasis_unit(s: String) -> Option<(u32, u32, char)> {
//     //! <color=#C84800>>>>> LG_Distribute_WardenObjective, placing warden objective item with function HydroStatisUnit for wardenObjectiveType: HSU_FindTakeSample in zone: 52, Area: 15_Area B (LevelGeneration.LG_Area)</color>
//     if let Some(caps) = re::DISTRIBUTE_HSU.captures(s.as_str()) {
//         Some((
//             caps.name("alias")
//                 .map_or(0u32, |v| u32::from_str_radix(v.as_str(), 10).unwrap()),
//             caps.name("id")
//                 .map_or(0u32, |v| u32::from_str_radix(v.as_str(), 10).unwrap()),
//             caps.name("area")
//                 .map_or('-', |v| v.as_str().chars().next().unwrap()),
//         ))
//     } else {
//         None
//     }
// }

// // Mainly to keep track of of number of objective items that is queued up.
// // No way to know if the order is kept through.
// fn distribute_warden_objective() -> () {
//     //! <color=#C84800>LG_Distribute_WardenObjective.SelectZoneFromPlacementAndKeepTrackOnCount, creating dist in zone ZONE14 spawnZones[placementDataIndex].Count: 8 spawnZoneIndex: 6 spawnedInZoneCount: 1</color>
//     //! <color=#C84800>LG_Distribute_WardenObjective.DistributeGatherRetrieveItems, creating dist to spawn itemID: 149 for chainIndex: 0</color>
//     todo!()
// }

// fn warden_objective_mapper_power_generator<'a>() -> () {
//     //! LG_PowerGenerator_Graphics.OnSyncStatusChanged UnPowered
//     //! WardenObjectiveManager.RegisterObjectiveItemForCollection 0 item: GENERATOR_190
//     todo!()
// }

// // Generalized for any item
// fn warden_objective_mapper_register_item(s: String) -> Option<(u32, String)> {
//     //! WardenObjectiveManager.RegisterObjectiveItemForCollection 0 item: GENERATOR_190
//     if let Some(caps) = re::WARDEN_OBJECTIVE_MANAGER.captures(s.as_str()) {
//         Some((
//             caps.name("id")
//                 .map_or(0u32, |v| u32::from_str_radix(v.as_str(), 10).unwrap()),
//             caps.name("name").unwrap().as_str().to_owned(),
//         ))
//     } else {
//         None
//     }
// }
