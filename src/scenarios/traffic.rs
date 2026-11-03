use crate::engine::sprite::std_palette;
use crate::engine::*;
use ratatui::style::Color;

const V_SPACING: i32 = 28;
const H_SPACING: i32 = 12;
const V_ROAD_W: i32 = 9;
const H_ROAD_H: i32 = 5;
const X0: i32 = 6;
const Y0: i32 = 3;

const CITY_DAY: f32 = 120.0;
const ASPHALT: Color = Color::Rgb(58, 58, 64);
const SIDEWALK: Color = Color::Rgb(118, 118, 124);
const GROUND: Color = Color::Rgb(44, 66, 46);

// Directions: 0 east, 1 south, 2 west, 3 north.
const DX: [f32; 4] = [1.0, 0.0, -1.0, 0.0];
const DY: [f32; 4] = [0.0, 1.0, 0.0, -1.0];

const BODY_COLORS: &[char] = &['R', 'Y', 'B', 'C', 'W', 'O', 'G', 'M', 'K'];

fn car_art(dir: usize, kind: usize) -> (String, String) {
    match dir {
        0 | 2 => {
            let len = [4, 6, 9][kind];
            let body = "=".repeat(len - 2);
            let art = format!("[{body}>");
            let mask = format!("k{}k", "1".repeat(len - 2));
            if dir == 0 { (art, mask) } else { (flip_row(&art), mask) }
        }
        _ => {
            let rows = [2, 3, 4][kind];
            let mut art = String::new();
            let mut mask = String::new();
            for r in 0..rows {
                let (a, m) = if dir == 1 {
                    if r == rows - 1 { ("'v'", "kYk") } else { ("[=]", "k1k") }
                } else if r == 0 {
                    ("'^'", "kYk")
                } else {
                    ("[=]", "k1k")
                };
                art.push_str(a);
                mask.push_str(m);
                if r < rows - 1 {
                    art.push('\n');
                    mask.push('\n');
                }
            }
            (art, mask)
        }
    }
}

fn flip_row(s: &str) -> String {
    Frame::parse(s, None, Color::White, &[Color::White; 9]).mirrored().ch.iter().collect()
}

fn car_sprite(dir: usize, kind: usize, color: Color) -> Sprite {
    let (art, mask) = car_art(dir, kind);
    let mut digits = [Color::White; 9];
    digits[0] = color;
    let sprite = Sprite::art_d(&art, &mask, &digits);
    let (w, h) = (sprite.w() as i32, sprite.h() as i32);
    sprite.with_pixels(vec![crate::art::vehicle(dir, w, h, color, None)])
}

/// Truck-sized body with a flashing red/blue siren.
fn emergency_sprite(dir: usize) -> Sprite {
    let (art, mask) = car_art(dir, 1);
    let mut digits = [Color::White; 9];
    digits[0] = Color::Rgb(245, 245, 245);
    let base = Frame::parse(&art, Some(&mask), Color::White, &digits);
    let (sx, sy) = if dir % 2 == 0 { (2, 0) } else { (1, 1) };
    let frame = |c: Color| {
        let mut f = base.clone();
        f.set(sx, sy, '@', c, None);
        f
    };
    let (w, h) = (base.w as i32, base.h as i32);
    let white = Color::Rgb(246, 246, 246);
    let (red, blue) = (Color::Rgb(255, 50, 50), Color::Rgb(70, 110, 255));
    Sprite::new(vec![frame(red), frame(blue)]).with_pixels(vec![
        crate::art::vehicle(dir, w, h, white, Some(red)),
        crate::art::vehicle(dir, w, h, white, Some(blue)),
    ])
}

struct Car {
    dir: usize,
    kind: usize,
    color: Color,
    emergency: bool,
    v: f32,
    vmax: f32,
    sig: Option<Id>,
    turn: bool,
    stuck: f32,
    box_wait: f32,
}

impl Car {
    fn axis_scale(dir: usize) -> f32 {
        // Terminal cells are about twice as tall as wide.
        if dir % 2 == 1 { 0.5 } else { 1.0 }
    }

