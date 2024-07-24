use serde::{Deserialize, Serialize};
use strum::FromRepr;

/// Main enum which keeps list of all gatherable items in game and related data to them
/// Keys and Bulkhead Keys and HSU don't have item ID and/or have separate algorithm of
/// generating and are dependant on some internal datablocks(?).
/// Some items do have names cause there's literaly no other information that can be gotten
/// for those items. Items that have seed only may have more data, but seed data and other data
/// are split between 2 different batch jobs and there's no guarantee that the order is preserved.
#[derive(Debug, Serialize, Deserialize, Clone)]
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
