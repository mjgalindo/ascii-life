use crate::engine::sprite::rgb;
use crate::engine::*;
use ratatui::style::Color;

const DAY: f32 = 90.0;
const DAYS_PER_SEASON: f32 = 3.0;
const SEASON_NAMES: [&str; 4] = ["spring", "summer", "autumn", "winter"];
const GROUND: [Color; 4] = [rgb(74, 124, 62), rgb(64, 114, 48), rgb(116, 104, 52), rgb(206, 214, 228)];

/// Fraction of the day with 0.0 = midnight and 0.5 = noon, aligned with the engine's daylight wave.
fn clock(t: f32, day: f32) -> f32 {
    (t / day + 0.4).fract()
}

fn season(t: f32) -> usize {
    ((t / DAY / DAYS_PER_SEASON) as usize) % 4
}

#[derive(Clone, Copy, PartialEq)]
enum Job {
    Farmer,
    Woodcutter,
    Baker,
    Merchant,
    Child,
    Guard,
}

impl Job {
    fn glyph(self) -> (char, Color) {
        match self {
            Job::Farmer => ('Y', rgb(230, 200, 90)),
            Job::Woodcutter => ('T', rgb(190, 70, 60)),
            Job::Baker => ('B', rgb(240, 240, 240)),
            Job::Merchant => ('$', rgb(120, 200, 255)),
            Job::Child => ('i', rgb(255, 150, 200)),
            Job::Guard => ('#', rgb(150, 160, 190)),
        }
    }

    fn line(self) -> &'static str {
        match self {
            Job::Farmer => "",
            Job::Woodcutter => "chop!",
            Job::Baker => "*bake*",
            Job::Merchant => "$$$",
            Job::Child => "wheee!",
            Job::Guard => "halt!",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Goal {
    Home,
    Work,
    Social,
}

fn goal_for(job: Job, c: f32) -> Goal {
    if job == Job::Guard {
        // Night watch: sleeps through the middle of the day.
        return if (0.27..0.58).contains(&c) { Goal::Home } else { Goal::Work };
    }
    if !(0.22..0.92).contains(&c) {
        Goal::Home
    } else if c < 0.28 {
        Goal::Home
    } else if c < 0.63 {
        Goal::Work
    } else if c < 0.84 {
        Goal::Social
    } else {
        Goal::Home
    }
}

struct Villager {
    day: f32,
    job: Job,
    home: (f32, f32),
    work: (f32, f32),
    social: (f32, f32),
    goal_spot: (f32, f32),
    hidden: bool,
    body: Sprite,
    speed: f32,
    spot_cd: f32,
    work_cd: f32,
    trail_cd: f32,
}

impl Villager {
    fn target_base(&self, g: Goal) -> (f32, f32) {
        match g {
            Goal::Home => self.home,
            Goal::Work => self.work,
            Goal::Social => self.social,
        }
    }
}

impl Behavior for Villager {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, dt: f32) {
        let c = clock(cx.time, self.day);
        let goal = goal_for(self.job, c);
        let base = self.target_base(goal);
        self.spot_cd -= dt;
        if self.spot_cd <= 0.0 {
            self.spot_cd = cx.rng.range(2.0, 6.0);
            let r = if goal == Goal::Home { 0.0 } else if self.job == Job::Guard { 14.0 } else { 3.5 };
            self.goal_spot = (
                base.0 + cx.rng.range(-r, r),
                base.1 + cx.rng.range(-r * 0.4, r * 0.4),
            );
        }
        if goal == Goal::Home {
            self.goal_spot = base;
        }

        let (dx, dy) = (self.goal_spot.0 - e.pos.0, self.goal_spot.1 - e.pos.1);
        let dist = (dx * dx + (dy * 2.0) * (dy * 2.0)).sqrt();

        if goal == Goal::Home && dist < 0.8 {
            if !self.hidden {
                self.hidden = true;
                e.set_sprite(Sprite::one(Frame::blank(1, 2)));
            }
            e.vel = (0.0, 0.0);
            return;
        }
        if self.hidden {
            self.hidden = false;
            e.set_sprite(self.body.clone());
        }
        if dist < 0.8 {
            e.vel = (0.0, 0.0);
            self.work_cd -= dt;
            if goal == Goal::Work && self.work_cd <= 0.0 {
                self.work_cd = cx.rng.range(6.0, 12.0);
                match self.job {
                    Job::Woodcutter => cx.add("wood", 1.0),
                    Job::Baker => cx.add("food", 1.5),
                    Job::Merchant => cx.add("gold", 2.0),
                    _ => {}
                }
                let line = self.job.line();
                if !line.is_empty() {
                    cx.spawn(
                        Entity::new("emote", Sprite::mono(line, rgb(255, 250, 210)))
                            .at(e.pos.0, e.pos.1 - 1.0)
                            .vel(0.0, -1.0)
                            .z(2)
                            .lifetime(1.6),
                    );
                }
            }
            return;
        }
        e.vel = (dx / dist * self.speed, dy / dist * self.speed * 0.5);

        // Worn footpaths emerge where villagers walk.
        self.trail_cd -= dt;
        if self.trail_cd <= 0.0 && cx.daylight > 0.0 {
            self.trail_cd = 0.7;
            let dirt = Frame::filled(1, 1, ' ', rgb(150, 124, 86), Some(rgb(126, 100, 68)));
            cx.spawn(
                Entity::new("trail", Sprite::one(dirt))
                    .at(e.pos.0.round(), (e.pos.1 + 1.0).round())
                    .z(95)
                    .lifetime(70.0),
            );
        }
    }
}

struct Chimney;
impl Behavior for Chimney {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, _dt: f32) {
        let lit = cx.daylight < 0.55;
        e.frame = lit as usize;
        if lit && cx.rng.chance(0.03) {
            cx.spawn(
                Entity::new("smoke", Sprite::mono("o", rgb(190, 190, 195)))
                    .at(e.pos.0 + 6.0, e.pos.1 - 1.0)
                    .vel(0.5, -1.4)
                    .z(3)
                    .lifetime(2.6),
            );
        }
    }
}

