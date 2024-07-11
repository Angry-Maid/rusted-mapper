use std::{
    default,
    path::{Path, PathBuf},
    sync::mpsc::{channel, Receiver, Sender, TryRecvError},
    thread,
    time::Duration,
};

use chrono::NaiveTime;
use glam::Vec2;
use log::{error, info};
use might_sleep::cpu_limiter::CpuLimiter;
use notify::{
    event::{CreateKind, DataChange, ModifyKind, RenameMode},
    recommended_watcher, Error, Event, RecommendedWatcher, RecursiveMode, Watcher,
};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::tail::{Tail, TailCmd, TailMsg};

#[derive(Debug, Serialize, Deserialize)]
pub struct Zone {
    pub alias: u32,
    pub local: u32,
    pub dimension: String,
    pub layer: String,
    pub subzone: Option<char>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Record {
    pub time: NaiveTime,
    pub item: Option<GatherItem>,
    pub zone: Option<Zone>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TimerEntry {
    Start,
    Zone(Zone),
    Invariance(Vec<Zone>, InvarianceMethod),
    End,
}

#[derive(Default, Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Level {
    pub name: String,
    pub gathatable_items: Vec<GatherItem>,
    pub zones: Vec<TimerEntry>,
    pub maps: Vec<GatherableMap>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GatherableMap {
    pub outline_poly: Vec<Vec2>,
    pub blockouts: Vec<[Vec2; 4]>,
}

/// Main enum which keeps list of all gatherable items in game and related data to them
/// Keys and Bulkhead Keys and HSU don't have item ID and/or have separate algorithm of
/// generating and are dependant on some internal datablocks(?).
#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
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
    DataCube = 168,
    GLP2 = 169,
    Cargo = 176,
}

pub mod re {
    use regex::Regex;
    use std::sync::LazyLock;

    pub static BUILD_START: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^.*Builder\.Build.*buildSeed:\s(?P<build>\d+)\shostIDSeed:\s(?P<hostId>\d+)\ssessionSeed:\s(?P<session>\d+)$").unwrap()
    });

    pub static BUILD_END: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^.*BUILDER\s:\sBuildDone$").unwrap());

    pub static SPLIT_TIME: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?P<h>\d{2}):(?P<m>\d{2}):(?P<s>\d{2})\.(?P<millis>\d{3}).*$").unwrap()
    });
    pub static ZONE_CREATED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^.*?Alias: (?P<alias>\d+).*aliasOffset: \w+_(?P<local>\d+).*\s.*?Zone\sCreated.*?in\s(?P<dim>\w+)\s(?P<layer>\w+).*$"
        )
        .unwrap()
    });
    pub static CREATE_KEY_ITEM_DISTRIBUTION: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(concat!(
            r"^.*?PublicName:\s(?P<key>[A-Za-z0-9_]+).*?DimensionIndex:\s(?P<dim>\w+)\sLocalIndex:\s\w+_(?P<local>\d+).*?", // CreateKeyItemDistribution
            r"(?:\s|.*)*?",                                               // Discard
            r"TryGetExisting.*?ZONE(?P<alias>\d+).*?ri:\s(?P<ri>\d+).*$", // TryGetExistingGenericFunctionDistributionForSession
        ))
        .unwrap()
    });
    pub static DISTRIBUTE_HSU: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^.*zone:\s(?P<alias>\d+),\sArea:\s(?P<id>\d+)_\w+\s(?P<area>\w+).*$").unwrap()
    });
    pub static DISTRIBUTE_WARDEN_OBJECTIVE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^.*?zone\sZONE(?P<alias>\d+).*?Index:\s(?P<idx>\d+).*\n.*?itemID:\s(?P<item>\d+).*$",
        )
        .unwrap()
    });
    pub static WARDEN_OBJECTIVE_MANAGER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^(?P<gen>.*LG_PowerGenerator_Graphics.OnSyncStatusChanged.*)?\s?(?:.*?Collection\s(?P<id>\d+)\s.*?\s(?P<name>\w+_\d+))$"
        )
        .unwrap()
    });
}

