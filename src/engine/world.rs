use super::entity::{Entity, Id, Info};
use super::rng::Rng;
use ratatui::style::Color;
use rayon::prelude::*;
use std::collections::{BTreeMap, HashMap};

pub type Globals = BTreeMap<&'static str, f32>;

const CELL_W: i32 = 12;
const CELL_H: i32 = 6;

/// Cheap multiplicative hasher; ids and kind names need no DoS resistance.
#[derive(Default, Clone, Copy)]
pub struct FastHasher(u64);

impl std::hash::Hasher for FastHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0.rotate_left(5) ^ b as u64).wrapping_mul(0x517C_C1B7_2722_0A95);
        }
    }
    fn write_u64(&mut self, n: u64) {
        self.0 = (self.0.rotate_left(5) ^ n).wrapping_mul(0x517C_C1B7_2722_0A95);
    }
}

type FastMap<K, V> = HashMap<K, V, std::hash::BuildHasherDefault<FastHasher>>;

/// Spatial index over entity bounds, rebuilt every tick for neighbor queries.
pub struct Snapshot {
    pub infos: Vec<Info>,
    /// Flat bucket storage: `cell_start[c]..cell_start[c + 1]` indexes into `cell_items`.
    cell_start: Vec<u32>,
    cell_items: Vec<u32>,
    gw: i32,
    gh: i32,
    by_id: FastMap<Id, u32>,
    counts: FastMap<&'static str, usize>,
    /// Entities spanning many grid cells (backdrops); scanned linearly instead of indexed.
    big: Vec<u32>,
}

impl Snapshot {
    pub fn empty() -> Snapshot {
        Snapshot {
            infos: vec![],
            cell_start: vec![],
            cell_items: vec![],
            gw: 0,
            gh: 0,
            by_id: FastMap::default(),
            counts: FastMap::default(),
            big: vec![],
        }
    }

    fn build(ents: &[Entity], w: i32, h: i32) -> Snapshot {
        let gw = (w / CELL_W + 1).max(1);
        let gh = (h / CELL_H + 1).max(1);
        let infos: Vec<Info> = ents.iter().filter(|e| e.alive && !e.scenery).map(Info::of).collect();
        let mut s = Snapshot {
            infos,
            cell_start: vec![],
            cell_items: vec![],
            gw,
            gh,
            by_id: FastMap::default(),
            counts: FastMap::default(),
            big: vec![],
        };
        s.by_id.reserve(s.infos.len());

        // Counting sort of entity indices into grid cells.
        let ncells = (gw * gh) as usize;
        let mut counts = vec![0u32; ncells + 1];
        let spans: Vec<(i32, i32, i32, i32)> = s.infos.iter().map(|i| s.cells(i.rect)).collect();
        for (idx, sp) in spans.iter().enumerate() {
            let info = &s.infos[idx];
            s.by_id.insert(info.id, idx as u32);
            *s.counts.entry(info.kind).or_insert(0) += 1;
            if (sp.2 - sp.0 + 1) * (sp.3 - sp.1 + 1) > 16 {
                s.big.push(idx as u32);
                continue;
            }
            for cy in sp.1..=sp.3 {
                for cx in sp.0..=sp.2 {
                    counts[(cy * gw + cx) as usize + 1] += 1;
                }
            }
        }
        for c in 0..ncells {
            counts[c + 1] += counts[c];
        }
        let mut fill = counts.clone();
        let mut items = vec![0u32; counts[ncells] as usize];
        for (idx, sp) in spans.iter().enumerate() {
            if (sp.2 - sp.0 + 1) * (sp.3 - sp.1 + 1) > 16 {
                continue;
            }
            for cy in sp.1..=sp.3 {
                for cx in sp.0..=sp.2 {
                    let c = (cy * gw + cx) as usize;
                    items[fill[c] as usize] = idx as u32;
                    fill[c] += 1;
                }
            }
        }
        s.cell_start = counts;
        s.cell_items = items;
        s
    }

    fn cells(&self, r: (i32, i32, i32, i32)) -> (i32, i32, i32, i32) {
        let cl = |v: i32, hi: i32| v.clamp(0, hi - 1);
        (
            cl(r.0.div_euclid(CELL_W), self.gw),
            cl(r.1.div_euclid(CELL_H), self.gh),
            cl((r.0 + r.2.max(1) - 1).div_euclid(CELL_W), self.gw),
            cl((r.1 + r.3.max(1) - 1).div_euclid(CELL_H), self.gh),
        )
    }