/// Windows glow after dark.
struct Lit;
impl Behavior for Lit {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, _dt: f32) {
        e.frame = (cx.daylight < 0.5) as usize;
    }
}

struct Tree;
impl Behavior for Tree {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, _dt: f32) {
        e.frame = season(cx.time);
    }
}

/// Crops grow through the warm seasons and ripen when a farmer is close.
struct Field {
    growth: f32,
}
impl Behavior for Field {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, dt: f32) {
        let s = season(cx.time);
        if s != 3 && self.growth < 1.0 {
            self.growth = (self.growth + dt / 55.0).min(1.0);
        }
        if s == 3 && self.growth >= 1.0 {
            self.growth = 0.0;
        }
        e.frame = match self.growth {
            g if g < 0.33 => 0,
            g if g < 0.66 => 1,
            g if g < 1.0 => 2,
            _ => 3,
        };
        if self.growth >= 1.0 {
            let r = e.rect();
            let near = cx
                .snap
                .query(r.0 - 2, r.1 - 2, r.2 + 4, r.3 + 4)
                .iter()
                .any(|o| o.kind == "villager" && o.state == Job::Farmer as i32);
            if near {
                self.growth = 0.0;
                cx.add("food", 10.0);
                cx.spawn(
                    Entity::new("emote", Sprite::mono("harvest!", rgb(255, 240, 150)))
                        .at(e.pos.0 + 2.0, e.pos.1 - 1.0)
                        .vel(0.0, -1.0)
                        .z(2)
                        .lifetime(1.8),
                );
            }
        }
    }
}

struct Chicken {
    cd: f32,
    target: (f32, f32),
}
impl Behavior for Chicken {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, dt: f32) {
        self.cd -= dt;
        if self.cd <= 0.0 {
            self.cd = cx.rng.range(1.0, 5.0);
            if cx.rng.chance(0.5) {
                e.vel = (0.0, 0.0);
            } else {
                self.target = (
                    (e.pos.0 + cx.rng.range(-6.0, 6.0)).clamp(1.0, cx.w - 2.0),
                    (e.pos.1 + cx.rng.range(-2.0, 2.0)).clamp(1.0, cx.h - 2.0),
                );
            }
        }
        let (dx, dy) = (self.target.0 - e.pos.0, self.target.1 - e.pos.1);
        let d = (dx * dx + dy * dy * 4.0).sqrt();
        e.vel = if d > 0.5 { (dx / d * 2.0, dy / d) } else { (0.0, 0.0) };
    }
}

