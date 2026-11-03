use crate::engine::sprite::std_palette;
use crate::engine::*;
use ratatui::style::Color;

const PATH_H: i32 = 5;
const GRASS: Color = Color::Rgb(52, 92, 50);
const PATH: Color = Color::Rgb(176, 150, 108);
const FENCE: Color = Color::Rgb(120, 82, 48);

#[derive(Clone, Copy, PartialEq)]
enum Style {
    Walk,
    Hop,
    Slither,
    Fly,
}

struct Species {
    name: &'static str,
    awake: &'static str,
    asleep: &'static str,
    color: char,
    speed: f32,
    style: Style,
    ground: (u8, u8, u8),
    sound: &'static str,
}

const SPECIES: &[Species] = &[
    Species {
        name: "LION",
        awake: " ,^^,____ \n(  o_____)>\n ||    || ",
        asleep: "           \n ,^^,____ \n(__-_____)>",
        color: 'O',
        speed: 4.0,
        style: Style::Walk,
        ground: (150, 128, 72),
        sound: "ROAR!",
    },
    Species {
        name: "ELEPHANT",
        awake: "  .--~--.  \n (  o    )_\n /|      |  \\>\n ||      || ",
        asleep: "           \n  .--~--.  \n (  -    )_\n_/|______|__\\>",
        color: 'K',
        speed: 2.5,
        style: Style::Walk,
        ground: (150, 128, 72),
        sound: "TOOOT!",
    },
    Species {
        name: "GIRAFFE",
        awake: "      o>\n      ||\n  ___ ||\n (   )_/\n  || ||",
        asleep: "        \n        \n    __o>\n (_____/\n  _||_| ",
        color: 'Y',
        speed: 2.0,
        style: Style::Walk,
        ground: (146, 130, 76),
        sound: "mmm.",
    },
    Species {
        name: "PENGUINS",
        awake: " (o>\n //)\n  ^^",
        asleep: "     \n(-> \n/_)  ",
        color: 'W',
        speed: 3.0,
        style: Style::Hop,
        ground: (190, 205, 225),
        sound: "squawk!",
    },
    Species {
        name: "MONKEYS",
        awake: " (o o)\n /|*|\\~\n  / \\  ",
        asleep: "      \n(-_-)  \n/|_|\\   ",
        color: 'N',
        speed: 6.0,
        style: Style::Hop,
        ground: (36, 90, 48),
        sound: "OOH OOH!",
    },
    Species {
        name: "SNAKES",
        awake: "  _,_ \n_/   \\_/~o>",
        asleep: "        \n(@@)~~~~~",
        color: 'G',
        speed: 1.4,
        style: Style::Slither,
        ground: (70, 100, 52),
        sound: "ssssss",
    },
    Species {
        name: "FLAMINGOS",
        awake: " _\n(o>\n ) \n |/",
        asleep: "    \n (-)\n  | ",
        color: 'p',
        speed: 3.0,
        style: Style::Walk,
        ground: (44, 98, 140),
        sound: "honk!",
    },
    Species {
        name: "PARROTS",
        awake: " ,\n(o>~\n \\\\  ",
        asleep: "    \n(-)   ",
        color: 'R',
        speed: 9.0,
        style: Style::Fly,
        ground: (36, 90, 48),
        sound: "SQUAWK!",
    },
    Species {
        name: "BEARS",
        awake: " (\")_(\")\n(  o   )>\n /|   |\\ \n  d   b ",
        asleep: "          \n (\")_(\")  \n(__-____)>",
        color: 'n',
        speed: 3.0,
        style: Style::Walk,
        ground: (60, 100, 55),
        sound: "grrr",
    },
    Species {
        name: "ZEBRAS",
        awake: "  ,==o>\n /|||||\\\n  || || ",
        asleep: "        \n  ,==o>  \n_/|||||\\_",
        color: 'W',
        speed: 5.0,
        style: Style::Walk,
        ground: (146, 130, 76),
        sound: "neigh!",
    },
    Species {
        name: "CROCODILES",
        awake: "  ___________  \n_/ o   v v v \\>>\n  ~~~~~~~~~~~  ",
        asleep: "                \n_/_-__________>>",
        color: 'g',
        speed: 1.6,
        style: Style::Slither,
        ground: (44, 80, 100),
        sound: "SNAP!",
    },
    Species {
        name: "KANGAROOS",
        awake: "   ,o>\n  /|  )\n_/ \\ /\n   //  ",
        asleep: "       \n   ,-> \n__/_)_/",
        color: 'O',
        speed: 7.0,
        style: Style::Hop,
        ground: (150, 128, 72),
        sound: "boing!",
    },
    Species {
        name: "TIGER",
        awake: " ,^^,____ \n(  o=====)>\n ||    || ",
        asleep: "           \n ,^^,____  \n(__-=====)>",
        color: 'o',
        speed: 4.5,
        style: Style::Walk,
        ground: (60, 100, 55),
        sound: "GRRR!",
    },
    Species {
        name: "HIPPO",
        awake: " .-~~~-.   \n(  o    )__\n(_______)_)>\n ||   ||   ",
        asleep: "            \n            \n .-~~~-.    \n(_-_____)_)>",
        color: 'p',
        speed: 1.8,
        style: Style::Walk,
        ground: (44, 98, 130),
        sound: "HONK!",
    },
    Species {
        name: "RHINO",
        awake: "     ,/|\n ___/ o \\>\n(_______) \n ||   ||  ",
        asleep: "          \n          \n ___/ - >  \n(_______)  ",
        color: 'K',
        speed: 2.2,
        style: Style::Walk,
        ground: (150, 128, 72),
        sound: "SNORT!",
    },
    Species {
        name: "WOLVES",
        awake: " /\\____  \n< o ____\\_\n  || ||   ",
        asleep: "          \n /\\____  \n<_-___\\_  ",
        color: 'w',
        speed: 6.0,
        style: Style::Walk,
        ground: (188, 200, 214),
        sound: "AWOOO!",
    },
    Species {
        name: "DEER",
        awake: "   \\|/ \n  ( o> \n ,_||_  \n  /  \\ ",
        asleep: "        \n        \n  ,(-> \n ,_||_  ",
        color: 'N',
        speed: 5.0,
        style: Style::Walk,
        ground: (60, 108, 56),
        sound: "...",
    },
    Species {
        name: "TORTOISES",
        awake: "   ,---.  \n  (#####)o>\n   ^^ ^^  ",
        asleep: "          \n   ,---.  \n  (#####)-.",
        color: 'g',
        speed: 0.9,
        style: Style::Walk,
        ground: (130, 134, 76),
        sound: "...slow",
    },
    Species {
        name: "OSTRICHES",
        awake: "   o>\n   |\n .-'-.\n (___)\n  | | ",
        asleep: "      \n      \n      \n .-'-.o\n (___)  ",
        color: 'k',
        speed: 7.0,
        style: Style::Walk,
        ground: (160, 136, 80),
        sound: "BOOM!",
    },
];

