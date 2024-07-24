use serde::{Deserialize, Serialize};
use strum::FromRepr;

/// Values are corelated to the R8 live build
#[derive(FromRepr, Debug, Default, Serialize, Deserialize, Clone)]
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
    R6 = 41,
}