struct Weather;
impl Behavior for Weather {}

fn frames_for(art: &str, roof_rows: usize, roof: Color, wall: Color) -> Sprite {
    let lines: Vec<&str> = art.lines().collect();
    let w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let make = |night: bool| {
        let mut f = Frame::blank(w, lines.len());
        for (y, l) in lines.iter().enumerate() {
            for (x, ch) in l.chars().enumerate() {
                if ch == ' ' && y >= roof_rows {
                    f.set(x, y, ' ', wall, Some(wall));
                    continue;
                }
                if ch == ' ' {
                    continue;
                }
                let window = ch == '[' || ch == ']';
                let (fg, bg) = if y < roof_rows {
                    (roof, None)
                } else if window {
                    (if night { rgb(255, 225, 110) } else { rgb(90, 130, 190) }, Some(wall))
                } else {
                    (rgb(235, 225, 205), Some(wall))
                };
                f.set(x, y, ch, fg, bg);
            }
        }
        f
    };
    Sprite::new(vec![make(false), make(true)])
}

const HOUSE: &str = "   /^\\   \n  /   \\  \n /_____\\ \n |[] []| \n |_[ ]_| ";
const TAVERN: &str = "  _________  \n /  TAVERN \\ \n/___________\\\n |[] [] []| \n |_[_ _]_|_|";
const BAKERY: &str = "  .-------.\n /  BAKERY\\\n/_________\\\n |[] [] []|\n |___[_]__|";
const MARKET: &str = "  ~~~~~~~~~~~\n /=/=/=/=/=/=\\\n  MARKET  |  |\n  |_______|__|";
const WELL: &str = " _____ \n |~~~| \n |___| ";

fn tree_sprite() -> Sprite {
    let seasons = [
        ("&&&", rgb(120, 200, 100)),
        ("&&&", rgb(60, 150, 60)),
        ("&&&", rgb(220, 120, 40)),
        ("\\|/", rgb(240, 240, 250)),
    ];
    Sprite::new(
        seasons
            .iter()
            .map(|&(top, col)| {
                let mut f = Frame::blank(5, 3);
                let crown = if top == "\\|/" { [" \\|/ ", " -|- "] } else { [" &&& ", "&&&&&"] };
                for (y, row) in crown.iter().enumerate() {
                    for (x, ch) in row.chars().enumerate() {
                        if ch != ' ' {
                            f.set(x, y, ch, col, None);
                        }
                    }
                }
                f.set(2, 2, '|', rgb(110, 80, 50), None);
                f
            })
            .collect(),
    )
}

fn field_sprite(w: usize) -> Sprite {
    let soil = rgb(94, 68, 44);
    let mk = |ch: char, fg: Color| {
        let mut f = Frame::filled(w, 3, ch, fg, Some(soil));
        for x in 0..w {
            for y in 0..3 {
                if (x + y) % 2 == 0 {
                    f.set(x, y, ' ', fg, Some(soil));
                }
            }
        }
        f
    };
    Sprite::new(vec![
        mk('.', rgb(130, 100, 70)),
        mk(',', rgb(110, 190, 80)),
        mk('i', rgb(150, 200, 70)),
        mk('#', rgb(235, 200, 70)),
    ])
}

#[derive(Default)]
pub struct Village {
    homes: Vec<(f32, f32)>,
    work_sites: Vec<(Job, (f32, f32))>,
    plaza: (f32, f32),
    tavern: (f32, f32),
    forest: Vec<(f32, f32)>,
    season: usize,
    births_cd: f32,
}

fn free(rects: &[(i32, i32, i32, i32)], r: (i32, i32, i32, i32), margin: i32) -> bool {
    let grown = (r.0 - margin, r.1 - margin, r.2 + 2 * margin, r.3 + 2 * margin);
    !rects.iter().any(|&o| crate::engine::world::overlaps(grown, o))
}

