use std::{collections::HashMap, fmt::Display, ops::Index};

use serde::{Deserialize, Serialize};

use super::{GatherItem, GatherableMap, Rundown, TimerEntry, Zone};

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Level {
    /// General info about level
    pub rundown: Rundown,
    pub exp_name: String,
    pub timer_zones: Vec<TimerEntry>,

    /// Learning mode
    pub zones: Vec<Zone>,
    pub gathatable_items: HashMap<Zone, GatherItem>,
    pub maps: Vec<GatherableMap>,
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
