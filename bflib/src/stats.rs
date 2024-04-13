use crate::cfg::Vehicle;
use dcso3::{coalition::Side, net::Ucid, String};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StatKind {
    NewRound,
    RoundEnd { winner: Side },
    PlayerRegister { name: String, ucid: Ucid, side: Side },
    PlayerSideswitch { ucid: Ucid, side: Side },
    Takeoff { ucid: Ucid, aircraft: Vehicle },
    Land { ucid: Ucid, life_returned: bool },
    
}
