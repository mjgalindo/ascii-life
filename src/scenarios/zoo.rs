use crate::engine::sprite::std_palette;
use crate::engine::*;
use ratatui::style::Color;

const PATH_H: i32 = 5;
const PEN_W: i32 = 24;
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
];

#[derive(Clone)]
struct Pen {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    /// Standing spot on the path where visitors watch this pen.
    view: (f32, f32),
}

struct Faces {
    awake: [Sprite; 2],
    asleep: [Sprite; 2],
}

fn species_faces(sp: &Species) -> Faces {
    let color = std_palette(sp.color).unwrap();
    let awake = Sprite::mono(sp.awake, color);
    let asleep = Sprite::mono(sp.asleep, color);
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
    Entity::new("emote", Sprite::mono(text, Color::Rgb(255, 250, 200)))
        .at(x, y)
        .vel(0.0, -1.2)
        .z(5)
        .lifetime(1.6)
}

struct Walker {
    route: Vec<(f32, f32)>,
    speed: f32,
    dwell: f32,
    keeper: bool,
    phase: f32,
}

impl Behavior for Walker {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, dt: f32) {
        // The park closes at dusk; everyone heads for the gate.
        if cx.daylight < 0.3 && !self.keeper && self.route.len() > 1 {
            let exit = *self.route.last().unwrap();
            self.route = vec![exit];
            self.dwell = 0.0;
        }
        if self.dwell > 0.0 {
            self.dwell -= dt;
            e.vel = (0.0, 0.0);
            return;
        }
        let Some(&(tx, ty)) = self.route.first() else {
            e.vel = (-self.speed, 0.0);
            return;
        };
        let (dx, dy) = (tx - e.pos.0, ty - e.pos.1);
        let dist = (dx * dx + (dy * 2.0) * (dy * 2.0)).sqrt();
        if dist < 0.8 && self.route.len() > 1 {
            self.route.remove(0);
            self.dwell = cx.rng.range(2.5, 7.0);
            let text = if self.keeper {
                "*munch*"
            } else {
                *cx.rng.pick(&["<3", "!", "?", "wow", ":)", "cute!", "aww"])
            };
            cx.spawn(emote(text, e.pos.0, e.pos.1 - 1.0));
            if self.keeper {
                cx.add("fed", 1.0);
            }
            return;
        }
        if dist < 0.8 {
            // Reached the exit tile; keep walking off-screen.
            self.route.clear();
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
    Entity::new(kind, Sprite::one(f)).z(20).die_offscreen()
}

pub struct Zoo {
    pens: Vec<Pen>,
    path_y: i32,
    spawn_cd: f32,
    keeper_cd: f32,
}

impl Default for Zoo {
    fn default() -> Self {
        Zoo { pens: vec![], path_y: 0, spawn_cd: 0.0, keeper_cd: 10.0 }
    }
}

impl Zoo {
    fn layout(&mut self, w: i32, h: i32) {
        self.pens.clear();
        let two_rows = h >= 20;
        let rows_h = if two_rows { (h - PATH_H - 1) / 2 } else { h - PATH_H - 1 };
        self.path_y = if two_rows { rows_h + 1 } else { rows_h + 1 };
        let cols = (w / PEN_W).max(1);
        let pen_w = (w / cols).min(32);
        let x_off = (w - pen_w * cols) / 2;
        for c in 0..cols {
            let x = x_off + c * pen_w;
            let top_h = rows_h.max(6);
            self.pens.push(Pen {
                x,
                y: 0,
                w: pen_w,
                h: top_h,
                view: ((x + pen_w / 2) as f32, (self.path_y + 1) as f32),
            });
            if two_rows {
                self.pens.push(Pen {
                    x,
                    y: self.path_y + PATH_H,
                    w: pen_w,
                    h: h - (self.path_y + PATH_H),
                    view: ((x + pen_w / 2) as f32, (self.path_y + PATH_H - 2) as f32),
                });
            }
        }
    }

