use super::common::{firework, floating_text, random_name, Spark, Weather};
use crate::engine::sprite::rgb;
use crate::engine::*;
use ratatui::style::Color;
use std::sync::{Arc, Mutex};

const DAY: f32 = 90.0;
const DAYS_PER_SEASON: f32 = 3.0;
/// Time compression for life stages; a village sees several generations in an evening.
const YEARS_PER_DAY: f32 = 4.0;
/// Lumber a new house costs.
const HOUSE_WOOD: f32 = 16.0;
const SEASON_NAMES: [&str; 4] = ["spring", "summer", "autumn", "winter"];
const GROUND: [Color; 4] = [rgb(74, 124, 62), rgb(64, 114, 48), rgb(116, 104, 52), rgb(206, 214, 228)];

/// Fraction of the day with 0.0 = midnight and 0.5 = noon, aligned with the engine's daylight wave.
fn clock(t: f64, day: f32) -> f32 {
    (t / day as f64 + 0.4).fract() as f32
}

/// Season for a day length; a non-positive length means the default.
fn season(t: f64, day: f32) -> usize {
    let day = if day > 0.0 { day } else { DAY };
    ((t / day as f64 / DAYS_PER_SEASON as f64) as usize) % 4
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
    if !(0.28..0.84).contains(&c) {
        Goal::Home
    } else if c < 0.63 {
        Goal::Work
    } else {
        Goal::Social
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
    name: &'static str,
    /// Age in years; `None` lifespan means the person never dies (city dioramas).
    age: f32,
    lifespan: Option<f32>,
    /// What a child becomes on growing up.
    adult: Option<(Job, (f32, f32))>,
    /// Places a woodcutter rotates between.
    woods: Arc<Vec<(f32, f32)>>,
    last_goal: Option<Goal>,
    alert_cd: f32,
    wolf: Option<(f32, f32)>,
    cause: &'static str,
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
        self.age += dt / self.day * YEARS_PER_DAY;
        if let Some(life) = self.lifespan {
            let famine = cx.global("famine") > 0.0;
            if self.age > life {
                self.cause = "of old age";
                e.kill();
                return;
            }
            if famine && cx.rng.chance(0.0002) {
                self.cause = "of hunger";
                e.kill();
                return;
            }
        }
        if let (true, Some((job, work))) = (self.age >= 16.0, self.adult) {
            self.job = job;
            self.work = work;
            self.adult = None;
            e.state = job as i32;
            let (g, col) = job.glyph();
            let mut f = Frame::blank(1, 2);
            f.set(0, 0, 'o', rgb(235, 200, 160), None);
            f.set(0, 1, g, col, None);
            self.body = Sprite::one(f).with_pixels(vec![crate::art::person(col, None)]);
            if !self.hidden {
                e.set_sprite(self.body.clone());
            }
        }

        let c = clock(cx.time, self.day);
        let mut goal = goal_for(self.job, c);
        if self.job != Job::Guard && goal != Goal::Home && cx.global("festival") > 0.0 && c > 0.55 {
            goal = Goal::Social;
        }
        if goal == Goal::Work && self.last_goal != Some(Goal::Work) && !self.woods.is_empty() && self.job == Job::Woodcutter {
            self.work = *cx.rng.pick(&self.woods);
        }
        self.last_goal = Some(goal);
        let mut base = self.target_base(goal);
        let mut hurry = 1.0;

        // Everyone able drops what they are doing to fight a nearby fire.
        let fire = (cx.global("fire_x"), cx.global("fire_y"));
        let on_fire = cx.global("fire_on") > 0.0 && self.job != Job::Child;
        let responding = on_fire && (fire.0 - e.pos.0).abs() < 70.0 && (fire.1 - e.pos.1).abs() < 30.0;
        if responding {
            base = fire;
            hurry = 1.7;
            goal = Goal::Work;
            self.spot_cd = self.spot_cd.min(0.2);
        }

        if self.job == Job::Guard {
            self.alert_cd -= dt;
            if self.alert_cd <= 0.0 {
                self.alert_cd = 0.8;
                let (x, y) = (e.pos.0 as i32, e.pos.1 as i32);
                self.wolf = cx.snap.query(x - 45, y - 20, 90, 40).iter().find(|o| o.kind == "wolf").map(|o| o.pos);
            }
            if let Some(w) = self.wolf {
                base = w;
                hurry = 1.8;
                goal = Goal::Work;
            }
        }

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
                    Job::Baker => cx.add("food", 1.5),
                    Job::Merchant => cx.add("gold", 2.0),
                    _ => {}
                }
                let line = self.job.line();
                if !line.is_empty() {
                    cx.spawn(
                        Entity::new("emote", Sprite::text(line, rgb(255, 250, 210)))
                            .at(e.pos.0, e.pos.1 - 1.0)
                            .vel(0.0, -1.0)
                            .z(2)
                            .lifetime(1.6),
                    );
                }
            }
            return;
        }
        let speed = self.speed * hurry;
        e.vel = (dx / dist * speed, dy / dist * speed * 0.5);
    }

    fn on_death(&mut self, e: &Entity, cx: &mut Ctx) {
        if self.lifespan.is_none() {
            return;
        }
        cx.news(format!("{} died {}, aged {:.0}.", self.name, self.cause, self.age), Some(e.pos));
        cx.add("deaths", 1.0);
        let grave = Sprite::mono(" + \n_|_", rgb(170, 170, 176));
        cx.spawn(Entity::new("grave", grave).at(e.pos.0, e.pos.1).z(60).lifetime(self.day * 6.0));
    }

    fn describe(&self, e: &Entity) -> Option<String> {
        let job = match self.job {
            Job::Farmer => "farmer",
            Job::Woodcutter => "woodcutter",
            Job::Baker => "baker",
            Job::Merchant => "merchant",
            Job::Child => "child",
            Job::Guard => "guard",
        };
        let doing = if self.hidden { "at home" } else if e.vel == (0.0, 0.0) { "busy" } else { "walking" };
        Some(format!("{}, {job}, aged {:.0}\n{doing}", self.name, self.age))
    }
}

