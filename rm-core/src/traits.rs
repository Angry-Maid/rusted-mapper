use chrono::NaiveTime;
use serde::{Deserialize, Serialize};

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
    /// Zone, Spawn Zone idx, Item Seed
    ID(Zone, u8, u32),
    /// Zone, Spawn Zone idx, Item Seed
    PD(Zone, u8, u32),
    /// Zone, Spawn Zone idx
    Cell(Zone, u8),
    /// Zone, Name
    FogTurbine(Zone, String),
    /// Zone, Name - R2E1 only level for this
    Neonate(Zone, String),
    /// Zone, Name
    Cryo(Zone, String),
    /// Zone, Spawn Zone idx, Item Seed
    GLP1(Zone, u8, u32),
    /// Zone, Spawn Zone idx, Item Seed
    OSIP(Zone, u8, u32),
    /// Zone, Spawn Zone idx
    Datasphere(Zone, u8),
    /// Zone, Spawn Zone idx, Item Seed
    PlantSample(Zone, u8, u32),
    /// Zone, Name
    HiSec(Zone, String),
    /// Zone, Spawn Zone idx, Item Seed
    DataCube(Zone, u8, u32),
    /// Zone, Spawn Zone idx, Item Seed
    GLP2(Zone, u8, u32),
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