fn place(
    cx: &mut Ctx,
    rects: &mut Vec<(i32, i32, i32, i32)>,
    w: i32,
    h: i32,
    margin: i32,
) -> Option<(i32, i32)> {
    for _ in 0..80 {
        let x = cx.rng.int(2, (cx.w as i32 - w - 2).max(3));
        let y = cx.rng.int(2, (cx.h as i32 - h - 3).max(3));
        if free(rects, (x, y, w, h), margin) {
            rects.push((x, y, w, h));
            return Some((x, y));
        }
    }
    None
}

fn make_villager(
    cx: &mut Ctx,
    job: Job,
    home: (f32, f32),
    work: (f32, f32),
    social: (f32, f32),
    start: (f32, f32),
    day: f32,
) {
    let (g, col) = job.glyph();
    let mut f = Frame::blank(1, 2);
    f.set(0, 0, 'o', rgb(235, 200, 160), None);
    f.set(0, 1, g, col, None);
    let body = Sprite::one(f);
    let villager = Villager {
        day,
        job,
        home,
        work,
        social,
        goal_spot: home,
        hidden: false,
        body: body.clone(),
        speed: cx.rng.range(3.0, 5.0),
        spot_cd: 0.0,
        work_cd: cx.rng.range(1.0, 6.0),
        trail_cd: cx.rng.range(0.0, 1.0),
    };
    let mut e = Entity::new("villager", body).at(start.0, start.1).z(20).with(villager);
    e.state = job as i32;
    cx.spawn(e);
    cx.add("pop", 1.0);
}

fn pick_chicken(cx: &mut Ctx, x: f32, y: f32) {
    let hen = Sprite::mono("c'", rgb(240, 230, 210)).facing(cx.rng.chance(0.5));
    cx.spawn(Entity::new("chicken", hen).at(x, y).z(22).with(Chicken { cd: 0.0, target: (x, y) }));
}

/// A small farmstead: house with chimney, a field, a farmer and some hens.
pub fn yard(cx: &mut Ctx, rect: (i32, i32, i32, i32), day: f32) {
    let (x, y, w, h) = rect;
    if h < 6 || w < 16 {
        return;
    }
    let sprite = frames_for(HOUSE, 3, rgb(176, 60, 50), rgb(150, 120, 90));
    let (hx, hy) = (x + 1, y + h - 5);
    cx.spawn(Entity::new("house", sprite).at(hx as f32, hy as f32).z(40).with(Lit));
    cx.spawn(
        Entity::new("chimney", Sprite::new(vec![Frame::blank(1, 1), Frame::blank(1, 1)]))
            .at(hx as f32, hy as f32)
            .with(Chimney),
    );
    let fw = (w - 12).clamp(6, 14);
    let growth = cx.rng.f();
    cx.spawn(
        Entity::new("field", field_sprite(fw as usize))
            .at((x + 11) as f32, (y + h - 4) as f32)
            .z(70)
            .with(Field { growth }),
    );
    let home = ((hx + 4) as f32, (hy + 5).min(y + h - 1) as f32);
    let work = ((x + 11 + fw / 2) as f32, (y + h - 1) as f32);
    make_villager(cx, Job::Farmer, home, work, home, home, day);
    pick_chicken(cx, (x + 10) as f32, (y + 1) as f32);
    let trees = tree_sprite();
    cx.spawn(Entity::new("tree", trees).at((x + w - 6) as f32, y as f32).z(45).with(Tree));
}

/// A pocket park with trees and a bench.
pub fn park(cx: &mut Ctx, rect: (i32, i32, i32, i32)) {
    let (x, y, w, h) = rect;
    let trees = tree_sprite();
    for _ in 0..(w * h / 24).max(2) {
        let tx = cx.rng.int(x, (x + w - 5).max(x + 1));
        let ty = cx.rng.int(y, (y + h - 3).max(y + 1));
        cx.spawn(Entity::new("tree", trees.clone()).at(tx as f32, ty as f32).z(45).with(Tree));
    }
    cx.spawn(
        Entity::new("bench", Sprite::mono("[====]", rgb(150, 100, 60)))
            .at((x + w / 2 - 3) as f32, (y + h - 1) as f32)
            .z(46),
    );
}

