

use std::collections::HashMap;

use dcso3::{ coalition::Side, Vector3};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlayerInfo {
    pub ucid : String,
    pub funds : isize,
    pub name : Option<String>,
    pub airframes : HashMap<String, Airframe>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Airframe {
    name : String,
    type_name : String,
    cost : isize,
    fuel : isize,
    current_payload : HashMap<isize, String>,
    location : Vector3
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pmc {
    pub name : String,
    pub side : Side,
    pub funds : isize,
    players : HashMap<String, PlayerInfo>,
}