#[derive(Clone)]
struct Pen {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    /// Standing spot on the path where visitors watch this pen.
    view: (f32, f32),
    strip: usize,
}

struct Faces {
    awake: [Sprite; 2],
    asleep: [Sprite; 2],
}

pub fn species_sprites(si: usize) -> (&'static str, Sprite, Sprite) {
    let sp = &SPECIES[si];
    let f = species_faces(sp);
    (sp.name, f.awake[1].clone(), f.asleep[1].clone())
}

fn species_faces(sp: &Species) -> Faces {
    let color = std_palette(sp.color).unwrap();
    let mut awake = Sprite::mono(sp.awake, color);
    let mut asleep = Sprite::mono(sp.asleep, color);
    if let Some(c) = crate::art::animal(sp.name, awake.w() as i32, awake.h() as i32) {
        asleep = asleep.with_pixels(vec![c.lying().frame()]);
        awake = awake.with_pixels(vec![c.frame()]);
    }
    Faces { awake: [awake.mirrored(), awake], asleep: [asleep.mirrored(), asleep] }
}

struct Animal {
    species: usize,
    faces: Faces,
    bounds: (f32, f32, f32, f32),
    target: (f32, f32),
    pause: f32,
    right: bool,
    sleeping: bool,
    speak_cd: f32,
}

impl Animal {
    fn pick_target(&mut self, cx: &mut Ctx) {
        let b = self.bounds;
        self.target = (cx.rng.range(b.0, b.2.max(b.0 + 0.1)), cx.rng.range(b.1, b.3.max(b.1 + 0.1)));
    }