/// Footpaths: painted from a wear map that the scenario owns and republishes.
struct Terrain {
    next: Arc<Mutex<Option<Sprite>>>,
}

impl Behavior for Terrain {
    fn update(&mut self, e: &mut Entity, _cx: &mut Ctx, _dt: f32) {
        if let Some(s) = self.next.lock().unwrap().take() {
            e.set_sprite(s);
        }
    }
}

/// A felled-and-regrowing tree. Standing woodcutters chop it down for lumber.
struct Woodlot {
    hp: f32,
    regrow: f32,
}

impl Behavior for Woodlot {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, dt: f32) {
        if self.regrow > 0.0 {
            self.regrow -= dt;
            e.frame = 4;
            if self.regrow <= 0.0 {
                self.hp = 12.0;
            }
            return;
        }
        e.frame = season(cx.time, cx.global("daylen"));
        let r = e.rect();
        let chopping = cx
            .snap
            .query(r.0 - 2, r.1 - 1, r.2 + 4, r.3 + 3)
            .iter()
            .filter(|o| o.kind == "villager" && o.state == Job::Woodcutter as i32 && o.vel == (0.0, 0.0))
            .count();
        if chopping > 0 {
            self.hp -= chopping as f32 * dt;
            if cx.rng.chance(0.01) {
                cx.spawn(floating_text("chop!", rgb(255, 250, 210), e.pos.0, e.pos.1 - 1.0));
            }
            if self.hp <= 0.0 {
                cx.add("wood", 4.0);
                self.regrow = cx.global("daylen").max(DAY) * 2.0;
                cx.spawn(floating_text("timber!", rgb(255, 230, 150), e.pos.0, e.pos.1 - 1.0));
            }
        }
    }
}

/// Burns until put out by villagers or rain. State 1 means saved, 2 means burned to the ground.
struct Fire {
    hp: f32,
    burn: f32,
}

impl Behavior for Fire {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, dt: f32) {
        let r = e.rect();
        let helpers = cx
            .snap
            .query(r.0 - 6, r.1 - 3, r.2 + 12, r.3 + 6)
            .iter()
            .filter(|o| o.kind == "villager")
            .count();
        self.hp -= helpers as f32 * dt * 0.7;
        if cx.global("rain") >= 1.0 && cx.global("rain") < 3.0 {
            self.hp -= dt * 1.5;
        }
        self.burn -= dt;
        e.state = if self.hp <= 0.0 {
            1
        } else if self.burn <= 0.0 {
            2
        } else {
            0
        };
        if cx.rng.chance(0.3) {
            let dx = cx.rng.range(0.0, e.w());
            cx.spawn(
                Entity::new("smoke", Sprite::mono("o", rgb(120, 120, 124)))
                    .at(e.pos.0 + dx, e.pos.1 - 1.0)
                    .vel(0.8, -2.2)
                    .z(3)
                    .lifetime(2.4),
            );
        }
    }

    fn describe(&self, _e: &Entity) -> Option<String> {
        Some(format!("house fire\nneeds {:.0} more buckets", self.hp.max(0.0)))
    }
}

/// House under construction: rises through its frames, then reports state 1.
struct Site {
    progress: f32,
}

impl Behavior for Site {
    fn update(&mut self, e: &mut Entity, _cx: &mut Ctx, dt: f32) {
        self.progress += dt / 45.0;
        e.frame = ((self.progress * 4.0) as usize).min(3);
        e.state = (self.progress >= 1.0) as i32;
    }

    fn describe(&self, _e: &Entity) -> Option<String> {
        Some(format!("house going up\n{:.0}% built", self.progress * 100.0))
    }
}

fn site_sprite() -> Sprite {
    let col = rgb(196, 160, 110);
    Sprite::new(vec![
        Frame::parse("         \n         \n         \n         \n _______ ", None, col, &[Color::White; 9]),
        Frame::parse("         \n         \n         \n |     | \n |_____| ", None, col, &[Color::White; 9]),
        Frame::parse("         \n         \n _______ \n |  x  | \n |_x_x_| ", None, col, &[Color::White; 9]),
        Frame::parse("   /^\\   \n  / x \\  \n /_____\\ \n |  x  | \n |_x_x_| ", None, col, &[Color::White; 9]),
    ])
}

