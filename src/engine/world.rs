use super::entity::{Entity, Id, Info};
use super::rng::Rng;
use ratatui::style::Color;
use rayon::prelude::*;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

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
    /// Indices of each kind, so rare kinds can be scanned without touching the crowd.
    kinds: FastMap<&'static str, Vec<u32>>,
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
            kinds: FastMap::default(),
            big: vec![],
        }
    }

    fn build(ents: &[Entity], w: i32, h: i32) -> Snapshot {
        let gw = (w / CELL_W + 1).max(1);
        let gh = (h / CELL_H + 1).max(1);
        let infos: Vec<Info> = ents.par_iter().filter(|e| e.alive && !e.scenery).map(Info::of).collect();
        let mut s = Snapshot {
            infos,
            cell_start: vec![],
            cell_items: vec![],
            gw,
            gh,
            by_id: FastMap::default(),
            kinds: FastMap::default(),
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
            s.kinds.entry(info.kind).or_default().push(idx as u32);
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
        let mut out = Vec::new();
        self.each(x, y, w, h, |i| out.push(i));
        out
    }

    /// Visits overlapping entities once each, without allocating.
    pub fn each<'s>(&'s self, x: i32, y: i32, w: i32, h: i32, mut visit: impl FnMut(&'s Info)) {
        if self.cell_start.is_empty() {
            return;
        }
        let (x0, y0, x1, y1) = self.cells((x, y, w, h));
        for &i in &self.big {
            let inf = &self.infos[i as usize];
            if overlaps(inf.rect, (x, y, w, h)) {
                visit(inf);
            }
        }
        for cy in y0..=y1 {
            for cx in x0..=x1 {
                let c = (cy * self.gw + cx) as usize;
                let range = self.cell_start[c] as usize..self.cell_start[c + 1] as usize;
                for &i in &self.cell_items[range] {
                    let inf = &self.infos[i as usize];
                    if !overlaps(inf.rect, (x, y, w, h)) {
                        continue;
                    }
                    // An entity spanning cells counts only in the first cell shared with the query.
                    let own = self.cells(inf.rect);
                    if own.0.max(x0) == cx && own.1.max(y0) == cy {
                        visit(inf);
                    }
                }
            }
        }
    }

    pub fn get(&self, id: Id) -> Option<&Info> {
        self.by_id.get(&id).map(|&i| &self.infos[i as usize])
    }

    pub fn count(&self, kind: &str) -> usize {
        self.kinds.get(kind).map_or(0, Vec::len)
    }

    pub fn of_kind<'a>(&'a self, kind: &str) -> impl Iterator<Item = &'a Info> {
        self.kinds.get(kind).into_iter().flatten().map(|&i| &self.infos[i as usize])
    }
}

pub fn overlaps(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
    a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
}

/// Entities handled per parallel work unit. Fixed so results do not depend on thread count.
const CHUNK: usize = 192;
/// Chunk slot reserved for work done on the calling thread.
const MAIN_SLOT: u64 = 0xFFF;
/// Chunk slot reserved for viewer actions.
const VIEWER_SLOT: u64 = 0xFFE;
/// Headlines kept for the log panel.
const NEWS_KEPT: usize = 200;

/// A notable happening, optionally tied to a place the camera can jump to.
#[derive(Clone)]
pub struct News {
    pub time: f64,
    pub text: String,
    pub pos: Option<(f32, f32)>,
}

/// Read-only tick parameters shared by every work unit.
#[derive(Clone, Copy)]
struct TickEnv<'a> {
    w: f32,
    h: f32,
    time: f64,
    daylight: f32,
    snap: &'a Snapshot,
    base: &'a Globals,
}

/// Everything a behavior or scenario may touch during a tick.
/// Writes (spawns, kills, global changes) are buffered and merged after each phase.
pub struct Ctx<'a> {
    pub rng: &'a mut Rng,
    pub w: f32,
    pub h: f32,
    /// Seconds since the world began; double precision so long fast-forwards keep ticking.
    pub time: f64,
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
    news: Vec<News>,
}

/// Buffered output of one work unit.
struct Out {
    spawned: Vec<Entity>,
    killed: Vec<Id>,
    adds: Globals,
    sets: Globals,
    news: Vec<News>,
}