    fn apply_pose(&mut self, e: &mut Entity) {
        let i = self.right as usize;
        let s = if self.sleeping { &self.faces.asleep[i] } else { &self.faces.awake[i] };
        e.set_sprite(s.clone());
    }
}

impl Behavior for Animal {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, dt: f32) {
        let sp = &SPECIES[self.species];
        let should_sleep = cx.daylight < 0.33;
        if should_sleep != self.sleeping {
            self.sleeping = should_sleep;
            // Sleeping animals rest on the pen floor; keep the sprite inside bounds.
            self.apply_pose(e);
            e.pos.1 = e.pos.1.min(self.bounds.3);
        }
        if self.sleeping {
            e.vel = (0.0, 0.0);
            if cx.rng.chance(0.002) {
                cx.spawn(emote("z", e.pos.0 + e.w() - 1.0, e.pos.1 - 1.0));
            }
            return;
        }

        self.speak_cd -= dt;
        if self.speak_cd <= 0.0 {
            let (x, y) = (e.pos.0 - 8.0, e.pos.1 + 10.0);
            let watched = cx
                .snap
                .query(x as i32 - 8, y as i32 - 14, 28, 22)
                .iter()
                .any(|o| o.kind == "visitor" || o.kind == "ped");
            if watched && cx.rng.chance(0.35) {
                cx.spawn(emote(sp.sound, e.pos.0, e.pos.1 - 1.0));
            }
            self.speak_cd = cx.rng.range(4.0, 12.0);
        }

        if self.pause > 0.0 {
            self.pause -= dt;
            e.vel = (0.0, 0.0);
            return;
        }
        let (dx, dy) = (self.target.0 - e.pos.0, self.target.1 - e.pos.1);
        let dist = (dx * dx + (dy * 2.0) * (dy * 2.0)).sqrt();
        if dist < 0.7 {
            e.vel = (0.0, 0.0);
            self.pause = match sp.style {
                Style::Hop => cx.rng.range(0.1, 0.6),
                Style::Fly => cx.rng.range(0.0, 0.3),
                _ => cx.rng.range(1.0, 6.0),
            };
            self.pick_target(cx);
            return;
        }
        let speed = sp.speed;
        e.vel = (dx / dist * speed, dy / dist * speed * 0.5);
        if sp.style == Style::Fly {
            e.vel.1 += (e.age * 6.0).sin() * 1.5;
        }
        let right = dx > 0.0;
        if right != self.right && dx.abs() > 0.5 {
            self.right = right;
            self.apply_pose(e);
        }
    }
}

fn emote(text: &'static str, x: f32, y: f32) -> Entity {
    Entity::new("emote", Sprite::text(text, Color::Rgb(255, 250, 200)))
        .at(x, y)
        .vel(0.0, -1.2)
        .z(5)
        .lifetime(1.6)
}

/// Waypoint kinds for walkers.
const TRAVEL: u8 = 0;
const VIEW_PEN: u8 = 1;
const KIOSK: u8 = 2;

struct Walker {
    route: Vec<(f32, f32, u8)>,
    speed: f32,
    dwell: f32,
    keeper: bool,
    phase: f32,
    /// Boulevard column and the vertical centers of the walking strips, for heading home.
    boulevard: f32,
    strips: Vec<f32>,
    world_h: f32,
}