    /// Entities whose bounds overlap the rectangle.
    pub fn query(&self, x: i32, y: i32, w: i32, h: i32) -> Vec<&Info> {
        if self.cell_start.is_empty() {
            return vec![];
        }
        let (x0, y0, x1, y1) = self.cells((x, y, w, h));
        let mut out: Vec<&Info> = Vec::new();
        for &i in &self.big {
            let inf = &self.infos[i as usize];
            if overlaps(inf.rect, (x, y, w, h)) {
                out.push(inf);
            }
        }
        let multi = x1 > x0 || y1 > y0;
        for cy in y0..=y1 {
            for cx in x0..=x1 {
                let c = (cy * self.gw + cx) as usize;
                let range = self.cell_start[c] as usize..self.cell_start[c + 1] as usize;
                for &i in &self.cell_items[range] {
                    let inf = &self.infos[i as usize];
                    if overlaps(inf.rect, (x, y, w, h)) && (!multi || !out.iter().any(|o| o.id == inf.id)) {
                        out.push(inf);
                    }
                }
            }
        }
        out
    }

    pub fn get(&self, id: Id) -> Option<&Info> {
        self.by_id.get(&id).map(|&i| &self.infos[i as usize])
    }

    pub fn count(&self, kind: &str) -> usize {
        self.counts.get(kind).copied().unwrap_or(0)
    }

    pub fn of_kind<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = &'a Info> {
        self.infos.iter().filter(move |i| i.kind == kind)
    }
}

pub fn overlaps(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
    a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
}

/// Entities handled per parallel work unit. Fixed so results do not depend on thread count.
const CHUNK: usize = 192;
/// Chunk slot reserved for work done on the calling thread.
const MAIN_SLOT: u64 = 0xFFF;

/// Everything a behavior or scenario may touch during a tick.
/// Writes (spawns, kills, global changes) are buffered and merged after each phase.
pub struct Ctx<'a> {
    pub rng: &'a mut Rng,
    pub w: f32,
    pub h: f32,
    pub time: f32,
    /// 1.0 at noon, 0.0 at midnight; constant 1.0 for scenarios without a day cycle.
    pub daylight: f32,
    pub snap: &'a Snapshot,
    base: &'a Globals,
    adds: Globals,
    sets: Globals,
    id_base: u64,
    id_next: u64,
    spawned: Vec<Entity>,
    killed: Vec<Id>,
}

/// Buffered output of one work unit.
struct Out {
    spawned: Vec<Entity>,
    killed: Vec<Id>,
    adds: Globals,
    sets: Globals,
}

impl<'a> Ctx<'a> {
    fn new(
        rng: &'a mut Rng,
        w: f32,
        h: f32,
        time: f32,
        daylight: f32,
        snap: &'a Snapshot,
        base: &'a Globals,
        epoch: u64,
        slot: u64,
    ) -> Ctx<'a> {
        Ctx {
            rng,
            w,
            h,
            time,
            daylight,
            snap,
            base,
            adds: Globals::new(),
            sets: Globals::new(),
            id_base: (epoch << 32) | (slot << 20),
            id_next: 0,
            spawned: vec![],
            killed: vec![],
        }
    }

    fn finish(self) -> Out {
        Out { spawned: self.spawned, killed: self.killed, adds: self.adds, sets: self.sets }
    }

    pub fn spawn(&mut self, mut e: Entity) -> Id {
        self.id_next += 1;
        e.id = self.id_base | self.id_next;
        let id = e.id;
        self.spawned.push(e);
        id
    }

    pub fn kill(&mut self, id: Id) {
        self.killed.push(id);
    }

    /// Accumulates into a shared counter; merged after the phase.
    pub fn add(&mut self, key: &'static str, amount: f32) {
        *self.adds.entry(key).or_insert(0.0) += amount;
    }

    /// Overwrites a shared value; intended for scenario-level code.
    pub fn set(&mut self, key: &'static str, value: f32) {
        self.sets.insert(key, value);
        self.adds.remove(key);
    }

    /// Value as of the start of the tick plus this context's own additions.
    pub fn global(&self, key: &str) -> f32 {
        let own = self.sets.get(key).copied().unwrap_or_else(|| self.base.get(key).copied().unwrap_or(0.0));
        own + self.adds.get(key).copied().unwrap_or(0.0)
    }
}