    fn gap(dir: usize, me: (i32, i32, i32, i32), o: (i32, i32, i32, i32)) -> i32 {
        match dir {
            0 => o.0 - (me.0 + me.2),
            1 => o.1 - (me.1 + me.3),
            2 => me.0 - (o.0 + o.2),
            _ => me.1 - (o.1 + o.3),
        }
    }

    fn change_dir(&mut self, e: &mut Entity, dir: usize, x: f32, y: f32) {
        self.dir = dir;
        e.set_sprite(if self.emergency {
            emergency_sprite(dir)
        } else {
            car_sprite(dir, self.kind, self.color)
        });
        e.pos = (x, y);
        self.v *= 0.6;
        self.turn = false;
    }
}

impl Behavior for Car {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, dt: f32) {
        let mut boxed = false;
        let d = self.dir;
        let scale = Car::axis_scale(d);
        let (a_acc, a_brake) = (6.0 * scale, 9.0 * scale);
        let me = e.rect();
        let my_len = if d % 2 == 0 { me.2 } else { me.3 };
        let stop_dist = self.v * self.v / (2.0 * a_brake);
        let look = ((stop_dist + 8.0) as i32).max(24);
        let probe = match d {
            0 => (me.0 + me.2, me.1, look, me.3),
            1 => (me.0, me.1 + me.3, me.2, look),
            2 => (me.0 - look, me.1, look, me.3),
            _ => (me.0, me.1 - look, me.2, look),
        };

        let ahead = cx.snap.query(probe.0, probe.1, probe.2, probe.3);
        let mut allowed = f32::MAX;
        let mut stationary_beyond: Vec<i32> = vec![];
        // A car wedged in a junction for too long phases through crossing traffic to break gridlock.
        let ghost = self.stuck > 5.0;
        for o in ahead.iter().filter(|o| o.kind == "car" && o.id != e.id) {
            if ghost && (o.state % 10) as usize != d + 1 {
                continue;
            }
            let g = Car::gap(d, me, o.rect);
            if g >= 0 {
                allowed = allowed.min((g - 1) as f32);
                let speed_along = o.vel.0 * DX[d] + o.vel.1 * DY[d];
                if speed_along.abs() < 2.0 {
                    stationary_beyond.push(g);
                }
            }
        }
        for o in ahead.iter().filter(|o| o.kind == "ped") {
            let g = Car::gap(d, me, o.rect);
            if g >= 0 {
                allowed = allowed.min((g - 1) as f32);
            }
        }
        for s in ahead.iter().filter(|o| o.kind == "signal") {
            let g = Car::gap(d, me, s.rect);
            if g < 0 {
                continue;
            }
            let phase = s.state;
            let (green, yellow) = if d % 2 == 1 { (0, 1) } else { (3, 4) };
            let can_run_yellow = phase == yellow && (g as f32) < stop_dist;
            let must_stop = phase != green && !can_run_yellow && !self.emergency;
            let inter_len = if d % 2 == 0 { s.rect.2 } else { s.rect.3 };
            let box_blocked =
                stationary_beyond.iter().any(|&sg| sg >= g && sg < g + inter_len + my_len + 2);
            let box_blocked = box_blocked && self.box_wait < 8.0;
            if must_stop || box_blocked {
                boxed |= !must_stop;
                allowed = allowed.min((g - 1) as f32);
            }
        }

        let target = if allowed <= 0.0 {
            0.0
        } else {
            self.vmax.min((2.0 * a_brake * allowed).sqrt())
        };
        self.box_wait = if boxed && self.v < 0.1 { self.box_wait + dt } else if boxed { self.box_wait } else { 0.0 };
        let dv = (target - self.v).clamp(-a_brake * dt * 1.5, a_acc * dt);
        self.v = (self.v + dv).max(0.0);
        e.vel = (DX[d] * self.v, DY[d] * self.v);
        e.data = self.v;
        e.state = 1 + d as i32 + if self.emergency { 10 } else { 0 };