impl Behavior for Walker {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, dt: f32) {
        // The park closes at dusk; visitors walk back along the nearest strip and boulevard.
        if cx.daylight < 0.3 && !self.keeper && self.route.len() > 2 {
            let strip = self
                .strips
                .iter()
                .copied()
                .min_by(|a, b| (a - e.pos.1).abs().total_cmp(&(b - e.pos.1).abs()))
                .unwrap_or(e.pos.1);
            self.route = vec![(self.boulevard, strip, TRAVEL), (self.boulevard, self.world_h + 4.0, TRAVEL)];
            self.dwell = 0.0;
        }
        if self.dwell > 0.0 {
            self.dwell -= dt;
            e.vel = (0.0, 0.0);
            return;
        }
        let Some(&(tx, ty, kind)) = self.route.first() else {
            e.vel = (0.0, self.speed * 0.5);
            return;
        };
        let (dx, dy) = (tx - e.pos.0, ty - e.pos.1);
        let dist = (dx * dx + (dy * 2.0) * (dy * 2.0)).sqrt();
        if dist < 0.8 {
            self.route.remove(0);
            match kind {
                VIEW_PEN | KIOSK => {
                    self.dwell = cx.rng.range(2.5, 7.0);
                    let text = if self.keeper {
                        "*munch*"
                    } else if kind == KIOSK {
                        cx.add("cash", 6.0);
                        *cx.rng.pick(&["yum!", "icecream!", "snack!", "$6"])
                    } else {
                        *cx.rng.pick(&["<3", "!", "?", "wow", ":)", "cute!", "aww", "look!"])
                    };
                    cx.spawn(emote(text, e.pos.0, e.pos.1 - 1.0));
                    if self.keeper {
                        cx.add("fed", 1.0);
                    }
                }
                _ => {}
            }
            return;
        }
        let bob = (e.age * 5.0 + self.phase).sin() * 0.3;
        e.vel = (dx / dist * self.speed, dy / dist * self.speed * 0.5 + bob * 0.3);
    }
}

fn person(kind: &'static str, shirt: Color) -> Entity {
    let head = Color::Rgb(235, 200, 160);
    let mut f = Frame::blank(1, 2);
    f.set(0, 0, 'o', head, None);
    f.set(0, 1, if kind == "keeper" { 'H' } else { 'A' }, shirt, None);
    let hat = if kind == "keeper" { Some(Color::Rgb(70, 110, 60)) } else { None };
    Entity::new(kind, Sprite::one(f).with_pixels(vec![crate::art::person(shirt, hat)])).z(20).die_offscreen()
}

const BOULEVARD_W: i32 = 8;
const PEN_H_TARGET: i32 = 16;
const PEN_W_TARGET: i32 = 34;

pub struct Zoo {
    pens: Vec<Pen>,
    /// Vertical centers of the horizontal walking strips.
    strips: Vec<f32>,
    strip_tops: Vec<i32>,
    kiosks: Vec<(f32, f32, usize)>,
    spawn_cd: f32,
    keeper_cd: f32,
}

impl Default for Zoo {
    fn default() -> Self {
        Zoo {
            pens: vec![],
            strips: vec![],
            strip_tops: vec![],
            kiosks: vec![],
            spawn_cd: 0.0,
            keeper_cd: 6.0,
        }
    }
}

impl Zoo {
    fn boulevard_x(&self) -> f32 {
        (BOULEVARD_W / 2) as f32
    }

    /// Pens in rows above and below each walking strip, a boulevard down the left edge.
    fn layout(&mut self, w: i32, h: i32) {
        self.pens.clear();
        self.strips.clear();
        self.strip_tops.clear();
        self.kiosks.clear();
        let units = ((h as f32 / (2 * PEN_H_TARGET + PATH_H) as f32).round() as i32).max(1);
        let unit_h = h / units;
        let pen_h = ((unit_h - PATH_H) / 2).max(6);
        let avail = w - BOULEVARD_W - 1;
        let cols = (avail / PEN_W_TARGET).max(1);
        let pen_w = avail / cols;
        for u in 0..units {
            let y0 = u * unit_h;
            let strip_y = y0 + pen_h;
            let idx = self.strips.len();
            self.strips.push((strip_y + PATH_H / 2) as f32);
            self.strip_tops.push(strip_y);
            for c in 0..cols {
                let x = BOULEVARD_W + 1 + c * pen_w;
                self.pens.push(Pen {
                    x,
                    y: y0,
                    w: pen_w,
                    h: pen_h,
                    view: ((x + pen_w / 2) as f32, (strip_y + 1) as f32),
                    strip: idx,
                });
                self.pens.push(Pen {
                    x,
                    y: strip_y + PATH_H,
                    w: pen_w,
                    h: pen_h,
                    view: ((x + pen_w / 2) as f32, (strip_y + PATH_H - 2) as f32),
                    strip: idx,
                });
                if c % 3 == 1 {
                    self.kiosks.push(((x + pen_w / 2 + 6) as f32, (strip_y + 1) as f32, idx));
                }
            }
        }
    }

