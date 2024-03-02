use anyhow::Result;
use core::fmt;
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

#[derive(Debug, Clone, Copy)]
pub struct Stats {
    pub calls: usize,
    pub hits: usize,
    pub diffs: usize,
}

impl std::fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hitrate = (self.hits as f32 / self.calls as f32) * 100.;
        let diffrate = (self.diffs as f32 / self.hits as f32) * 100.;
        write!(
            f,
            "calls: {}, hits: {}({:.02}%), diffs: {}({:.02}%)",
            self.calls, self.hits, hitrate, self.diffs, diffrate
        )
    }
}

#[derive(Debug, Clone)]
pub struct LandCache {
    h: IndexMap<(Tile, Tile), CacheEntry, FxBuildHasher>,
    max_size: usize,
    added: usize,
    debug: bool,
    stats: Stats,
}

impl Default for LandCache {
    fn default() -> Self {
        Self::new(10 * 1024 * 1024)
    }
}

impl LandCache {
    pub fn new(max_size: usize) -> LandCache {
        Self {
            h: IndexMap::with_capacity_and_hasher(max_size, FxBuildHasher::default()),
            added: 0,
            max_size,
            debug: true,
            stats: Stats {
                calls: 0,
                hits: 0,
                diffs: 0,
            },
        }
    }

    pub fn stats(&self) -> Stats {
        self.stats
    }

    pub fn is_visible(&mut self, land: &Land, d: f64, p0: Vector3, p1: Vector3) -> Result<bool> {
        self.stats.calls += 1;
        let t0 = Tile::new(d, p0);
        let t1 = Tile::new(d, p1);
        let ans = match self.h.entry((t0, t1)) {
            Entry::Occupied(mut e) => {
                self.stats.hits += 1;
                let ent = e.get_mut();
                ent.hits = u32::saturating_add(ent.hits, 1);
                if self.debug {
                    let visible = land.is_visible(LuaVec3(p0), LuaVec3(p1))?;
                    if ent.visible != visible {
                        self.stats.diffs += 1;
                    }
                }
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