/// A themed world: supplies the initial population and spawns new life over time.
pub trait Scenario {
    fn name(&self) -> &'static str;
    fn setup(&mut self, cx: &mut Ctx);
    fn tick(&mut self, _cx: &mut Ctx, _dt: f32) {}
    /// Seconds per full day; `None` disables the day/night cycle.
    fn day_length(&self) -> Option<f32> {
        None
    }
    fn background(&self) -> Color {
        Color::Rgb(0, 0, 0)
    }
    /// Extra status text for the HUD.
    fn hud(&self, _globals: &Globals, _daylight: f32) -> String {
        String::new()
    }
}

pub struct World {
    pub w: i32,
    pub h: i32,
    pub ents: Vec<Entity>,
    pub rng: Rng,
    pub time: f32,
    pub daylight: f32,
    pub globals: Globals,
    pub scenario: Option<Box<dyn Scenario>>,
    pub ticks: u64,
    seed: u64,
}

impl World {
    pub fn new(w: i32, h: i32, seed: u64, scenario: Box<dyn Scenario>) -> World {
        let mut world = World {
            w,
            h,
            ents: vec![],
            rng: Rng::new(seed),
            time: 0.0,
            daylight: 1.0,
            globals: Globals::new(),
            scenario: Some(scenario),
            ticks: 0,
            seed,
        };
        world.setup();
        world
    }

    fn setup(&mut self) {
        let snap = Snapshot::empty();
        let mut scen = self.scenario.take().unwrap();
        self.daylight = day_factor(scen.as_ref(), 0.0);
        let base = self.globals.clone();
        let mut cx = Ctx::new(
            &mut self.rng, self.w as f32, self.h as f32, 0.0, self.daylight, &snap, &base, 0, MAIN_SLOT,
        );
        scen.setup(&mut cx);
        let out = cx.finish();
        self.merge(vec![out]);
        self.scenario = Some(scen);
    }

    /// Folds buffered phase outputs into the world in a fixed order.
    fn merge(&mut self, outs: Vec<Out>) {
        for mut o in outs {
            for (k, v) in o.sets {
                self.globals.insert(k, v);
            }
            for (k, v) in o.adds {
                *self.globals.entry(k).or_insert(0.0) += v;
            }
            for id in o.killed {
                if let Some(e) = self.ents.iter_mut().find(|e| e.id == id) {
                    e.alive = false;
                }
            }
            self.ents.append(&mut o.spawned);
        }
    }