    fn base(&self, w: i32, h: i32) -> Frame {
        let mut f = Frame::filled(w as usize, h as usize, ' ', Color::White, Some(GRASS));
        let speck = Color::Rgb(140, 118, 82);
        let path = |f: &mut Frame, x: i32, y: i32| {
            let ch = if (x * 7 + y * 3) % 13 == 0 { '.' } else { ' ' };
            f.set(x as usize, y as usize, ch, speck, Some(PATH));
        };
        for &top in &self.strip_tops {
            for y in top..(top + PATH_H).min(h) {
                for x in 0..w {
                    path(&mut f, x, y);
                }
            }
        }
        for y in 0..h {
            for x in 0..BOULEVARD_W.min(w) {
                path(&mut f, x, y);
            }
        }
        f
    }

    /// Waypoints from the boulevard bottom to a pen's viewing spot and back toward the boulevard.
    fn route_to(&self, from_strip: usize, to_pen: &Pen, cx: &mut Ctx) -> Vec<(f32, f32, u8)> {
        let blv = self.boulevard_x();
        let mut r = vec![];
        if from_strip != to_pen.strip {
            r.push((blv, self.strips[from_strip], TRAVEL));
            r.push((blv, self.strips[to_pen.strip], TRAVEL));
        }
        let jitter = cx.rng.range(-6.0, 6.0);
        r.push((to_pen.view.0 + jitter, self.strips[to_pen.strip], TRAVEL));
        r.push((to_pen.view.0 + jitter, to_pen.view.1, VIEW_PEN));
        r
    }
}

pub fn species_height(si: usize) -> i32 {
    SPECIES[si].awake.lines().count() as i32
}

pub fn species_count() -> usize {
    SPECIES.len()
}

/// Fenced enclosure with a sign, filled with a herd of one species and some habitat decor.
/// `rect` is (x, y, w, h); the sign sits on the bottom fence when `sign_bottom`.
pub fn enclosure(cx: &mut Ctx, rect: (i32, i32, i32, i32), si: usize, sign_bottom: bool) {
    let sp = &SPECIES[si];
    let sprite = pen_sprite(rect, sp, cx.rng);
    cx.spawn(Entity::new("pen", sprite).at(rect.0 as f32, rect.1 as f32).z(40));

    let label = format!("[ {} ]", sp.name);
    let lx = rect.0 + (rect.2 - label.len() as i32) / 2;
    let ly = if sign_bottom { rect.1 + rect.3 - 1 } else { rect.1 };
    cx.spawn(
        Entity::new("label", Sprite::text(&label, Color::Rgb(250, 240, 200)))
            .at(lx as f32, ly as f32)
            .z(41),
    );

    // Habitat: a few trees tucked against the back fence.
    let trees = (rect.2 * rect.3 / 90).clamp(1, 8);
    for _ in 0..trees {
        let tx = cx.rng.int(rect.0 + 1, (rect.0 + rect.2 - 6).max(rect.0 + 2));
        let ty = cx.rng.int(rect.1 + 1, (rect.1 + rect.3 / 3).max(rect.1 + 2));
        let t = Sprite::mono("&&&\n |", Color::Rgb(70, 150, 70)).with_pixels(vec![crate::art::tree(1)]);
        cx.spawn(Entity::new("tree", t).at(tx as f32, ty as f32).z(45));
    }

    let faces0 = species_faces(sp);
    let (aw, ah) = (faces0.awake[1].w() as i32, faces0.awake[1].h() as i32);
    let interior = ((rect.2 - 2) * (rect.3 - 2)).max(1);
    let herd = (interior as f32 / (aw * ah) as f32 / 2.4).round() as i32;
    let count = (herd + cx.rng.int(-1, 3)).clamp(2, 30);
    for _ in 0..count {
        let faces = species_faces(sp);
        let b = (
            rect.0 as f32 + 1.0,
            rect.1 as f32 + 1.0,
            (rect.0 + rect.2 - 1 - aw) as f32,
            (rect.1 + rect.3 - 1 - ah) as f32,
        );
        let start = (cx.rng.range(b.0, b.2.max(b.0 + 0.1)), cx.rng.range(b.1, b.3.max(b.1 + 0.1)));
        let mut a = Animal {
            species: si,
            faces,
            bounds: b,
            target: start,
            pause: cx.rng.range(0.0, 3.0),
            right: cx.rng.chance(0.5),
            sleeping: false,
            speak_cd: cx.rng.range(2.0, 14.0),
        };
        a.pick_target(cx);
        let sprite = a.faces.awake[a.right as usize].clone();
        let mut ent = Entity::new("animal", sprite).at(start.0, start.1).z(30).with(a);
        if sp.style == Style::Fly {
            ent.z = 8;
        }
        cx.spawn(ent);
        cx.add("animals", 1.0);
    }
}