#[derive(Debug)]
pub struct Parser {
    watch_path: PathBuf,
    dir_watcher: Option<RecommendedWatcher>,
    tail_cmd_tx: Option<Sender<TailCmd>>,
    tail: Option<Tail>,
    pub rx: Option<Receiver<ParserMsg>>,
}

#[derive(Debug)]
pub enum ParserMsg {
    LevelInit(Level),
    Gatherable(GatherItem),
    LevelStart,
    ZoneDoorOpened,
    LevelFinish,
    NewFile,
}

#[derive(Debug, Default)]
enum ParserState {
    #[default]
    Initial,
    LevelGenerationStart,
    LevelGenerationFinish,
    ElevatorDropFinish,
    LevelFinish,
}

#[derive(Debug, Default)]
struct ParserManager {
    buffer: String,
    pos: usize,
    state: ParserState,
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
            tail: None,
            rx: None,
        }
    }

    pub fn start_watcher(&mut self) -> anyhow::Result<()> {
        self.tail.replace(Tail::new());

        let (command_tx, data_rx): (Sender<TailCmd>, Receiver<TailMsg>) =
            self.tail.unwrap().start_listen()?;

        let (parser_tx, parser_rx) = channel::<ParserMsg>();

        self.tail_cmd_tx.replace(command_tx.clone());
        self.rx = Some(parser_rx);

        thread::Builder::new()
            .name("parser".into())
            .spawn(|| Parser::parser(data_rx, parser_tx))?;

        // We first look for `NICKNAME_NETSTATUS` file in case
        // rusted-mapper was opened after the game was open.
        for entry in WalkDir::new(self.watch_path.clone().as_path().to_owned())
            .min_depth(1)
            .max_depth(1)
            .sort_by(|a, b| {
                b.metadata()
                    .unwrap()
                    .modified()
                    .unwrap()
                    .cmp(&a.metadata().unwrap().modified().unwrap())
            })
        {
            if let Ok(dir_entry) = entry {
                info!("{:?}", dir_entry.file_name());
                if match dir_entry.file_name().to_str() {
                    Some(val) => val.contains("NICKNAME_NETSTATUS"),
                    None => false,
                } {
                    command_tx.send(TailCmd::Open(dir_entry.path().to_path_buf()))?;
                    break;
                }
            }
        }

        let mut watcher = recommended_watcher(move |res: Result<Event, Error>| match res {
            Ok(event) => {
                info!("{:?} {:?} {:?}", event.kind, event.attrs, event.paths);
                match event.kind {
                    notify::EventKind::Create(CreateKind::Any) => {
                        if let Some(path) = event.paths.first() {
                            match path.file_name() {
                                Some(filename) => {
                                    if match filename.to_str() {
                                        Some(val) => val.contains("NICKNAME_NETSTATUS"),
                                        None => false,
                                    } {
                                        command_tx.send(TailCmd::Open(path.to_path_buf())).unwrap();
                                    }
                                }
                                None => {}
                            }
                        }
                        info!(
                            "Filename: {:?}",
                            event.paths.first().unwrap().file_name().unwrap()
                        );
                    }
                    notify::EventKind::Modify(ModifyKind::Data(DataChange::Content)) => {
                        // On new data in file <- doesn't work cause lib uses `ReadDirectoryChangesW`
                    }
                    notify::EventKind::Modify(ModifyKind::Name(RenameMode::Any)) => {
                        // On file rename ???
                    }
                    _ => {}
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
        // Inner state manager

        let mut limiter = CpuLimiter::new(Duration::from_millis(250));
        let mut parser_manager = ParserManager::default();

        loop {
            match data_rx.try_recv() {
                Ok(val) => {
                    // For now we get the message and propagate it back
                    match val {
                        TailMsg::Content(s) => {
                            parser_manager.buffer.extend(s.chars());
                            info!("{}", parser_manager.buffer);
                            // parser_tx.send(ParserMsg::Content(s))?;
                        }
                        TailMsg::NewFile => {
                            parser_manager.buffer.clear();
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

    fn parse() -> anyhow::Result<()> {
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