        // Right turns at intersections.
        let inside = cx.snap.query(me.0, me.1, me.2, me.3);
        let in_box = inside.iter().any(|o| o.kind == "signal");
        self.stuck = if in_box && self.v < 0.1 { self.stuck + dt } else { 0.0 };
        if let Some(s) = inside.iter().find(|o| o.kind == "signal") {
            if self.sig != Some(s.id) {
                self.sig = Some(s.id);
                self.turn = cx.rng.chance(0.28);
            }
            if self.turn {
                let (vx, hy) = (s.pos.0, s.pos.1);
                let (x, y, w, h) = (e.pos.0, e.pos.1, me.2 as f32, me.3 as f32);
                let target = match d {
                    0 if x + w >= vx + 4.0 => Some((1, vx + 1.0, y)),
                    1 if y + h >= hy + 2.0 => Some((2, vx, hy + 1.0)),
                    2 if x <= vx + 5.0 => Some((3, vx + 5.0, hy)),
                    3 if y <= hy + 2.0 => Some((0, vx + 5.0, hy + 3.0)),
                    _ => None,
                };
                if let Some((nd, nx, ny)) = target {
                    // Only merge into the cross lane when its entry is clear; otherwise go straight.
                    let (nw, nh) = if nd % 2 == 0 { (me.2.max(4), 1) } else { (3, me.3.max(2)) };
                    let clear = !cx
                        .snap
                        .query(nx as i32 - 1, ny as i32 - 1, nw + 2, nh + 2)
                        .iter()
                        .any(|o| o.kind == "car" && o.id != e.id);
                    if clear {
                        self.change_dir(e, nd, nx, ny);
                    } else {
                        self.turn = false;
                    }
                }
            }
        }
    }

    fn on_death(&mut self, _e: &Entity, cx: &mut Ctx) {
        cx.add("trips", 1.0);
    }
}

/// Cycles one intersection through north-south green, east-west green, with all-red gaps.
/// Approaching emergency vehicles hold their axis green.
struct Signal {
    t: f32,
    /// Seconds spent preempting, and the lockout before the next preemption is allowed.
    held: f32,
    lockout: f32,
}

const PHASES: [(i32, f32); 6] = [(0, 9.0), (1, 2.0), (2, 1.5), (3, 9.0), (4, 2.0), (5, 1.5)];
const CYCLE: f32 = 25.0;

fn phase_start(p: i32) -> f32 {
    PHASES.iter().take(p as usize).map(|x| x.1).sum()
}

impl Behavior for Signal {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, dt: f32) {
        self.t += dt;
        self.t %= CYCLE;

        let r = e.rect();
        let near = cx.snap.query(r.0 - 22, r.1 - 12, r.2 + 44, r.3 + 24);
        let wanted = near.iter().find_map(|o| {
            if o.kind != "car" || o.state < 11 {
                return None;
            }
            let d = (o.state - 11) as usize;
            let approaching = match d {
                0 => o.rect.0 < r.0 + r.2 && o.rect.1 >= r.1,
                2 => o.rect.0 + o.rect.2 > r.0 && o.rect.1 >= r.1,
                1 => o.rect.1 < r.1 + r.3,
                _ => o.rect.1 + o.rect.3 > r.1,
            };
            approaching.then_some(d % 2)
        });
        let phase = PHASES
            .iter()
            .scan(0.0, |acc, p| {
                *acc += p.1;
                Some((*acc, p.0))
            })
            .find(|(end, _)| self.t < *end)
            .map_or(0, |p| p.1);
        self.lockout = (self.lockout - dt).max(0.0);
        if wanted.is_none() {
            self.held = 0.0;
        }
        let wanted = if self.lockout > 0.0 { None } else { wanted };
        if let Some(axis) = wanted {
            self.held += dt;
            if self.held > 10.0 {
                self.held = 0.0;
                self.lockout = 25.0;
            }
            let (green, other_green, other_yellow) =
                if axis == 1 { (0, 3, 4) } else { (3, 0, 1) };
            if phase == green {
                self.t = self.t.min(phase_start(green) + 6.0);
            } else if phase == other_green {
                self.t = phase_start(other_yellow);
            }
        }
        e.state = phase;
    }
}

struct Ped {
    dir: usize,
    speed: f32,
    waited: f32,
    phase: f32,
}