fn pen_sprite(rect: (i32, i32, i32, i32), sp: &Species, rng: &mut Rng) -> Sprite {
    let (r, g, b) = sp.ground;
    let ground = Color::Rgb(r, g, b);
    let speck = Color::Rgb(r.saturating_sub(18), g.saturating_sub(18), b.saturating_sub(18));
    let (w, h) = (rect.2 as usize, rect.3 as usize);
    let mut f = Frame::filled(w, h, ' ', speck, Some(ground));
    for y in 0..h {
        for x in 0..w {
            if rng.chance(0.03) {
                f.set(x, y, if sp.ground.2 > 120 { '~' } else { ',' }, speck, Some(ground));
            }
        }
    }
    // A pond or mud wallow gives the herd somewhere to gather.
    if rng.chance(0.7) {
        let water = if sp.ground.2 > 100 { Color::Rgb(30, 80, 140) } else { Color::Rgb(46, 100, 150) };
        let (cx0, cy0) = (rng.range(w as f32 * 0.25, w as f32 * 0.75), rng.range(h as f32 * 0.4, h as f32 * 0.8));
        let (rx, ry) = (rng.range(3.0, w as f32 * 0.16), rng.range(1.2, h as f32 * 0.16));
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let (dx, dy) = ((x as f32 - cx0) / rx, (y as f32 - cy0) / ry);
                if dx * dx + dy * dy <= 1.0 {
                    f.set(x, y, '~', Color::Rgb(120, 170, 230), Some(water));
                }
            }
        }
    }
    for x in 0..w {
        for y in [0, h - 1] {
            f.set(x, y, '=', FENCE, Some(ground));
        }
    }
    for y in 0..h {
        for x in [0, w - 1] {
            f.set(x, y, '|', FENCE, Some(ground));
        }
    }
    for (x, y) in [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)] {
        f.set(x, y, '+', FENCE, Some(ground));
    }
    Sprite::one(f)
}