/// Hunts chickens by night, and runs for the trees at dawn or when guards come close.
struct Wolf {
    right: bool,
    sprites: [Sprite; 2],
    think: f32,
    target: Option<(f32, f32)>,
    fleeing: bool,
}

impl Behavior for Wolf {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, dt: f32) {
        let (x, y) = (e.pos.0 as i32, e.pos.1 as i32);
        self.think -= dt;
        if self.think <= 0.0 {
            self.think = cx.rng.range(0.5, 1.2);
            let around = cx.snap.query(x - 40, y - 18, 80, 36);
            let guard_close = around.iter().any(|o| {
                o.kind == "villager"
                    && o.state == Job::Guard as i32
                    && (o.pos.0 - e.pos.0).abs() < 12.0
                    && (o.pos.1 - e.pos.1).abs() < 6.0
            });
            if (guard_close || cx.daylight > 0.45) && !self.fleeing {
                self.fleeing = true;
                e.die_offscreen = true;
                cx.news("A wolf is driven back into the forest.", Some(e.pos));
                cx.add("wolves_driven", 1.0);
            }
            self.target = around.iter().find(|o| o.kind == "chicken").map(|o| o.pos);
            if !self.fleeing && self.target.is_none() && cx.rng.chance(0.3) {
                cx.spawn(floating_text("awoo", rgb(210, 220, 240), e.pos.0, e.pos.1 - 1.0));
            }
        }
        if self.fleeing {
            let dir = if e.pos.0 < cx.w / 2.0 { -1.0 } else { 1.0 };
            e.vel = (dir * 11.0, 0.0);
        } else {
            let to = self.target.unwrap_or((cx.w / 2.0, cx.h / 2.0));
            let (dx, dy) = (to.0 - e.pos.0, to.1 - e.pos.1);
            let d = (dx * dx + dy * dy * 4.0).sqrt().max(0.1);
            e.vel = (dx / d * 5.5, dy / d * 2.7);
            if d < 2.0 {
                if let Some(hen) = cx.snap.query(x - 2, y - 1, 6, 3).into_iter().find(|o| o.kind == "chicken") {
                    cx.kill(hen.id);
                    cx.spawn(floating_text("*crunch*", rgb(255, 180, 170), e.pos.0, e.pos.1 - 1.0));
                    cx.add("hens_lost", 1.0);
                }
            }
        }
        let right = e.vel.0 > 0.0;
        if right != self.right {
            self.right = right;
            e.set_sprite(self.sprites[right as usize].clone());
        }
    }

    fn describe(&self, _e: &Entity) -> Option<String> {
        Some(if self.fleeing { "wolf, running away".to_string() } else { "wolf, hungry".to_string() })
    }
}

/// Trader wagon: rolls to the plaza, stops to deal (state 1), then leaves the map.
struct Caravan {
    stop: f32,
    wait: f32,
}

impl Behavior for Caravan {
    fn update(&mut self, e: &mut Entity, _cx: &mut Ctx, dt: f32) {
        if e.state == 0 && e.pos.0 >= self.stop {
            e.state = 1;
            e.vel = (0.0, 0.0);
        } else if e.state == 1 {
            self.wait -= dt;
            if self.wait <= 0.0 {
                e.state = 2;
                e.vel = (6.0, 0.0);
            }
        }
    }

    fn describe(&self, _e: &Entity) -> Option<String> {
        Some("trade caravan\nbuys wood, sells grain".to_string())
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
        e.frame = season(cx.time, cx.global("daylen"));
    }
}

