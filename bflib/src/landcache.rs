use anyhow::Result;
use dcso3::{land::Land, LuaVec3, MizLua, Vector3};
use fxhash::FxBuildHasher;
use indexmap::{map::Entry, IndexMap};
use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Hv3 {
    x: f64,
    y: f64,
    z: f64,
}

impl Eq for Hv3 {}

impl Hash for Hv3 {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.x.to_bits().hash(state);
        self.y.to_bits().hash(state);
        self.z.to_bits().hash(state);
    }
}

impl From<Vector3> for Hv3 {
    fn from(value: Vector3) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}

impl Into<LuaVec3> for Hv3 {
    fn into(self) -> LuaVec3 {
        LuaVec3(Vector3::new(self.x, self.y, self.z))
    }
}

#[derive(Debug, Clone)]
pub struct LandCache {
    h: IndexMap<(Hv3, Hv3), (bool, u32), FxBuildHasher>,
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

    pub fn is_visible(&mut self, land: &Land, p0: Vector3, p1: Vector3) -> Result<bool> {
        let ans = match self.h.entry((p0.into(), p1.into())) {
            Entry::Occupied(mut e) => {
                let (res, cnt) = e.get_mut();
                *cnt = u32::saturating_add(*cnt, 1);
                Ok(*res)
            }
            Entry::Vacant(e) => {
                let ans = land.is_visible(LuaVec3(p0), LuaVec3(p1))?;
                e.insert((ans, 1));
                self.added += 1;
                Ok(ans)
            }
        };
        if self.added > self.max_size {
            self.added = 0;
            self.h.sort_by(|_, (_, c0), _, (_, c1)| c1.cmp(c0));
            while self.h.len() > self.max_size {
                self.h.pop();
            }
        }
        ans
    }
}