impl Scenario for Zoo {
    fn name(&self) -> &'static str {
        "zoo"
    }

    fn background(&self) -> Color {
        GRASS
    }

    fn day_length(&self) -> Option<f32> {
        Some(150.0)
    }

    fn preferred_size(&self) -> Option<(i32, i32)> {
        Some((380, 140))
    }

    fn hud(&self, g: &Globals, daylight: f32) -> String {
        let v = |k| g.get(k).copied().unwrap_or(0.0) as i32;
        format!(
            "visitors {} (total {}) animals {} fed {} cash ${} | {}",
            v("visitors"),
            v("arrived"),
            v("animals"),
            v("fed"),
            v("cash"),
            if daylight > 0.33 { "open" } else { "closed - lights out" }
        )
    }

    fn setup(&mut self, cx: &mut Ctx) {
        let (w, h) = (cx.w as i32, cx.h as i32);
        self.layout(w, h);
        let base = self.base(w, h);
        cx.spawn(Entity::new("ground", Sprite::one(base)).z(100));

        // Entrance arch at the bottom of the boulevard.
        cx.spawn(
            Entity::new("label", Sprite::text("[ ZOO ]", Color::Rgb(255, 235, 170)))
                .at(0.5, (h - 2) as f32)
                .z(41),
        );

        // Cycle through shuffled species so neighbouring pens differ.
        let mut order: Vec<usize> = (0..SPECIES.len()).collect();
        for i in (1..order.len()).rev() {
            let j = cx.rng.int(0, i as i32 + 1) as usize;
            order.swap(i, j);
        }

        let pens = self.pens.clone();
        for (i, pen) in pens.iter().enumerate() {
            let si = order[i % order.len()];
            let sign_bottom = pen.y + pen.h <= self.strip_tops[pen.strip] + 1;
            enclosure(cx, (pen.x, pen.y, pen.w, pen.h), si, sign_bottom);
        }

        // Lamp posts, benches and kiosks along the strips.
        let lamp = Sprite::mono("Y", Color::Rgb(255, 230, 120));
        for &top in &self.strip_tops {
            let mut x = BOULEVARD_W + 4;
            while x < w - 2 {
                cx.spawn(Entity::new("lamp", lamp.clone()).at(x as f32, top as f32).z(25));
                cx.spawn(
                    Entity::new("bench", Sprite::mono("[===]", Color::Rgb(150, 100, 60)))
                        .at((x + 6) as f32, (top + PATH_H - 1) as f32)
                        .z(26),
                );
                x += 16;
            }
        }
        for &(x, y, _) in &self.kiosks {
            let kiosk = Sprite::mono(" .--. \n|ICE | \n|____|", Color::Rgb(230, 120, 140));
            cx.spawn(Entity::new("kiosk", kiosk).at(x - 3.0, y).z(35));
        }
    }

    fn tick(&mut self, cx: &mut Ctx, dt: f32) {
        cx.set("visitors", cx.snap.count("visitor") as f32);
        self.spawn_cd -= dt;
        self.keeper_cd -= dt;
        let open = cx.daylight > 0.4;
        let cap = (cx.w * cx.h / 220.0) as usize;
        let blv = self.boulevard_x();
        if open && self.spawn_cd <= 0.0 && cx.snap.count("visitor") < cap {
            self.spawn_cd = cx.rng.range(0.15, 1.0);
            let start_strip = 0usize;
            let n_stops = cx.rng.int(2, 7);
            let mut route = vec![];
            let mut strip = start_strip;
            route.push((blv, self.strips[0], TRAVEL));
            for _ in 0..n_stops {
                if cx.rng.chance(0.2) && !self.kiosks.is_empty() {
                    let (kx, ky, ks) = *cx.rng.pick(&self.kiosks);
                    if ks != strip {
                        route.push((blv, self.strips[strip], TRAVEL));
                        route.push((blv, self.strips[ks], TRAVEL));
                    }
                    route.push((kx, self.strips[ks], TRAVEL));
                    route.push((kx, ky + 3.0, KIOSK));
                    strip = ks;
                } else {
                    let pen = cx.rng.pick(&self.pens).clone();
                    route.extend(self.route_to(strip, &pen, cx));
                    strip = pen.strip;
                }
            }
            route.push((blv, self.strips[strip], TRAVEL));
            route.push((blv, cx.h + 4.0, TRAVEL));
            let shirts = ['R', 'B', 'G', 'Y', 'M', 'C', 'O', 'p', 'W'];
            let shirt = std_palette(*cx.rng.pick(&shirts)).unwrap();
            let speed = cx.rng.range(3.0, 6.0);
            let phase = cx.rng.range(0.0, 6.0);
            let sx = cx.rng.range(1.0, BOULEVARD_W as f32 - 2.0);
            let e = person("visitor", shirt).at(sx, cx.h - 1.0).with(Walker {
                route,
                speed,
                dwell: 0.0,
                keeper: false,
                phase,
                boulevard: blv,
                strips: self.strips.clone(),
                world_h: cx.h,
            });
            cx.spawn(e);
            cx.add("arrived", 1.0);
            cx.add("cash", 12.0);
        }
        let keeper_cap = (cx.w / 30.0) as usize;
        if self.keeper_cd <= 0.0 && cx.snap.count("keeper") < keeper_cap {
            self.keeper_cd = cx.rng.range(2.0, 8.0);
            let pen = cx.rng.pick(&self.pens).clone();
            let mut route = vec![(blv, self.strips[0], TRAVEL)];
            route.extend(self.route_to(0, &pen, cx));
            route.push((blv, self.strips[pen.strip], TRAVEL));
            route.push((blv, cx.h + 4.0, TRAVEL));
            let e = person("keeper", Color::Rgb(60, 140, 70)).at(blv, cx.h - 1.0).with(Walker {
                route,
                speed: 5.0,
                dwell: 0.0,
                keeper: true,
                phase: 0.0,
                boulevard: blv,
                strips: self.strips.clone(),
                world_h: cx.h,
            });
            cx.spawn(e);
        }
    }
}