impl Behavior for Ped {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, dt: f32) {
        let d = self.dir;
        let me = e.rect();
        let probe = match d {
            0 => (me.0 + 1, me.1, 2, 1),
            2 => (me.0 - 2, me.1, 2, 1),
            1 => (me.0, me.1 + me.3, 1, 2),
            _ => (me.0, me.1 - 2, 1, 2),
        };
        let mut wait = false;
        for s in cx.snap.query(probe.0, probe.1, probe.2, probe.3) {
            if s.kind != "signal" {
                continue;
            }
            let inside = crate::engine::world::overlaps(me, s.rect);
            // Crossing a road is safe while that road's own traffic is stopped.
            let safe = if d % 2 == 0 { s.state == 3 } else { s.state == 0 };
            if !inside && !safe {
                wait = true;
            }
        }
        let scale = if d % 2 == 1 { 0.5 } else { 1.0 };
        let v = if wait { 0.0 } else { self.speed * scale };
        self.waited = if wait { self.waited + dt } else { 0.0 };
        let bob = (e.age * 6.0 + self.phase).sin() * 0.2;
        e.vel = (DX[d] * v + if d % 2 == 1 { bob } else { 0.0 }, DY[d] * v);
        e.data = v;
    }
}

/// Lamp showing one axis of a signal: frame 0 green, 1 yellow, 2 red.
struct Lamp {
    signal: Id,
    vertical: bool,
}

impl Behavior for Lamp {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, _dt: f32) {
        let Some(s) = cx.snap.get(self.signal) else { return };
        let (g, y) = if self.vertical { (0, 1) } else { (3, 4) };
        e.frame = if s.state == g {
            0
        } else if s.state == y {
            1
        } else {
            2
        };
    }
}

fn lamp_sprite() -> Sprite {
    let mk = |c: Color| {
        let mut f = Frame::blank(1, 1);
        f.set(0, 0, '@', c, None);
        f
    };
    Sprite::new(vec![
        mk(Color::Rgb(60, 255, 90)),
        mk(Color::Rgb(255, 220, 60)),
        mk(Color::Rgb(255, 60, 60)),
    ])
}

/// Day frame has dark windows, night frame lit ones.
struct Building;
impl Behavior for Building {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, _dt: f32) {
        e.frame = if cx.daylight < 0.45 { 1 } else { 0 };
    }
}

fn building(w: usize, h: usize, rng: &mut Rng) -> Sprite {
    let walls = [
        Color::Rgb(120, 100, 95),
        Color::Rgb(95, 105, 125),
        Color::Rgb(125, 120, 100),
        Color::Rgb(100, 110, 100),
    ];
    let wall = *rng.pick(&walls);
    let trim = Color::Rgb(210, 210, 215);
    let dark_win = Color::Rgb(70, 90, 130);
    let lit_win = Color::Rgb(255, 225, 110);
    let lit: Vec<bool> = (0..w * h).map(|_| rng.chance(0.55)).collect();
    let make = |night: bool| {
        let mut f = Frame::filled(w, h, ' ', trim, Some(wall));
        for x in 0..w {
            f.set(x, 0, '=', trim, Some(wall));
        }
        for y in 1..h {
            f.set(0, y, '|', trim, Some(wall));
            f.set(w - 1, y, '|', trim, Some(wall));
            for x in 1..w - 1 {
                if y % 2 == 1 && x % 3 == 2 {
                    let on = night && lit[y * w + x];
                    f.set(x, y, '#', if on { lit_win } else { dark_win }, Some(wall));
                }
            }
        }
        f
    };
    Sprite::new(vec![make(false), make(true)])
}

fn tree(rng: &mut Rng) -> Sprite {
    let c = if rng.chance(0.5) { 'G' } else { 'g' };
    let _ = std_palette(c);
    Sprite::mono(if rng.chance(0.5) { "&" } else { "Y" }, std_palette(c).unwrap())
}

pub struct Traffic {
    /// When set, city blocks become small dioramas instead of plain buildings.
    city: bool,
    vroads: Vec<i32>,
    hroads: Vec<i32>,
    spawn_acc: f32,
}

impl Default for Traffic {
    fn default() -> Self {
        Traffic { city: false, vroads: vec![], hroads: vec![], spawn_acc: 0.0 }
    }
}

impl Traffic {
    pub fn city() -> Traffic {
        Traffic { city: true, ..Traffic::default() }
    }

