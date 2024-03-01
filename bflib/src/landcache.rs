use anyhow::Result;
use dcso3::{land::Land, LuaVec3, Vector3};
use fxhash::FxBuildHasher;
use indexmap::{map::Entry, IndexMap};
use std::{cmp::max, hash::Hash};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Tile {
    x: i32,
    y: i32,
    z: i32,
    d: u32,
}

impl Tile {
    fn new(d: f64, v: Vector3) -> Self {
        // tile size is 1 / 16th of the distance between the two
        // points being checked rounded to the nearest power of 2
        let d = max(1, ((d.trunc() as i64) >> 4) as u32).next_power_of_two();
        let df = d as f64;
        let x = v.x.div_euclid(df) as i32;
        let y = v.y.div_euclid(df) as i32;
        let z = v.z.div_euclid(df) as i32;
        Self { x, y, z, d }
    }
}

#[derive(Debug, Clone, Copy)]
struct CacheEntry {
    visible: bool,
    hits: u32,
}

#[derive(Debug, Clone)]
pub struct LandCache {
    h: IndexMap<(Tile, Tile), CacheEntry, FxBuildHasher>,
    max_size: usize,
    added: usize,
}

impl Default for LandCache {
    fn default() -> Self {
        Self::new(1024 * 1024)
    }
}

impl LandCache {
    pub fn new(max_size: usize) -> LandCache {
        Self {
            h: IndexMap::with_capacity_and_hasher(max_size, FxBuildHasher::default()),
            added: 0,
            max_size,
        }
    }

    pub fn is_visible(&mut self, land: &Land, d: f64, p0: Vector3, p1: Vector3) -> Result<bool> {
        let t0 = Tile::new(d, p0);
        let t1 = Tile::new(d, p1);
        let ans = match self.h.entry((t0, t1)) {
            Entry::Occupied(mut e) => {
                let ent = e.get_mut();
                ent.hits = u32::saturating_add(ent.hits, 1);
                Ok(ent.visible)
            }
            Entry::Vacant(e) => {
                let visible = land.is_visible(LuaVec3(p0), LuaVec3(p1))?;
                e.insert(CacheEntry { visible, hits: 1 });
                self.added += 1;
                Ok(visible)
            }
        };
        if self.added > self.max_size {
            self.added = 0;
            self.h.sort_by(|_, e0, _, e1| e1.hits.cmp(&e0.hits));
            while self.h.len() > self.max_size {
                self.h.pop();
            }
        }
        ans
    }
}
