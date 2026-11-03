use super::entity::{Entity, Id, Info};
use super::rng::Rng;
use ratatui::style::Color;
use std::collections::{BTreeMap, HashMap};

pub type Globals = BTreeMap<&'static str, f32>;

const CELL_W: i32 = 12;
const CELL_H: i32 = 6;

/// Spatial index over entity bounds, rebuilt every tick for neighbor queries.
pub struct Snapshot {
    pub infos: Vec<Info>,
    grid: Vec<Vec<u32>>,
    gw: i32,
    gh: i32,
    by_id: HashMap<Id, u32>,
    counts: HashMap<&'static str, usize>,
}

impl Snapshot {
    pub fn empty() -> Snapshot {
        Snapshot {
            infos: vec![],
            grid: vec![],
            gw: 0,
            gh: 0,
            by_id: HashMap::new(),
            counts: HashMap::new(),
        }
    }

    fn build(ents: &[Entity], w: i32, h: i32) -> Snapshot {
        let gw = (w / CELL_W + 1).max(1);
        let gh = (h / CELL_H + 1).max(1);
        let mut s = Snapshot {
            infos: Vec::with_capacity(ents.len()),
            grid: vec![Vec::new(); (gw * gh) as usize],
            gw,
            gh,
            by_id: HashMap::with_capacity(ents.len()),
            counts: HashMap::new(),
        };
        for e in ents.iter().filter(|e| e.alive) {
            let idx = s.infos.len() as u32;
            let info = Info::of(e);
            s.by_id.insert(info.id, idx);
            *s.counts.entry(info.kind).or_insert(0) += 1;
            let (x0, y0, x1, y1) = s.cells(info.rect);
            for cy in y0..=y1 {
                for cx in x0..=x1 {
                    s.grid[(cy * gw + cx) as usize].push(idx);
                }
            }
            s.infos.push(info);
        }
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
        if self.grid.is_empty() {
            return vec![];
        }
        let (x0, y0, x1, y1) = self.cells((x, y, w, h));
        let mut out: Vec<&Info> = Vec::new();
        for cy in y0..=y1 {
            for cx in x0..=x1 {
                for &i in &self.grid[(cy * self.gw + cx) as usize] {
                    let inf = &self.infos[i as usize];
                    if overlaps(inf.rect, (x, y, w, h)) && !out.iter().any(|o| o.id == inf.id) {
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

/// Everything a behavior or scenario may touch during a tick.
pub struct Ctx<'a> {
    pub rng: &'a mut Rng,
    pub w: f32,
    pub h: f32,
    pub time: f32,
    /// 1.0 at noon, 0.0 at midnight; constant 1.0 for scenarios without a day cycle.
    pub daylight: f32,
    pub snap: &'a Snapshot,
    pub globals: &'a mut Globals,
    next_id: &'a mut Id,
    spawned: Vec<Entity>,
    killed: Vec<Id>,
}

impl Ctx<'_> {
    pub fn spawn(&mut self, mut e: Entity) -> Id {
        *self.next_id += 1;
        e.id = *self.next_id;
        let id = e.id;
        self.spawned.push(e);
        id
    }

    pub fn kill(&mut self, id: Id) {
        self.killed.push(id);
    }

    /// A context over a different snapshot that shares this one's rng, globals and id counter.
    /// Spawns and kills are folded back by `absorb`.
    fn reborrow<'b>(&'b mut self, snap: &'b Snapshot) -> Ctx<'b> {
        Ctx {
            rng: &mut *self.rng,
            w: self.w,
            h: self.h,
            time: self.time,
            daylight: self.daylight,
            snap,
            globals: &mut *self.globals,
            next_id: &mut *self.next_id,
            spawned: vec![],
            killed: vec![],
        }
    }

    fn absorb(&mut self, mut spawned: Vec<Entity>, mut killed: Vec<Id>) {
        self.spawned.append(&mut spawned);
        self.killed.append(&mut killed);
    }

    pub fn add(&mut self, key: &'static str, amount: f32) {
        *self.globals.entry(key).or_insert(0.0) += amount;
    }

    pub fn global(&self, key: &str) -> f32 {
        self.globals.get(key).copied().unwrap_or(0.0)
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
    next_id: Id,
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
            next_id: 0,
        };
        world.setup();
        world
    }

    fn setup(&mut self) {
        let snap = Snapshot::empty();
        let mut scen = self.scenario.take().unwrap();
        self.daylight = day_factor(scen.as_ref(), 0.0);
        let mut cx = Ctx {
            rng: &mut self.rng,
            w: self.w as f32,
            h: self.h as f32,
            time: 0.0,
            daylight: self.daylight,
            snap: &snap,
            globals: &mut self.globals,
            next_id: &mut self.next_id,
            spawned: vec![],
            killed: vec![],
        };
        scen.setup(&mut cx);
        self.ents.append(&mut cx.spawned);
        self.scenario = Some(scen);
    }

    pub fn scenario_name(&self) -> &'static str {
        self.scenario.as_ref().map_or("", |s| s.name())
    }

    pub fn tick(&mut self, dt: f32) {
        self.time += dt;
        let mut scen = self.scenario.take().unwrap();
        self.daylight = day_factor(scen.as_ref(), self.time);
        let (w, h) = (self.w, self.h);

        let snap = Snapshot::build(&self.ents, w, h);
        let mut cx = Ctx {
            rng: &mut self.rng,
            w: w as f32,
            h: h as f32,
            time: self.time,
            daylight: self.daylight,
            snap: &snap,
            globals: &mut self.globals,
            next_id: &mut self.next_id,
            spawned: vec![],
            killed: vec![],
        };

        scen.tick(&mut cx, dt);

        for e in self.ents.iter_mut().filter(|e| e.alive) {
            if let Some(mut b) = e.behavior.take() {
                b.update(e, &mut cx, dt);
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

        // Collisions use positions after movement.
        if self.ents.iter().any(|e| e.physical && e.alive) {
            let snap2 = Snapshot::build(&self.ents, w, h);
            let mut hit_cx = cx.reborrow(&snap2);
            for e in self.ents.iter_mut().filter(|e| e.physical && e.alive) {
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
                            b.on_collide(e, o, &mut hit_cx);
                        }
                    }
                    e.behavior = Some(b);
                }
            }
            let (spawned, killed) = (hit_cx.spawned, hit_cx.killed);
            cx.absorb(spawned, killed);
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

        for id in std::mem::take(&mut cx.killed) {
            if let Some(e) = self.ents.iter_mut().find(|e| e.id == id) {
                e.alive = false;
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
        for e in dead.iter_mut() {
            if let Some(mut b) = e.behavior.take() {
                b.on_death(e, &mut cx);
            }
        }

        let mut spawned = std::mem::take(&mut cx.spawned);
        drop(cx);
        self.ents.append(&mut spawned);
        self.scenario = Some(scen);
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
