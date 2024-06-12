use std::path::{Path, PathBuf};

use log::{debug, info, warn};
use notify::{
    event::{CreateKind, DataChange, ModifyKind, RenameMode},
    recommended_watcher, Error, Event, RecommendedWatcher, RecursiveMode, Watcher,
};

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

#[derive(Default, Debug, Eq, PartialEq, Clone, Copy)]
pub enum ParserState {
    #[default]
    InitialState,
    BuilderStart,
    BuilderEnd,
}

pub struct Parser {
    pub local_path: PathBuf,
    pub dir_watcher: RecommendedWatcher,
    buffer: Vec<String>,
    end_pos: usize,
    state: ParserState,
}

impl Parser {
    pub fn new() -> Self {
        let profile_path =
            Path::new(env!("USERPROFILE")).join("appdata\\locallow\\10 Chambers Collective\\GTFO");
        let path = profile_path.as_path();

        let mut watcher = recommended_watcher(|res: Result<Event, Error>| match res {
            Ok(event) => {
                info!("{:?} {:?} {:?}", event.kind, event.attrs, event.paths);
                match event.kind {
                    notify::EventKind::Any => {}
                    notify::EventKind::Create(CreateKind::File) => {
                        // On new File creation
                    }
                    notify::EventKind::Modify(ModifyKind::Data(DataChange::Content)) => {
                        // On new data in file <- doesn't work cause lib uses `ReadDirectoryChangesW`
                    }
                    notify::EventKind::Modify(ModifyKind::Name(RenameMode::Any)) => {
                        // On file rename
                    }
                    _ => {}
                }
            }
            Err(e) => info!("{:?}", e),
        })
        .unwrap();

        watcher.watch(path, RecursiveMode::NonRecursive).unwrap();

        Parser {
            local_path: profile_path,
            dir_watcher: watcher,
            buffer: Default::default(),
            end_pos: 0,
            state: Default::default(),
        }
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