    /// Fills a block with a themed diorama; returns false when the block is too small.
    fn diorama(&self, cx: &mut Ctx, rect: (i32, i32, i32, i32)) -> bool {
        let (_, _, bw, bh) = rect;
        if bw < 16 || bh < 6 {
            return false;
        }
        let roll = cx.rng.f();
        if roll < 0.32 {
            let fits: Vec<usize> = (0..super::zoo::species_count())
                .filter(|&i| super::zoo::species_height(i) <= bh - 3)
                .collect();
            let si = *cx.rng.pick(&fits);
            let sign_bottom = cx.rng.chance(0.5);
            super::zoo::enclosure(cx, rect, si, sign_bottom);
        } else if roll < 0.50 {
            super::aquarium::tank(cx, rect);
        } else if roll < 0.72 {
            super::village::yard(cx, rect, CITY_DAY);
        } else if roll < 0.86 {
            super::village::park(cx, rect);
        } else {
            return false;
        }
        true
    }

    fn lane_capacity(&self, w: i32, h: i32) -> usize {
        let lane_len = 2 * w * self.hroads.len() as i32 + 2 * h * self.vroads.len() as i32;
        (lane_len / 18) as usize
    }

    fn initial_cap(&self, w: i32, h: i32) -> i32 {
        self.lane_capacity(w, h) as i32 / 2
    }

    fn road_background(&self, w: i32, h: i32) -> Frame {
        let mut f = Frame::filled(w as usize, h as usize, ' ', Color::White, Some(GROUND));
        let line = Color::Rgb(220, 190, 60);
        for y in 0..h {
            for x in 0..w {
                let hy = self.hroads.iter().find(|&&r| y >= r && y < r + H_ROAD_H).map(|r| y - r);
                let vx = self.vroads.iter().find(|&&c| x >= c && x < c + V_ROAD_W).map(|c| x - c);
                let (ux, uy) = (x as usize, y as usize);
                match (hy, vx) {
                    (Some(_), Some(_)) => f.set(ux, uy, ' ', line, Some(ASPHALT)),
                    (Some(r), None) => match r {
                        0 | 4 => f.set(ux, uy, ' ', line, Some(SIDEWALK)),
                        2 => f.set(ux, uy, if x % 4 < 2 { '-' } else { ' ' }, line, Some(ASPHALT)),
                        _ => f.set(ux, uy, ' ', line, Some(ASPHALT)),
                    },
                    (None, Some(c)) => match c {
                        0 | 8 => f.set(ux, uy, ' ', line, Some(SIDEWALK)),
                        4 => f.set(ux, uy, if y % 4 < 2 { '|' } else { ' ' }, line, Some(ASPHALT)),
                        _ => f.set(ux, uy, ' ', line, Some(ASPHALT)),
                    },
                    _ => {}
                }
            }
        }
        f
    }

    /// Edge spawn points: (direction, x, y) for every lane entering the map.
    fn lanes(&self, w: i32, h: i32) -> Vec<(usize, f32, f32)> {
        let mut out = vec![];
        for &r in &self.hroads {
            out.push((0, -1.0, (r + 3) as f32));
            out.push((2, w as f32, (r + 1) as f32));
        }
        for &c in &self.vroads {
            out.push((1, (c + 1) as f32, -1.0));
            out.push((3, (c + 5) as f32, h as f32));
        }
        out
    }

    fn spawn_car(&self, cx: &mut Ctx, dir: usize, x: f32, y: f32, moving: bool) {
        let emergency = moving && cx.rng.chance(0.012);
        let kind = match cx.rng.f() {
            f if f < 0.78 => 0,
            f if f < 0.93 => 1,
            _ => 2,
        };
        let color = std_palette(*cx.rng.pick(BODY_COLORS)).unwrap();
        let kind = if emergency { 1 } else { kind };
        let sprite = if emergency { emergency_sprite(dir) } else { car_sprite(dir, kind, color) };
        let (w, h) = (sprite.w() as f32, sprite.h() as f32);
        let (x, y) = match dir {
            0 => (x - w, y),
            2 => (x, y),
            1 => (x, y - h),
            _ => (x, y),
        };
        let probe = (x as i32 - 2, y as i32 - 2, w as i32 + 4, h as i32 + 4);
        if cx.snap.query(probe.0, probe.1, probe.2, probe.3).iter().any(|o| o.kind == "car") {
            return;
        }
        let mut vmax = cx.rng.range(7.0, 13.0) * Car::axis_scale(dir);
        if emergency {
            vmax *= 1.4;
        }
        let car = Car { dir, kind, color, emergency, v: if moving { vmax } else { 0.0 }, vmax, sig: None, turn: false, stuck: 0.0, box_wait: 0.0 };
        let mut e = Entity::new("car", sprite).at(x, y).z(10).die_offscreen().with(car);
        if emergency {
            e.anim_fps = 4.0;
        }
        cx.spawn(e);
    }
}

