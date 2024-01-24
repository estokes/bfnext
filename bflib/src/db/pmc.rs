use std::collections::HashMap;

use anyhow::{anyhow, Result, bail};
use dcso3::{coalition::Side, net::Ucid, Vector3};
use serde_derive::{Deserialize, Serialize};

use super::{Db, Map};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub ucid: Ucid,
    pub funds: isize,
    pub name: Option<String>,
    pub airframes: HashMap<String, Airframe>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Airframe {
    name: String,
    type_name: String,
    cost: isize,
    fuel: isize,
    current_payload: HashMap<isize, String>,
    location: Vector3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pmc {
    pub name: dcso3::String,
    pub side: Side,
    pub funds: isize,
    pub players: Map<Ucid, PlayerInfo>,
    pub inventory: Map<String, (String, isize)>,
}

impl Pmc {
    fn add_player(&mut self, player: PlayerInfo) -> Result<()> {
        self.players.insert_cow(player.ucid.clone(), player);
        Ok(())
    }
    fn remove_player(&mut self, ucid: &Ucid) -> Result<PlayerInfo> {
        self.players
            .remove_cow(ucid)
            .ok_or(anyhow!("player not found in PMC"))
    }
}

impl Db {
    pub fn pmc(&self, name: &str) -> Option<&Pmc> {
        self.persisted.pmcs.get(name)
    }

    pub fn register_pmc(&mut self, pmc: Pmc) -> Result<()> {
        match self.persisted.pmcs.get_key(&pmc.name) {
            Some(p) => bail!("{} already exists in DB", p),
            None => {
                self.persisted.pmcs.insert_cow(pmc.name.clone(), pmc);
                self.ephemeral.dirty();
            }
        }
        Ok(())
    }
}
