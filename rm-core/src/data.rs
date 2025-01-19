use std::{collections::HashMap, fmt::Display, ops::Index};

use serde::{Deserialize, Serialize};
use strum::FromRepr;

#[derive(Debug, Clone)]
pub enum Token {
    Seeds(u32, u32, u32),
    Expedition(Rundown, String, usize),
    Zone(Zone),
    Start,
    Split,
    End,
    // Local Index, Item
    Gatherable(u32, String, GatherItem),
    Uncategorized(ItemIdentifier, u32),
    Reset,
}

/// Values are corelated to the R8 live build
#[derive(FromRepr, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Clone)]
#[repr(u16)]
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
    Tutorial = 39,
    R6 = 41,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct Zone {
    pub alias: u32,
    pub local: u32,
    pub dimension: String,
    pub layer: String,
    pub area: Option<char>,
}

impl Display for Zone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ZONE_{} {} {}", self.alias, self.layer, self.dimension)
    }
}

impl Ord for Zone {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.alias, &self.layer, &self.dimension).cmp(&(
            other.alias,
            &other.layer,
            &other.dimension,
        ))
    }
}

impl PartialOrd for Zone {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some((self.alias, &self.layer, &self.dimension).cmp(&(
            other.alias,
            &other.layer,
            &other.dimension,
        )))
    }
}

/// Main enum which keeps list of all gatherable items in game and related data to them
/// Keys and Bulkhead Keys and HSU don't have item ID and/or have separate algorithm of
/// generating and are dependant on some internal datablocks(?).
/// Some items do have names cause there's literaly no other information that can be gotten
/// for those items. Items that have seed only may have more data, but seed data and other data
/// are split between 2 different batch jobs and there's no guarantee that the order is preserved.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Clone)]
pub enum GatherItem {
    /// Name, Dimension, Zone, ri
    Key(String, String, u32, u32),
    /// Name
    BulkheadKey(String),
    /// Local Area ID, Local Area Name
    HSU(u32, char),
    /// Name, item idx, idx
    Generator(String, u8, u8),
    /// Container, Item Seed
    ID(String, u32),
    /// Container, Item Seed
    PD(String, u32),
    /// Spawn Zone idx
    Cell(u8),
    /// Name
    FogTurbine(String),
    /// Name - R2E1 only level for this
    Neonate(String),
    /// Name
    Cryo(String),
    /// Container, Item Seed
    GLP1(String, u32),
    /// Container, Item Seed
    OSIP(String, u32),
    /// Spawn Zone idx
    Datasphere(u8),
    /// Container, Item Seed
    PlantSample(String, u32),
    /// Name
    HiSec(String),
    /// Container, Item Seed
    DataCube(String, u32),
    /// Container, Item Seed
    GLP2(String, u32),
    /// Name
    Cargo(String),
    /// Locker, seed
    Seeded(String, u32),
}

#[derive(FromRepr, Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
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
    MWP = 164,
    DataCubeR8 = 165,
    DataCube = 168,
    GLP2 = 169,
    Cargo = 176,
    Unknown(u8),
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Level {
    /// General info about level
    pub rundown: Option<Rundown>,
    pub tier: Option<String>,
    pub exp: Option<usize>,

    /// Learning mode
    pub zones: Vec<Zone>,
    pub gathatable_items: HashMap<Zone, GatherItem>,
    pub uncategorized: Vec<ItemIdentifier>,
}

impl Index<(u32, String)> for Level {
    type Output = Zone;

    fn index(&self, index: (u32, String)) -> &Self::Output {
        self.zones
            .iter()
            .find(|v| v.alias == index.0 && v.dimension.eq(&index.1))
            .unwrap()
    }
}

impl Index<u32> for Level {
    type Output = Zone;

    fn index(&self, index: u32) -> &Self::Output {
        self.zones.iter().find(|v| v.alias == index).unwrap()
    }
}

impl Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let (Some(rundown), Some(tier), Some(exp_idx)) =
            (self.rundown.clone(), self.tier.clone(), self.exp.clone())
        {
            if (matches!(rundown, Rundown::Tutorial)) {
                write!(f, "{:?}", rundown)
            } else {
                write!(f, "{:?}{}{}", rundown, tier, exp_idx)
            }
        } else {
            write!(f, "{:?}", None::<Level>)
        }
    }
}

impl Level {
    // TODO: impl fn on Level to load level from file
}