impl Traffic {
    /// Pedestrians walk the full length of a sidewalk, crossing at signals.
    fn spawn_ped(&self, cx: &mut Ctx) {
        let (w, h) = (cx.w as i32, cx.h as i32);
        let horizontal = cx.rng.chance(0.6);
        let (dir, x, y) = if horizontal && !self.hroads.is_empty() {
            let r = *cx.rng.pick(&self.hroads);
            let row = r + if cx.rng.chance(0.5) { 0 } else { 4 };
            if cx.rng.chance(0.5) { (0, -1, row) } else { (2, w, row) }
        } else if !self.vroads.is_empty() {
            let c = *cx.rng.pick(&self.vroads);
            let col = c + if cx.rng.chance(0.5) { 0 } else { 8 };
            if cx.rng.chance(0.5) { (1, col, -2) } else { (3, col, h) }
        } else {
            return;
        };
        let probe = (x - 2, y - 2, 5, 5);
        if cx.snap.query(probe.0, probe.1, probe.2, probe.3).iter().any(|o| o.kind == "ped") {
            return;
        }
        let shirt = std_palette(*cx.rng.pick(&['R', 'B', 'G', 'Y', 'M', 'C', 'O', 'p', 'W'])).unwrap();
        let mut f = Frame::blank(1, 1);
        f.set(0, 0, 'i', shirt, None);
        let speed = cx.rng.range(2.0, 3.5);
        let phase = cx.rng.range(0.0, 6.0);
        cx.spawn(
            Entity::new("ped", Sprite::one(f).with_pixels(vec![crate::art::walker(shirt)]))
                .at(x as f32, y as f32)
                .z(12)
                .die_offscreen()
                .with(Ped { dir, speed, waited: 0.0, phase }),
        );
    }
}