    fn base(&self, w: i32, h: i32) -> Frame {
        let mut f = Frame::filled(w as usize, h as usize, ' ', Color::White, Some(GRASS));
        let py = self.path_y;
        for y in py..(py + PATH_H).min(h) {
            for x in 0..w {
                let ch = if (x + y * 3) % 11 == 0 { '.' } else { ' ' };
                f.set(x as usize, y as usize, ch, Color::Rgb(140, 118, 82), Some(PATH));
            }
        }
        f
    }
}


pub fn species_height(si: usize) -> i32 {
    SPECIES[si].awake.lines().count() as i32
}

pub fn species_count() -> usize {
    SPECIES.len()
}

/// Fenced enclosure with a sign, filled with animals of one species.
/// `rect` is (x, y, w, h); the sign sits on the bottom fence when `sign_bottom`.
pub fn enclosure(cx: &mut Ctx, rect: (i32, i32, i32, i32), si: usize, sign_bottom: bool) {
    let sp = &SPECIES[si];
    let sprite = pen_sprite(rect, sign_bottom, sp, cx.rng);
    cx.spawn(Entity::new("pen", sprite).at(rect.0 as f32, rect.1 as f32).z(40));

    let area = rect.2 * rect.3;
    let big = sp.style == Style::Slither || sp.name == "ELEPHANT" || sp.name == "GIRAFFE";
    let count = if big { 1 + cx.rng.int(0, 2) } else { 1 + (area / 70).max(1) + cx.rng.int(0, 2) };
    for _ in 0..count {
        let faces = species_faces(sp);
        let aw = faces.awake[1].w() as f32;
        let ah = faces.awake[1].h() as f32;
        let b = (
            rect.0 as f32 + 1.0,
            rect.1 as f32 + 1.0,
            (rect.0 + rect.2 - 1) as f32 - aw,
            (rect.1 + rect.3 - 1) as f32 - ah,
        );
        let start = (cx.rng.range(b.0, b.2.max(b.0 + 0.1)), cx.rng.range(b.1, b.3.max(b.1 + 0.1)));
        let mut a = Animal {
            species: si,
            faces,
            bounds: b,
            target: start,
            pause: cx.rng.range(0.0, 3.0),
            right: true,
            sleeping: false,
            speak_cd: cx.rng.range(2.0, 10.0),
        };
        a.pick_target(cx);
        let mut ent = Entity::new("animal", a.faces.awake[1].clone()).at(start.0, start.1).z(30).with(a);
        if sp.style == Style::Fly {
            ent.z = 8;
        }
        cx.spawn(ent);
        cx.add("animals", 1.0);
    }
}

fn pen_sprite(rect: (i32, i32, i32, i32), sign_bottom: bool, sp: &Species, rng: &mut Rng) -> Sprite {
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
    // Sign on the fence nearest the path.
    let label = format!("[ {} ]", sp.name);
    let sy = if sign_bottom { h - 1 } else { 0 };
    let sx = (w.saturating_sub(label.len())) / 2;
    for (i, c) in label.chars().enumerate() {
        f.set(sx + i, sy, c, Color::Rgb(250, 240, 200), Some(Color::Rgb(90, 60, 36)));
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

        // Shuffle species so each run has a different zoo.
        let mut order: Vec<usize> = (0..SPECIES.len()).collect();
        for i in (1..order.len()).rev() {
            let j = cx.rng.int(0, i as i32 + 1) as usize;
            order.swap(i, j);
        }

        let pens = self.pens.clone();
        for (i, pen) in pens.iter().enumerate() {
            let si = order[i % order.len()];
            enclosure(cx, (pen.x, pen.y, pen.w, pen.h), si, pen.y == 0);

            // Lamp posts on the path edge.
            let lx = (pen.x + pen.w / 2) as f32;
            let lamp = Sprite::mono("Y", Color::Rgb(255, 230, 120));
            let ly = if pen.y == 0 { self.path_y as f32 } else { (self.path_y + PATH_H - 1) as f32 };
            cx.spawn(Entity::new("lamp", lamp).at(lx + 8.0, ly).z(25));
        }
    }

    fn tick(&mut self, cx: &mut Ctx, dt: f32) {
        cx.set("visitors", cx.snap.count("visitor") as f32);
        self.spawn_cd -= dt;
        self.keeper_cd -= dt;
        let open = cx.daylight > 0.4;
        if open && self.spawn_cd <= 0.0 && cx.snap.count("visitor") < (cx.w as usize) / 3 {
            self.spawn_cd = cx.rng.range(0.6, 3.0);
            let n_stops = cx.rng.int(1, 4);
            let gate_y = (self.path_y + 1 + cx.rng.int(0, PATH_H - 2)) as f32;
            let mut route = vec![];
            for _ in 0..n_stops {
                let pen = cx.rng.pick(&self.pens);
                let jitter = cx.rng.range(-5.0, 5.0);
                route.push((pen.view.0 + jitter, pen.view.1));
            }
            route.push((-3.0, gate_y));
            let shirts = ['R', 'B', 'G', 'Y', 'M', 'C', 'O', 'p', 'W'];
            let shirt = std_palette(*cx.rng.pick(&shirts)).unwrap();
            let speed = cx.rng.range(2.5, 5.0);
            let phase = cx.rng.range(0.0, 6.0);
            let e = person("visitor", shirt).at(-1.0, gate_y).with(Walker {
                route,
                speed,
                dwell: 0.0,
                keeper: false,
                phase,
            });
            cx.spawn(e);
            cx.add("arrived", 1.0);
            cx.add("cash", 12.0);
        }
        if self.keeper_cd <= 0.0 && cx.snap.count("keeper") < 2 {
            self.keeper_cd = cx.rng.range(12.0, 30.0);
            let pen = cx.rng.pick(&self.pens).clone();
            let gate_y = (self.path_y + 2) as f32;
            let e = person("keeper", Color::Rgb(60, 140, 70)).at(-1.0, gate_y).with(Walker {
                route: vec![(pen.view.0, pen.view.1), (-3.0, gate_y)],
                speed: 4.0,
                dwell: 0.0,
                keeper: true,
                phase: 0.0,
            });
            cx.spawn(e);
        }
    }
}
