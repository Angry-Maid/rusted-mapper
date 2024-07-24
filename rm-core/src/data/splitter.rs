use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use super::{GatherItem, ItemIdentifier, Zone};

#[derive(Debug, Serialize, Deserialize)]
pub struct Record {
    pub time: Timestamp,
    pub item: Option<GatherItem>,
    pub zone: Option<Zone>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TimerEntry {
    Start,
    Zone(Zone),
    Custom(String),
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