/// Crops grow through the warm seasons and ripen when a farmer is close.
struct Field {
    growth: f32,
}
impl Behavior for Field {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, dt: f32) {
        let s = season(cx.time, cx.global("daylen"));
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
                    Entity::new("emote", Sprite::text("harvest!", rgb(255, 240, 150)))
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

fn house_sprite(roof: Color, wall: Color) -> Sprite {
    frames_for(HOUSE, 3, roof, wall).with_pixels(vec![
        crate::art::house(roof, wall, false),
        crate::art::house(roof, wall, true),
    ])
}

const HOUSE: &str = "   /^\\   \n  /   \\  \n /_____\\ \n |[] []| \n |_[ ]_| ";
const TAVERN: &str = "  _________  \n /  TAVERN \\ \n/___________\\\n |[] [] []| \n |_[_ _]_|_|";
const BAKERY: &str = "  .-------.\n /  BAKERY\\\n/_________\\\n |[] [] []|\n |___[_]__|";
const MARKET: &str = "  ~~~~~~~~~~~\n /=/=/=/=/=/=\\\n  MARKET  |  |\n  |_______|__|";
const WELL: &str = " _____ \n |~~~| \n |___| ";

fn tree_sprite() -> Sprite {
    tree_ascii().with_pixels((0..4).map(crate::art::tree).collect())
}

fn tree_ascii() -> Sprite {
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

/// Season frames plus a final stump frame.
fn woodlot_sprite() -> Sprite {
    let tree = tree_ascii();
    let mut frames: Vec<Frame> = tree.frames.iter().cloned().collect();
    let mut stump = Frame::blank(5, 3);
    stump.set(2, 2, 'u', rgb(150, 110, 70), None);
    frames.push(stump);
    let mut pix: Vec<_> = (0..4).map(crate::art::tree).collect();
    pix.push(crate::art::stump());
    Sprite::new(frames).with_pixels(pix)
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

/// Door, house entity, chimney entity, top-left corner.
type House = ((f32, f32), Id, Id, (f32, f32));
/// Door, top-left corner, construction site entity.
type Plot = ((f32, f32), (f32, f32), Id);

pub struct Village {
    homes: Vec<(f32, f32)>,
    work_sites: Vec<(Job, (f32, f32))>,
    plaza: (f32, f32),
    taverns: Vec<(f32, f32)>,
    day: f32,
    forest: Vec<(f32, f32)>,
    season: usize,
    births_cd: f32,
    weather: Weather,
    /// Standing houses: (door, house entity, chimney entity, top-left corner).
    houses: Vec<House>,
    rects: Vec<(i32, i32, i32, i32)>,
    /// Plots waiting for lumber, then plots being built on: (door, corner, site entity).
    ruins: Vec<((f32, f32), (f32, f32))>,
    sites: Vec<Plot>,
    fire: Option<(Id, usize)>,
    wear: Vec<u8>,
    wear_cd: f32,
    paint_cd: f32,
    terrain: Arc<Mutex<Option<Sprite>>>,
    wolf_cd: f32,
    caravan: Option<Id>,
    caravan_cd: f32,
    traded: bool,
    festival_day: i64,
    show_cd: f32,
    famine_news: bool,
    pop_trend: Spark,
    trend_cd: f32,
    grid: (i32, i32),
    woods: Arc<Vec<(f32, f32)>>,
}

fn spawn_wolf(cx: &mut Ctx, x: f32, y: f32) {
    let mut right = Sprite::mono(" /\\___\n< o __\\\n  || || ", rgb(170, 175, 190));
    if let Some(c) = crate::art::animal("WOLVES", 10, 3) {
        right = right.with_pixels(vec![c.frame()]);
    }
    let sprites = [right.mirrored(), right];
    let facing = cx.rng.chance(0.5);
    let wolf = Wolf { right: facing, sprites: sprites.clone(), think: 0.0, target: None, fleeing: false };
    cx.spawn(Entity::new("wolf", sprites[facing as usize].clone()).at(x, y).z(19).with(wolf));
}

impl Default for Village {
    fn default() -> Self {
        Village {
            homes: vec![],
            work_sites: vec![],
            plaza: (0.0, 0.0),
            taverns: vec![],
            day: 0.0,
            forest: vec![],
            season: 0,
            births_cd: 0.0,
            weather: Weather::default(),
            houses: vec![],
            rects: vec![],
            ruins: vec![],
            sites: vec![],
            fire: None,
            wear: vec![],
            wear_cd: 0.0,
            paint_cd: 0.0,
            terrain: Arc::new(Mutex::new(None)),
            wolf_cd: 20.0,
            caravan: None,
            caravan_cd: 120.0,
            traded: false,
            festival_day: -1,
            show_cd: 0.0,
            famine_news: false,
            pop_trend: Spark::default(),
            trend_cd: 0.0,
            grid: (0, 0),
            woods: Arc::new(vec![]),
        }
    }
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

/// Where someone stands in life when they enter the world.
struct Life {
    age: f32,
    lifespan: Option<f32>,
    adult: Option<(Job, (f32, f32))>,
    woods: Arc<Vec<(f32, f32)>>,
}

#[allow(clippy::too_many_arguments)]
fn make_villager(
    cx: &mut Ctx,
    job: Job,
    home: (f32, f32),
    work: (f32, f32),
    social: (f32, f32),
    start: (f32, f32),
    day: f32,
    life: Life,
) {
    let (g, col) = job.glyph();
    let mut f = Frame::blank(1, 2);
    f.set(0, 0, 'o', rgb(235, 200, 160), None);
    f.set(0, 1, g, col, None);
    let body = Sprite::one(f).with_pixels(vec![crate::art::person(col, None)]);
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
        name: random_name(cx.rng),
        age: life.age,
        lifespan: life.lifespan,
        adult: life.adult,
        woods: life.woods,
        last_goal: None,
        alert_cd: cx.rng.range(0.0, 1.0),
        wolf: None,
        cause: "suddenly",
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
    let sprite = house_sprite(rgb(176, 60, 50), rgb(150, 120, 90));
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
    let life = Life { age: 30.0, lifespan: None, adult: None, woods: Arc::new(vec![]) };
    make_villager(cx, Job::Farmer, home, work, home, home, day, life);
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
    fn day_len(&self) -> f32 {
        if self.day > 0.0 { self.day } else { DAY }
    }

    fn nearest_tavern(&self, p: (f32, f32)) -> (f32, f32) {
        self.taverns
            .iter()
            .copied()
            .min_by(|a, b| {
                let d = |q: &(f32, f32)| (q.0 - p.0).powi(2) + (q.1 - p.1).powi(2) * 4.0;
                d(a).total_cmp(&d(b))
            })
            .unwrap_or(self.plaza)
    }

    fn work_for(&self, cx: &mut Ctx, job: Job, home: (f32, f32)) -> (f32, f32) {
        let sites: Vec<(f32, f32)> = self.work_sites.iter().filter(|(j, _)| *j == job).map(|(_, p)| *p).collect();
        if job == Job::Woodcutter && !self.forest.is_empty() {
            *cx.rng.pick(&self.forest)
        } else if job == Job::Child || job == Job::Guard || sites.is_empty() {
            self.nearest_tavern(home)
        } else {
            *cx.rng.pick(&sites)
        }
    }

    fn spawn_villager(&self, cx: &mut Ctx, job: Job, home: (f32, f32), at_home: bool, age: f32) {
        let workspot = self.work_for(cx, job, home);
        let tavern = self.nearest_tavern(home);
        let social = (tavern.0 + cx.rng.range(-2.0, 2.0), tavern.1);
        let start = if at_home {
            home
        } else {
            (cx.rng.range(2.0, cx.w - 3.0), cx.rng.range(2.0, cx.h - 4.0))
        };
        let adult = if job == Job::Child {
            let grown = *cx.rng.pick(&[Job::Farmer, Job::Farmer, Job::Woodcutter, Job::Baker, Job::Merchant, Job::Guard]);
            Some((grown, self.work_for(cx, grown, home)))
        } else {
            None
        };
        let life = Life { age, lifespan: Some(cx.rng.range(55.0, 85.0)), adult, woods: self.woods.clone() };
        make_villager(cx, job, home, workspot, social, start, self.day_len(), life);
    }

    /// Puts a finished house on a plot and returns its home record.
    fn raise_house(&mut self, cx: &mut Ctx, door: (f32, f32), corner: (f32, f32)) {
        let roofs = [rgb(176, 60, 50), rgb(150, 80, 40), rgb(70, 90, 140), rgb(110, 110, 60)];
        let walls = [rgb(150, 120, 90), rgb(130, 110, 100), rgb(160, 135, 95)];
        let (roof, wall) = (*cx.rng.pick(&roofs), *cx.rng.pick(&walls));
        let house = cx.spawn(Entity::new("house", house_sprite(roof, wall)).at(corner.0, corner.1).z(40).with(Lit));
        let chimney = cx.spawn(
            Entity::new("chimney", Sprite::new(vec![Frame::blank(1, 1), Frame::blank(1, 1)]))
                .at(corner.0, corner.1)
                .with(Chimney),
        );
        self.houses.push((door, house, chimney, corner));
    }

    fn ignite(&mut self, cx: &mut Ctx, i: usize) {
        let corner = self.houses[i].3;
        let flames = Sprite::anim(&[("^ ^ ^ ^\n^^^^^^^", "OYOYOYO\nrOrOrOr"), (" ^ ^ ^ \n^^^^^^^", "YOYOYOY\nOrOrOrO")]);
        let f = Entity::new("fire", flames).at(corner.0 + 1.0, corner.1 + 2.0).z(6).fps(6.0).with(Fire { hp: 16.0, burn: 50.0 });
        let id = cx.spawn(f);
        self.fire = Some((id, i));
        cx.news("FIRE! A house is burning, grab your buckets!", Some(corner));
    }

    /// Scenario-level drama: fires, new houses, wolves, caravans, festivals, famine.
    fn run_events(&mut self, cx: &mut Ctx, dt: f32) {
        let winter = self.season == 3;
        let strike = self.weather.tick(cx, dt, winter);

        // Ignition: a lightning strike near a roof, or a rare accident in a dry summer.
        if self.fire.is_none() && !self.houses.is_empty() {
            let struck = strike.and_then(|p| {
                self.houses.iter().position(|h| (h.3.0 + 4.0 - p.0).abs() < 25.0 && (h.3.1 + 2.0 - p.1).abs() < 12.0)
            });
            let accident = self.season == 1 && cx.rng.chance(0.0002);
            let idx = struck.or_else(|| accident.then(|| cx.rng.int(0, self.houses.len() as i32) as usize));
            if let Some(i) = idx {
                self.ignite(cx, i);
            }
        }
        cx.set("fire_on", self.fire.is_some() as i32 as f32);
        if let Some((id, i)) = self.fire {
            let corner = self.houses[i].3;
            cx.set("fire_x", corner.0 + 4.0);
            cx.set("fire_y", corner.1 + 5.0);
            match cx.snap.get(id).map_or(0, |f| f.state) {
                1 => {
                    cx.kill(id);
                    self.fire = None;
                    cx.news("The fire is out. The house is saved!", Some(corner));
                }
                2 => {
                    cx.kill(id);
                    self.fire = None;
                    let (door, house, chimney, corner) = self.houses.swap_remove(i);
                    cx.kill(house);
                    cx.kill(chimney);
                    let ruin = Sprite::mono("  . ,  \n /_%_\\ \n %.#%.% ", rgb(70, 66, 64));
                    cx.spawn(Entity::new("grave", ruin).at(corner.0 + 1.0, corner.1 + 2.0).z(41).lifetime(120.0));
                    self.ruins.push((door, corner));
                    cx.news("A house burned to the ground.", Some(corner));
                }
                _ => {}
            }
        }

        // Construction: rebuild ruins first, then expand when the village is overcrowded.
        let pop = cx.snap.count("villager");
        let wood = cx.global("wood");
        if wood >= HOUSE_WOOD && self.sites.len() < 3 {
            let plot = if let Some(r) = self.ruins.pop() {
                Some(r)
            } else if pop > (self.houses.len() + self.sites.len()) * 2 {
                let (sw, sh) = (9, 5);
                let mut rects = std::mem::take(&mut self.rects);
                let spot = place(cx, &mut rects, sw, sh, 2);
                self.rects = rects;
                spot.map(|(x, y)| (((x + sw / 2) as f32, (y + sh) as f32), (x as f32, y as f32)))
            } else {
                None
            };
            if let Some((door, corner)) = plot {
                cx.add("wood", -HOUSE_WOOD);
                let id = cx.spawn(Entity::new("site", site_sprite()).at(corner.0, corner.1).z(40).with(Site { progress: 0.0 }));
                self.sites.push((door, corner, id));
                cx.news("Builders break ground on a new house.", Some(corner));
            }
        }
        let mut i = 0;
        while i < self.sites.len() {
            let (door, corner, id) = self.sites[i];
            if cx.snap.get(id).is_some_and(|s| s.state == 1) {
                cx.kill(id);
                self.sites.swap_remove(i);
                self.raise_house(cx, door, corner);
                if !self.homes.contains(&door) {
                    self.homes.push(door);
                    self.spawn_villager(cx, Job::Farmer, door, true, 20.0);
                    cx.news("A new family moves into their home.", Some(corner));
                } else {
                    cx.news("A burned-out family has a roof again.", Some(corner));
                }
            } else {
                i += 1;
            }
        }

        // Winter nights bring wolves down from the woods.
        self.wolf_cd -= dt;
        if winter && cx.daylight < 0.35 && self.wolf_cd <= 0.0 && cx.snap.count("wolf") < 5 {
            self.wolf_cd = cx.rng.range(8.0, 25.0);
            let from_left = cx.rng.chance(0.5);
            let x = if from_left { -10.0 } else { cx.w };
            let y = cx.rng.range(2.0, cx.h - 6.0);
            spawn_wolf(cx, x, y);
            cx.news("Wolves are howling at the edge of the village.", Some((x.max(0.0), y)));
        }

        // Keep a few hens around; farmers raise more when wolves have been through.
        if cx.snap.count("chicken") < (cx.w / 20.0) as usize && cx.rng.chance(0.002) {
            let (x, y) = (cx.rng.range(5.0, cx.w - 5.0), cx.rng.range(3.0, cx.h - 4.0));
            pick_chicken(cx, x, y);
        }

        // Traders come in the warm months, buying lumber and selling grain for gold.
        if let Some(id) = self.caravan {
            match cx.snap.get(id) {
                None => {
                    self.caravan = None;
                    self.caravan_cd = self.day_len() * 1.5;
                }
                Some(c) if c.state == 1 && !self.traded => {
                    self.traded = true;
                    let pos = c.pos;
                    let (mut wood, mut gold, mut food) = (cx.global("wood"), cx.global("gold"), cx.global("food"));
                    let mut deals = vec![];
                    if wood > 30.0 {
                        wood -= 12.0;
                        gold += 30.0;
                        deals.push("sold lumber");
                    }
                    if food < 80.0 && gold >= 15.0 {
                        let lots = (gold / 15.0).min(4.0).floor();
                        gold -= lots * 15.0;
                        food += lots * 25.0;
                        deals.push("bought grain");
                    }
                    cx.set("wood", wood);
                    cx.set("gold", gold);
                    cx.set("food", food);
                    let what = if deals.is_empty() { "found no buyers".to_string() } else { deals.join(" and ") };
                    cx.news(format!("At the caravan, the village {what}."), Some(pos));
                }
                _ => {}
            }
        }
        self.caravan_cd -= dt;
        if self.caravan.is_none() && self.caravan_cd <= 0.0 && !winter {
            let wagon = Sprite::mono("  ____\n /____\\__\n  o  o  ~>", rgb(200, 170, 120));
            let y = self.plaza.1 + 6.0;
            let id = cx.spawn(
                Entity::new("caravan", wagon)
                    .at(-12.0, y)
                    .vel(6.0, 0.0)
                    .z(21)
                    .die_offscreen()
                    .with(Caravan { stop: self.plaza.0 - 14.0, wait: 18.0 }),
            );
            self.caravan = Some(id);
            self.traded = false;
            cx.news("A trade caravan is on the road to the plaza.", None);
        }

        // The harvest festival takes over the plaza for the middle day of autumn.
        let day = (cx.time / self.day_len() as f64) as i64;
        let in_festival = self.season == 2 && day % (DAYS_PER_SEASON as i64 * 4) == 7 && cx.global("food") > 15.0;
        cx.set("festival", in_festival as i32 as f32);
        if in_festival {
            if self.festival_day != day {
                self.festival_day = day;
                cx.news("The harvest festival begins on the plaza!", Some(self.plaza));
            }
            self.show_cd -= dt;
            if self.show_cd <= 0.0 && cx.daylight < 0.55 {
                self.show_cd = cx.rng.range(0.5, 1.6);
                let (x, y) = (
                    self.plaza.0 + cx.rng.range(-35.0, 35.0),
                    (self.plaza.1 - cx.rng.range(8.0, 20.0)).max(3.0),
                );
                firework(cx, x, y);
            }
        }
    }

    /// Accumulates footfall and periodically repaints the path layer.
    fn wear_paths(&mut self, cx: &mut Ctx, dt: f32) {
        let (gw, gh) = self.grid;
        self.wear_cd -= dt;
        if self.wear_cd <= 0.0 {
            self.wear_cd = 0.5;
            for v in cx.snap.of_kind("villager").filter(|v| v.vel != (0.0, 0.0)) {
                let (x, y) = (v.pos.0.round() as i32, (v.pos.1 + 1.0).round() as i32);
                if x >= 0 && y >= 0 && x < gw && y < gh {
                    let c = &mut self.wear[(y * gw + x) as usize];
                    *c = c.saturating_add(8);
                }
            }
        }
        self.paint_cd -= dt;
        if self.paint_cd <= 0.0 {
            self.paint_cd = 8.0;
            let mut f = Frame::blank(gw as usize, gh as usize);
            for y in 0..gh {
                for x in 0..gw {
                    let c = &mut self.wear[(y * gw + x) as usize];
                    let w = *c;
                    *c = c.saturating_sub(1);
                    if w >= 24 {
                        let k = (w as f32 / 200.0).min(1.0);
                        let col = rgb(
                            (116.0 + 34.0 * k) as u8,
                            (98.0 + 26.0 * k) as u8,
                            (62.0 + 26.0 * k) as u8,
                        );
                        f.set(x as usize, y as usize, ' ', col, Some(col));
                    }
                }
            }
            *self.terrain.lock().unwrap() = Some(Sprite::one(f));
        }
    }
}

impl Scenario for Village {
    fn name(&self) -> &'static str {
        "village"
    }

    fn tools(&self) -> &'static [&'static str] {
        &["wolf", "set a house on fire", "trade caravan", "hen", "thunderstorm"]
    }

    fn place(&mut self, cx: &mut Ctx, tool: usize, x: f32, y: f32) {
        match tool {
            0 => spawn_wolf(cx, x, y),
            1 => {
                let d = |h: &House| (h.3.0 + 4.0 - x).abs() + (h.3.1 + 2.0 - y).abs() * 2.0;
                let near = (0..self.houses.len()).min_by(|&a, &b| d(&self.houses[a]).total_cmp(&d(&self.houses[b])));
                if let (None, Some(i)) = (self.fire, near) {
                    self.ignite(cx, i);
                }
            }
            2 => self.caravan_cd = 0.0,
            3 => pick_chicken(cx, x, y),
            _ => self.weather.force_storm(cx),
        }
    }

    fn preferred_size(&self) -> Option<(i32, i32)> {
        Some((320, 110))
    }

    fn background(&self) -> Color {
        GROUND[self.season]
    }

    fn day_length(&self) -> Option<f32> {
        Some(self.day_len())
    }

    fn hud(&self, g: &Globals, _daylight: f32) -> String {
        let v = |k| g.get(k).copied().unwrap_or(0.0) as i32;
        format!(
            "{}{} | pop {} {} homes {} food {} wood {} gold {} | died {}",
            SEASON_NAMES[self.season],
            if v("famine") > 0 { " FAMINE" } else { "" },
            v("pop"),
            self.pop_trend.render(),
            self.houses.len(),
            v("food"),
            v("wood"),
            v("gold"),
            v("deaths"),
        )
    }

    fn setup(&mut self, cx: &mut Ctx) {
        let (w, h) = (cx.w as i32, cx.h as i32);
        let mut rects: Vec<(i32, i32, i32, i32)> = vec![];
        self.plaza = (cx.w / 2.0, cx.h / 2.0);
        rects.push((w / 2 - 8, h / 2 - 4, 16, 8));
        self.day = DAY * (cx.w / 160.0).clamp(1.0, 3.0);
        cx.set("daylen", self.day);
        cx.set("food", 40.0);
        cx.set("wood", 10.0);
        cx.set("gold", 15.0);
        self.grid = (w, h);
        self.wear = vec![0; (w * h) as usize];
        cx.spawn(
            Entity::new("terrain", Sprite::one(Frame::blank(1, 1)))
                .z(95)
                .with(Terrain { next: self.terrain.clone() }),
        );

        // Well at the heart of the plaza.
        cx.spawn(
            Entity::new("well", Sprite::mono(WELL, rgb(150, 150, 160)))
                .at(self.plaza.0 - 3.0, self.plaza.1 - 2.0)
                .z(30),
        );

        // Taverns, bakeries and markets: one set near the plaza, more spread across big maps.
        let specials: [(&str, Job, Color, Color, i32); 3] = [
            (TAVERN, Job::Child, rgb(120, 60, 40), rgb(110, 80, 60), 0),
            (BAKERY, Job::Baker, rgb(170, 100, 50), rgb(150, 120, 80), 1),
            (MARKET, Job::Merchant, rgb(190, 60, 60), rgb(130, 100, 70), 2),
        ];
        for _ in 0..(1 + w / 120) {
            for (art, job, roof, wall, i) in specials {
                let sprite = frames_for(art, 2, roof, wall);
                let (sw, sh) = (sprite.w() as i32, sprite.h() as i32);
                let sprite = sprite.with_pixels(vec![
                    crate::art::shop(sw, sh * 2, roof, wall, false),
                    crate::art::shop(sw, sh * 2, roof, wall, true),
                ]);
                if let Some((x, y)) = place(cx, &mut rects, sw, sh, 3) {
                    cx.spawn(Entity::new("building", sprite).at(x as f32, y as f32).z(40).with(Lit));
                    let door = ((x + sw / 2) as f32, (y + sh) as f32);
                    if i == 0 {
                        self.taverns.push(door);
                    } else {
                        self.work_sites.push((job, door));
                    }
                }
            }
        }
        if self.taverns.is_empty() {
            self.taverns.push(self.plaza);
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
        let n_homes = ((w * h) / 380).clamp(4, 120);
        for _ in 0..n_homes {
            let (sw, sh) = (9, 5);
            if let Some((x, y)) = place(cx, &mut rects, sw, sh, 2) {
                let (door, corner) = (((x + sw / 2) as f32, (y + sh) as f32), (x as f32, y as f32));
                self.raise_house(cx, door, corner);
                self.homes.push(door);
            }
        }

        // Forest trees, denser away from the plaza.
        let trees = tree_sprite();
        let lot = woodlot_sprite();
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
                // The trees closest to town are the ones that get logged.
                if self.forest.len() < 60 && far < 0.45 {
                    let hp = cx.rng.range(8.0, 14.0);
                    cx.spawn(Entity::new("woodlot", lot.clone()).at(x as f32, y as f32).z(45).with(Woodlot { hp, regrow: 0.0 }));
                    self.forest.push(((x + 2) as f32, (y + 3) as f32));
                } else {
                    cx.spawn(Entity::new("tree", trees.clone()).at(x as f32, y as f32).z(45).with(Tree));
                }
            }
        }

        self.woods = Arc::new(self.forest.clone());
        self.rects = rects;

        // Population: every home gets a family.
        let homes = self.homes.clone();
        for (i, home) in homes.iter().enumerate() {
            let job = [Job::Farmer, Job::Woodcutter, Job::Farmer, Job::Baker, Job::Merchant, Job::Guard]
                [i % 6];
            let age = cx.rng.range(18.0, 52.0);
            self.spawn_villager(cx, job, *home, false, age);
            if cx.rng.chance(0.5) {
                let age = cx.rng.range(1.0, 14.0);
                self.spawn_villager(cx, Job::Child, *home, false, age);
            }
        }

        for _ in 0..(3 + w / 20) {
            let (x, y) = (cx.rng.range(5.0, cx.w - 5.0), cx.rng.range(3.0, cx.h - 4.0));
            pick_chicken(cx, x, y);
        }
    }

    fn tick(&mut self, cx: &mut Ctx, dt: f32) {
        self.season = season(cx.time, self.day);
        cx.set("season", self.season as f32);

        let pop = cx.snap.count("villager") as f32;
        cx.set("pop", pop);
        self.trend_cd -= dt;
        if self.trend_cd <= 0.0 {
            self.trend_cd = 30.0;
            self.pop_trend.push(pop);
        }

        // Winters are hungry. Food going to zero starts a famine that kills until grain returns.
        let appetite = if self.season == 3 { 0.009 } else { 0.005 };
        cx.add("food", -pop * appetite * dt);
        let hungry = cx.global("food") <= 0.0;
        if hungry {
            cx.set("food", 0.0);
        }
        cx.set("famine", hungry as i32 as f32);
        if hungry != self.famine_news {
            self.famine_news = hungry;
            cx.news(if hungry { "FAMINE! The granaries are empty." } else { "Grain is back. The famine is over." }, None);
        }

        self.births_cd -= dt;
        let room = pop < self.houses.len() as f32 * 4.0;
        if self.births_cd <= 0.0 && cx.global("food") > 30.0 && room && !self.homes.is_empty() {
            self.births_cd = 25.0;
            cx.add("food", -10.0);
            let home = *cx.rng.pick(&self.homes);
            self.spawn_villager(cx, Job::Child, home, true, 0.0);
            cx.add("born", 1.0);
        }

        self.run_events(cx, dt);
        self.wear_paths(cx, dt);
    }
}