impl Scenario for Traffic {
    fn name(&self) -> &'static str {
        if self.city { "city" } else { "traffic" }
    }

    fn preferred_size(&self) -> Option<(i32, i32)> {
        Some(if self.city { (520, 160) } else { (400, 130) })
    }

    fn background(&self) -> Color {
        GROUND
    }

    fn day_length(&self) -> Option<f32> {
        Some(120.0)
    }

    fn hud(&self, g: &Globals, daylight: f32) -> String {
        let t = |k| g.get(k).copied().unwrap_or(0.0) as i32;
        format!(
            "cars {} (waiting {}) walkers {} animals {} trips {} | {}",
            t("cars"),
            t("waiting"),
            t("walkers"),
            t("animals"),
            t("trips"),
            if daylight > 0.45 { "day" } else { "night" }
        )
    }

    fn setup(&mut self, cx: &mut Ctx) {
        let (w, h) = (cx.w as i32, cx.h as i32);
        self.vroads = (0..).map(|i| X0 + i * V_SPACING).take_while(|&x| x < w).collect();
        self.hroads = (0..).map(|i| Y0 + i * H_SPACING).take_while(|&y| y < h).collect();

        let bg = self.road_background(w, h);
        cx.spawn(Entity::new("road", Sprite::one(bg)).z(100));

        // City blocks between roads.
        let mut cols: Vec<(i32, i32)> = vec![];
        let mut prev = 0;
        for &c in &self.vroads {
            cols.push((prev, c));
            prev = c + V_ROAD_W;
        }
        cols.push((prev, w));
        let mut rows: Vec<(i32, i32)> = vec![];
        let mut prev = 0;
        for &r in &self.hroads {
            rows.push((prev, r));
            prev = r + H_ROAD_H;
        }
        rows.push((prev, h));

        for &(x0, x1) in &cols {
            for &(y0, y1) in &rows {
                let (bw, bh) = (x1 - x0, y1 - y0);
                if bw < 5 || bh < 3 {
                    continue;
                }
                if self.city && self.diorama(cx, (x0, y0, bw, bh)) {
                    continue;
                }
                if cx.rng.chance(0.18) {
                    for _ in 0..(bw * bh / 6) {
                        let tx = cx.rng.int(x0, x1) as f32;
                        let ty = cx.rng.int(y0, y1) as f32;
                        let t = tree(cx.rng);
                        cx.spawn(Entity::new("tree", t).at(tx, ty).z(60));
                    }
                    continue;
                }
                let mut x = x0 + 1;
                while x < x1 - 4 {
                    let w = (cx.rng.int(8, 17)).min(x1 - 1 - x).max(5);
                    let hh = cx.rng.int(bh / 2 + 1, bh + 1).clamp(3, bh);
                    let s = building(w as usize, hh as usize, cx.rng);
                    cx.spawn(
                        Entity::new("building", s)
                            .at(x as f32, (y1 - hh) as f32)
                            .z(50)
                            .with(Building),
                    );
                    x += w + 1;
                }
            }
        }

        // Signals and their lamps.
        for &c in &self.vroads {
            for &r in &self.hroads {
                let (cf, rf) = (c as f32, r as f32);
                let offset = cx.rng.range(0.0, 23.0);
                let sig = cx.spawn(
                    Entity::new("signal", Sprite::one(Frame::blank(V_ROAD_W as usize, H_ROAD_H as usize)))
                        .at(cf, rf)
                        .z(20)
                        .with(Signal { t: offset, held: 0.0, lockout: 0.0 }),
                );
                // Lamp per approach, on the sidewalk before the stop line.
                let lamps = [
                    (cf - 1.0, rf + 4.0, false),
                    (cf + 9.0, rf, false),
                    (cf, rf - 1.0, true),
                    (cf + 8.0, rf + 5.0, true),
                ];
                for (lx, ly, vertical) in lamps {
                    cx.spawn(
                        Entity::new("lamp", lamp_sprite())
                            .at(lx, ly)
                            .z(15)
                            .with(Lamp { signal: sig, vertical }),
                    );
                }
            }
        }

        // Pre-populate roads with moving traffic.
        for _ in 0..((cx.w * cx.h / 60.0) as i32).min(self.initial_cap(w, h)) {
            let lanes = self.lanes(w, h);
            let &(dir, lx, ly) = cx.rng.pick(&lanes);
            let (x, y) = match dir {
                0 | 2 => (cx.rng.range(0.0, cx.w), ly),
                _ => (lx, cx.rng.range(0.0, cx.h)),
            };
            self.spawn_car(cx, dir, x + if dir == 0 { 4.0 } else { 0.0 }, y + if dir == 1 { 2.0 } else { 0.0 }, true);
        }
    }

    fn tick(&mut self, cx: &mut Ctx, dt: f32) {
        let waiting = cx.snap.of_kind("car").filter(|c| c.data < 0.3).count();
        cx.set("cars", cx.snap.count("car") as f32);
        cx.set("waiting", waiting as f32);

        cx.set("walkers", cx.snap.count("ped") as f32);

        self.spawn_acc += dt;
        if self.spawn_acc < 0.25 {
            return;
        }
        self.spawn_acc = 0.0;
        let (w, h) = (cx.w as i32, cx.h as i32);
        // Cap the fleet by total lane length so queues cannot spill back into gridlock.
        let capacity = self.lane_capacity(w, h);
        let room = cx.snap.count("car") < capacity;
        if cx.snap.count("ped") < (w / 5) as usize && cx.rng.chance(0.35) {
            self.spawn_ped(cx);
        }
        for (dir, x, y) in self.lanes(w, h) {
            if room && cx.rng.chance(0.1) {
                let (sx, sy) = match dir {
                    0 => (x + 4.0, y),
                    1 => (x, y + 2.0),
                    _ => (x, y),
                };
                self.spawn_car(cx, dir, sx, sy, true);
            }
        }
    }
}