    pub fn scenario_name(&self) -> &'static str {
        self.scenario.as_ref().map_or("", |s| s.name())
    }

    pub fn tick(&mut self, dt: f32) {
        self.time += dt;
        self.ticks += 1;
        let epoch = self.ticks * 4;
        let mut scen = self.scenario.take().unwrap();
        self.daylight = day_factor(scen.as_ref(), self.time);
        let (w, h, time, daylight) = (self.w, self.h, self.time, self.daylight);
        let base = self.globals.clone();

        // Scenario logic runs first, on this thread, against the start-of-tick snapshot.
        let snap = Snapshot::build(&self.ents, w, h);
        let mut scen_rng = Rng::new(self.seed ^ self.ticks.wrapping_mul(0x9E37_79B9));
        let mut cx = Ctx::new(&mut scen_rng, w as f32, h as f32, time, daylight, &snap, &base, epoch, MAIN_SLOT);
        scen.tick(&mut cx, dt);
        let scen_out = cx.finish();

        // Behavior phase: entities update in parallel, each chunk with its own rng and buffers.
        let seed = self.seed;
        let tick = self.ticks;
        let mut outs: Vec<Out> = vec![scen_out];
        let chunk_outs: Vec<Out> = self
            .ents
            .par_chunks_mut(CHUNK)
            .enumerate()
            .map(|(ci, chunk)| {
                let mut rng = Rng::new(seed ^ tick.wrapping_mul(0xA24B_AED4) ^ (ci as u64 + 1).wrapping_mul(0x9FB2_1C65));
                let mut cx = Ctx::new(&mut rng, w as f32, h as f32, time, daylight, &snap, &base, epoch + 1, ci as u64);
                for e in chunk.iter_mut().filter(|e| e.alive) {
                    step_entity(e, &mut cx, dt, w, h);
                }
                cx.finish()
            })
            .collect();
        outs.extend(chunk_outs);
        drop(snap);
        self.merge(outs);

        // Collision phase uses positions after movement.
        if self.ents.iter().any(|e| e.physical && e.alive) {
            let snap2 = Snapshot::build(&self.ents, w, h);
            let outs: Vec<Out> = self
                .ents
                .par_chunks_mut(CHUNK)
                .enumerate()
                .map(|(ci, chunk)| {
                    let mut rng = Rng::new(seed ^ tick.wrapping_mul(0xC2B2_AE35) ^ (ci as u64 + 7).wrapping_mul(0x85EB_CA6B));
                    let mut cx = Ctx::new(&mut rng, w as f32, h as f32, time, daylight, &snap2, &base, epoch + 2, ci as u64);
                    for e in chunk.iter_mut().filter(|e| e.physical && e.alive) {
                        let r = e.rect();
                        let others: Vec<Info> = snap2
                            .query(r.0, r.1, r.2, r.3)
                            .into_iter()
                            .filter(|o| o.id != e.id && o.physical)
                            .copied()
                            .collect();
                        if others.is_empty() {
                            continue;
                        }
                        if let Some(mut b) = e.behavior.take() {
                            for o in &others {
                                if e.alive {
                                    b.on_collide(e, o, &mut cx);
                                }
                            }
                            e.behavior = Some(b);
                        }
                    }
                    cx.finish()
                })
                .collect();
            self.merge(outs);
        }

        // Followers snap to their parent; orphans die with it.
        let mut positions: HashMap<Id, (f32, f32)> =
            self.ents.iter().filter(|e| e.alive).map(|e| (e.id, e.pos)).collect();
        for e in self.ents.iter_mut().filter(|e| e.alive) {
            if let Some(f) = e.follow {
                match positions.get(&f.parent) {
                    Some(p) => {
                        e.pos = (p.0 + f.dx, p.1 + f.dy);
                        e.vel = (0.0, 0.0);
                        positions.insert(e.id, e.pos);
                    }
                    None => e.alive = false,
                }
            }
        }

        // Death hooks may spawn replacements.
        let mut dead = vec![];
        let mut i = 0;
        while i < self.ents.len() {
            if self.ents[i].alive {
                i += 1;
            } else {
                dead.push(self.ents.swap_remove(i));
            }
        }
        if !dead.is_empty() {
            let snap3 = Snapshot::empty();
            let mut rng = Rng::new(seed ^ tick.wrapping_mul(0x27D4_EB2F));
            let mut cx = Ctx::new(&mut rng, w as f32, h as f32, time, daylight, &snap3, &base, epoch + 3, MAIN_SLOT);
            for e in dead.iter_mut() {
                if let Some(mut b) = e.behavior.take() {
                    b.on_death(e, &mut cx);
                }
            }
            let out = cx.finish();
            self.merge(vec![out]);
        }
        self.scenario = Some(scen);
    }
}

/// Advances one entity: behavior, movement, animation and expiry.
fn step_entity(e: &mut Entity, cx: &mut Ctx, dt: f32, w: i32, h: i32) {
    if let Some(mut b) = e.behavior.take() {
        b.update(e, cx, dt);
        e.behavior = Some(b);
    }
    e.pos.0 += e.vel.0 * dt;
    e.pos.1 += e.vel.1 * dt;
    e.age += dt;

    let nframes = e.sprite.frames.len();
    if nframes > 1 && e.anim_fps > 0.0 {
        e.anim_acc += dt * e.anim_fps;
        while e.anim_acc >= 1.0 {
            e.anim_acc -= 1.0;
            e.frame = (e.frame + 1) % nframes;
        }
    }
    e.frame %= nframes.max(1);

    if e.lifetime.is_some_and(|l| e.age >= l) {
        e.alive = false;
    }
    if e.die_offscreen && leaving(e, w, h) {
        e.alive = false;
    }
}

fn leaving(e: &Entity, w: i32, h: i32) -> bool {
    let (x, y) = (e.pos.0, e.pos.1);
    (x >= w as f32 && e.vel.0 > 0.0)
        || (x + e.w() <= 0.0 && e.vel.0 < 0.0)
        || (y >= h as f32 && e.vel.1 > 0.0)
        || (y + e.h() <= 0.0 && e.vel.1 < 0.0)
}

fn day_factor(s: &dyn Scenario, t: f32) -> f32 {
    match s.day_length() {
        // Smooth day/night wave, starting in the late morning.
        Some(len) => {
            let phase = (t / len + 0.15) * std::f32::consts::TAU;
            (0.5 + 0.65 * phase.sin()).clamp(0.0, 1.0)
        }
        None => 1.0,
    }
}