impl Village {
    fn spawn_villager(&self, cx: &mut Ctx, job: Job, home: (f32, f32), at_home: bool) {
        let work = self
            .work_sites
            .iter()
            .filter(|(j, _)| *j == job)
            .map(|(_, p)| *p)
            .collect::<Vec<_>>();
        let workspot = if job == Job::Woodcutter && !self.forest.is_empty() {
            *cx.rng.pick(&self.forest)
        } else if job == Job::Child || job == Job::Guard || work.is_empty() {
            self.plaza
        } else {
            *cx.rng.pick(&work)
        };
        let social = (self.tavern.0 + cx.rng.range(-2.0, 2.0), self.tavern.1);
        let start = if at_home {
            home
        } else {
            (cx.rng.range(2.0, cx.w - 3.0), cx.rng.range(2.0, cx.h - 4.0))
        };
        make_villager(cx, job, home, workspot, social, start, DAY);
    }
}

impl Scenario for Village {
    fn name(&self) -> &'static str {
        "village"
    }

    fn background(&self) -> Color {
        GROUND[self.season]
    }

    fn day_length(&self) -> Option<f32> {
        Some(DAY)
    }

    fn hud(&self, g: &Globals, _daylight: f32) -> String {
        let v = |k| g.get(k).copied().unwrap_or(0.0) as i32;
        format!(
            "{} | pop {} food {} wood {} gold {}",
            SEASON_NAMES[self.season],
            v("pop"),
            v("food"),
            v("wood"),
            v("gold")
        )
    }

    fn setup(&mut self, cx: &mut Ctx) {
        let (w, h) = (cx.w as i32, cx.h as i32);
        let mut rects: Vec<(i32, i32, i32, i32)> = vec![];
        self.plaza = (cx.w / 2.0, cx.h / 2.0);
        rects.push((w / 2 - 8, h / 2 - 4, 16, 8));
        cx.set("food", 20.0);

        // Well at the heart of the plaza.
        cx.spawn(
            Entity::new("well", Sprite::mono(WELL, rgb(150, 150, 160)))
                .at(self.plaza.0 - 3.0, self.plaza.1 - 2.0)
                .z(30),
        );

        // Tavern and market flank the plaza, bakery nearby.
        let specials: [(&str, Job, Color, Color, i32); 3] = [
            (TAVERN, Job::Child, rgb(120, 60, 40), rgb(110, 80, 60), 0),
            (BAKERY, Job::Baker, rgb(170, 100, 50), rgb(150, 120, 80), 1),
            (MARKET, Job::Merchant, rgb(190, 60, 60), rgb(130, 100, 70), 2),
        ];
        for (art, job, roof, wall, i) in specials {
            let sprite = match i {
                2 => frames_for(art, 2, roof, wall),
                _ => frames_for(art, 2, roof, wall),
            };
            let (sw, sh) = (sprite.w() as i32, sprite.h() as i32);
            let spot = place(cx, &mut rects, sw, sh, 3);
            if let Some((x, y)) = spot {
                cx.spawn(Entity::new("building", sprite).at(x as f32, y as f32).z(40).with(Lit));
                let door = ((x + sw / 2) as f32, (y + sh) as f32);
                if i == 0 {
                    self.tavern = door;
                } else {
                    self.work_sites.push((job, door));
                }
            }
        }
        if self.tavern == (0.0, 0.0) {
            self.tavern = self.plaza;
        }

        // Fields.
        for _ in 0..(1 + w / 60) {
            let fw = cx.rng.int(10, 16);
            if let Some((x, y)) = place(cx, &mut rects, fw, 3, 2) {
                let growth = cx.rng.f();
                cx.spawn(
                    Entity::new("field", field_sprite(fw as usize))
                        .at(x as f32, y as f32)
                        .z(70)
                        .with(Field { growth }),
                );
                self.work_sites.push((Job::Farmer, ((x + fw / 2) as f32, (y + 3) as f32)));
            }
        }

        // Homes.
        let n_homes = ((w * h) / 380).clamp(4, 24);
        let roofs = [rgb(176, 60, 50), rgb(150, 80, 40), rgb(70, 90, 140), rgb(110, 110, 60)];
        let walls = [rgb(150, 120, 90), rgb(130, 110, 100), rgb(160, 135, 95)];
        for _ in 0..n_homes {
            let roof = *cx.rng.pick(&roofs);
            let wall = *cx.rng.pick(&walls);
            let sprite = frames_for(HOUSE, 3, roof, wall);
            let (sw, sh) = (sprite.w() as i32, sprite.h() as i32);
            if let Some((x, y)) = place(cx, &mut rects, sw, sh, 2) {
                cx.spawn(Entity::new("house", sprite).at(x as f32, y as f32).z(40).with(Lit));
                cx.spawn(
                    Entity::new(
                        "chimney",
                        Sprite::new(vec![Frame::blank(1, 1), Frame::blank(1, 1)]),
                    )
                    .at(x as f32, y as f32)
                    .with(Chimney),
                );
                self.homes.push(((x + sw / 2) as f32, (y + sh) as f32));
            }
        }

        // Forest trees, denser away from the plaza.
        let trees = tree_sprite();
        for _ in 0..(w * h / 28) {
            let x = cx.rng.int(0, w - 5);
            let y = cx.rng.int(0, h - 4);
            let dx = (x as f32 - self.plaza.0) / w as f32;
            let dy = (y as f32 - self.plaza.1) / h as f32;
            let far = (dx * dx + dy * dy).sqrt();
            if far < 0.28 || cx.rng.f() > far * 1.6 {
                continue;
            }
            if free(&rects, (x, y, 5, 3), 0) {
                rects.push((x, y, 5, 3));
                cx.spawn(Entity::new("tree", trees.clone()).at(x as f32, y as f32).z(45).with(Tree));
                if self.forest.len() < 12 {
                    self.forest.push(((x + 2) as f32, (y + 3) as f32));
                }
            }
        }

        // Population: every home gets a family.
        let homes = self.homes.clone();
        for (i, home) in homes.iter().enumerate() {
            let job = [Job::Farmer, Job::Woodcutter, Job::Farmer, Job::Baker, Job::Merchant, Job::Guard]
                [i % 6];
            self.spawn_villager(cx, job, *home, false);
            if cx.rng.chance(0.5) {
                self.spawn_villager(cx, Job::Child, *home, false);
            }
        }

        for _ in 0..(3 + w / 20) {
            let (x, y) = (cx.rng.range(5.0, cx.w - 5.0), cx.rng.range(3.0, cx.h - 4.0));
            pick_chicken(cx, x, y);
        }
        let _ = Weather;
    }

    fn tick(&mut self, cx: &mut Ctx, dt: f32) {
        self.season = season(cx.time);
        cx.set("season", self.season as f32);

        // Food is eaten gradually; surplus lets the village grow.
        let pop = cx.snap.count("villager") as f32;
        cx.add("food", -pop * 0.012 * dt);
        if cx.global("food") < 0.0 {
            cx.set("food", 0.0);
        }

        self.births_cd -= dt;
        if self.births_cd <= 0.0 && cx.global("food") > 45.0 && !self.homes.is_empty() {
            self.births_cd = 25.0;
            cx.add("food", -25.0);
            let home = *cx.rng.pick(&self.homes);
            let job = *cx.rng.pick(&[Job::Child, Job::Farmer, Job::Woodcutter, Job::Merchant]);
            self.spawn_villager(cx, job, home, true);
        }

        // Rain in spring on odd days, snow all winter.
        let day = (cx.time / DAY) as i32;
        let (rain, snow) = (self.season == 0 && day % 2 == 1, self.season == 3);
        if rain || snow {
            let n = (cx.w / 12.0) as i32;
            for _ in 0..n {
                if cx.rng.chance(if rain { 0.5 } else { 0.25 }) {
                    let x = cx.rng.range(0.0, cx.w);
                    let e = if rain {
                        Entity::new("rain", Sprite::mono("|", rgb(130, 160, 230))).vel(-1.0, 22.0)
                    } else {
                        Entity::new("snow", Sprite::mono("*", rgb(255, 255, 255)))
                            .vel(cx.rng.range(-1.5, 1.5), 3.5)
                    };
                    cx.spawn(e.at(x, -1.0).z(1).die_offscreen());
                }
            }
        }
    }
}
