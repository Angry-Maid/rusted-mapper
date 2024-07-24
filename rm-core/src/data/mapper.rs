use glam::Vec2;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GatherableMap {
    pub outline_poly: Vec<Vec2>,
    pub blockouts: Vec<[Vec2; 4]>,
}