impl<'a> Ctx<'a> {
    fn new(rng: &'a mut Rng, env: TickEnv<'a>, epoch: u64, slot: u64) -> Ctx<'a> {
        Ctx {
            rng,
            w: env.w,
            h: env.h,
            time: env.time,
            daylight: env.daylight,
            snap: env.snap,
            base: env.base,
            adds: Globals::new(),
            sets: Globals::new(),
            id_base: (epoch << 32) | (slot << 20),
            id_next: 0,
            spawned: vec![],
            killed: vec![],
            news: vec![],
        }
    }

    fn finish(self) -> Out {
        Out { spawned: self.spawned, killed: self.killed, adds: self.adds, sets: self.sets, news: self.news }
    }

    /// Publishes a headline; `pos` lets the viewer jump to where it happened.
    pub fn news(&mut self, text: impl Into<String>, pos: Option<(f32, f32)>) {
        self.news.push(News { time: self.time, text: text.into(), pos });
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

/// How a world's flat coordinates map onto space in the perspective view.
#[derive(Clone, Copy, PartialEq)]
pub enum Space {
    /// Seen from above: `y` runs along the ground and sprites stand on it.
    Ground,
    /// Seen from the side: `y` is height, and creatures spread out behind the glass.
    Tank,
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
    /// World size this scenario is designed around; `None` follows the terminal.
    fn preferred_size(&self) -> Option<(i32, i32)> {
        None
    }
    /// Things the viewer can add to this world, listed in the add menu.
    fn tools(&self) -> &'static [&'static str] {
        &[]
    }
    /// Carries out entry `tool` of `tools` at a world position.
    fn place(&mut self, _cx: &mut Ctx, _tool: usize, _x: f32, _y: f32) {}
    /// Whether the world is exactly as large as the screen, with no scrolling.
    fn fits_terminal(&self) -> bool {
        false
    }
    fn space(&self) -> Space {
        Space::Ground
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
    pub time: f64,
    pub daylight: f32,
    pub globals: Globals,
    pub news: VecDeque<News>,
    pub scenario: Option<Box<dyn Scenario>>,
    pub ticks: u64,
    placed: u64,
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
            news: VecDeque::new(),
            scenario: Some(scenario),
            ticks: 0,
            placed: 0,
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
        let env = TickEnv {
            w: self.w as f32,
            h: self.h as f32,
            time: 0.0,
            daylight: self.daylight,
            snap: &snap,
            base: &base,
        };
        let mut cx = Ctx::new(&mut self.rng, env, 0, MAIN_SLOT);
        scen.setup(&mut cx);
        let out = cx.finish();
        self.merge(vec![out]);
        self.scenario = Some(scen);
    }

    /// Folds buffered phase outputs into the world in a fixed order.
    fn merge(&mut self, outs: Vec<Out>) {
        let mut killed: HashSet<Id, std::hash::BuildHasherDefault<FastHasher>> = HashSet::default();
        let mut spawned = vec![];
        for mut o in outs {
            for (k, v) in o.sets {
                self.globals.insert(k, v);
            }
            for (k, v) in o.adds {
                *self.globals.entry(k).or_insert(0.0) += v;
            }
            killed.extend(o.killed);
            spawned.append(&mut o.spawned);
            for n in o.news {
                self.news.push_back(n);
                if self.news.len() > NEWS_KEPT {
                    self.news.pop_front();
                }
            }
        }
        if !killed.is_empty() {
            self.ents.par_iter_mut().filter(|e| killed.contains(&e.id)).for_each(|e| e.alive = false);
        }
        self.ents.append(&mut spawned);
    }

    /// Applies a viewer action right away, so it also works while paused.
    pub fn place(&mut self, tool: usize, x: f32, y: f32) {
        let mut scen = self.scenario.take().unwrap();
        let snap = Snapshot::build(&self.ents, self.w, self.h);
        let base = self.globals.clone();
        let env = TickEnv { w: self.w as f32, h: self.h as f32, time: self.time, daylight: self.daylight, snap: &snap, base: &base };
        let mut cx = Ctx::new(&mut self.rng, env, self.ticks * 4 + 3, VIEWER_SLOT);
        // Ids must stay unique across several placements within one tick.
        cx.id_next = self.placed << 12;
        scen.place(&mut cx, tool, x, y);
        let out = cx.finish();
        self.placed += 1;
        self.merge(vec![out]);
        self.scenario = Some(scen);
    }

    pub fn scenario_name(&self) -> &'static str {
        self.scenario.as_ref().map_or("", |s| s.name())
    }

    pub fn tick(&mut self, dt: f32) {
        self.time += dt as f64;
        self.ticks += 1;
        let epoch = self.ticks * 4;
        let mut scen = self.scenario.take().unwrap();
        self.daylight = day_factor(scen.as_ref(), self.time);
        let (w, h) = (self.w, self.h);
        let base = self.globals.clone();

        // Scenario logic runs first, on this thread, against the start-of-tick snapshot.
        let snap = Snapshot::build(&self.ents, w, h);
        let mut scen_rng = Rng::new(self.seed ^ self.ticks.wrapping_mul(0x9E37_79B9));
        let env = TickEnv { w: w as f32, h: h as f32, time: self.time, daylight: self.daylight, snap: &snap, base: &base };
        let mut cx = Ctx::new(&mut scen_rng, env, epoch, MAIN_SLOT);
        scen.tick(&mut cx, dt);
        let scen_out = cx.finish();

        // Behavior phase: entities update in parallel, each chunk with its own rng and buffers.
        let seed = self.seed;
        let tick = self.ticks;
        let mut outs: Vec<Out> = vec![scen_out];
        let time_now = self.time;
        let daylight_now = self.daylight;
        let chunk_outs: Vec<Out> = self
            .ents
            .par_chunks_mut(CHUNK)
            .enumerate()
            .map(|(ci, chunk)| {
                let mut rng = Rng::new(seed ^ tick.wrapping_mul(0xA24B_AED4) ^ (ci as u64 + 1).wrapping_mul(0x9FB2_1C65));
                let env = TickEnv { w: w as f32, h: h as f32, time: time_now, daylight: daylight_now, snap: &snap, base: &base };
                let mut cx = Ctx::new(&mut rng, env, epoch + 1, ci as u64);
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
        if self.ents.iter().any(active_collider) {
            let snap2 = Snapshot::build(&self.ents, w, h);
            let env = TickEnv { snap: &snap2, ..env_static(self.w, self.h, self.time, self.daylight, &base) };
            let outs: Vec<Out> = self
                .ents
                .par_chunks_mut(CHUNK)
                .enumerate()
                .map(|(ci, chunk)| {
                    let mut rng = Rng::new(seed ^ tick.wrapping_mul(0xC2B2_AE35) ^ (ci as u64 + 7).wrapping_mul(0x85EB_CA6B));
                    let mut cx = Ctx::new(&mut rng, env, epoch + 2, ci as u64);
                    for e in chunk.iter_mut().filter(|e| active_collider(e)) {
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
        if self.ents.iter().any(|e| e.follow.is_some()) {
            let mut positions: FastMap<Id, (f32, f32)> =
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
            let env = TickEnv { snap: &snap3, ..env_static(self.w, self.h, self.time, self.daylight, &base) };
            let mut cx = Ctx::new(&mut rng, env, epoch + 3, MAIN_SLOT);
            for e in dead.iter_mut() {
                if let Some(mut b) = e.behavior.take() {
                    b.on_death(e, &mut cx);
                }
            }
            let out = cx.finish();
            self.merge(vec![out]);
        }
        self.scenario = Some(scen);
        // A lightning flash brightens the picture without convincing anything that it is day.
        self.daylight = self.daylight.max(self.globals.get("flash").copied().unwrap_or(0.0));
    }
}

fn env_static<'a>(w: i32, h: i32, time: f64, daylight: f32, base: &'a Globals) -> TickEnv<'a> {
    static EMPTY: std::sync::OnceLock<Snapshot> = std::sync::OnceLock::new();
    TickEnv { w: w as f32, h: h as f32, time, daylight, snap: EMPTY.get_or_init(Snapshot::empty), base }
}

/// Only behaviors that react to contact pay for the neighbor search.
fn active_collider(e: &Entity) -> bool {
    e.alive && e.physical && e.behavior.as_ref().is_some_and(|b| b.wants_collisions())
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

fn day_factor(s: &dyn Scenario, t: f64) -> f32 {
    match s.day_length() {
        // Smooth day/night wave, starting in the late morning.
        Some(len) => {
            let phase = (t / len as f64 + 0.15).fract() * std::f64::consts::TAU;
            (0.5 + 0.65 * phase.sin() as f32).clamp(0.0, 1.0)
        }
        None => 1.0,
    }
}
