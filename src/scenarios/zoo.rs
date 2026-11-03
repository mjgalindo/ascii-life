use super::common::{firework, random_name, Spark, Weather};
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

#[derive(Clone)]
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

/// Child-sized version of a species: tiny ASCII glyphs, shrunken pixel art.
fn baby_faces(sp: &Species) -> Faces {
    let color = std_palette(sp.color).unwrap();
    let full = Sprite::mono(sp.awake, color);
    let (w, h) = ((full.w() as i32 * 3 / 5).max(3), (full.h() as i32 * 3 / 5).max(1));
    let mut awake = Sprite::mono("o~", color);
    let mut asleep = Sprite::mono("-~", color);
    if let Some(c) = crate::art::animal(sp.name, w, h) {
        asleep = asleep.with_pixels(vec![c.lying().frame()]);
        awake = awake.with_pixels(vec![c.frame()]);
    }
    Faces { awake: [awake.mirrored(), awake], asleep: [asleep.mirrored(), asleep] }
}

/// Species names are stored as sign text; this gives the word for one animal.
fn singular(name: &str) -> String {
    let n = name.to_lowercase();
    match n.as_str() {
        "wolves" => "wolf".to_string(),
        "ostriches" => "ostrich".to_string(),
        "deer" => n,
        _ => n.strip_suffix('s').map(str::to_string).unwrap_or(n),
    }
}

fn is_predator(name: &str) -> bool {
    matches!(name, "LION" | "TIGER" | "WOLVES" | "BEARS" | "CROCODILES")
}

fn is_nocturnal(name: &str) -> bool {
    matches!(name, "WOLVES" | "TIGER")
}

fn is_paraders(name: &str) -> bool {
    matches!(name, "PENGUINS" | "FLAMINGOS" | "OSTRICHES" | "KANGAROOS")
}

const MODE_NORMAL: i32 = 0;
const MODE_ESCAPED: i32 = 2;
const MODE_RETURNING: i32 = 3;
const MODE_EATING: i32 = 4;
const MODE_PARADE: i32 = 5;

/// Ties an animal to the park's systems. City dioramas run without one.
#[derive(Clone, Copy)]
struct PenLink {
    pen: usize,
    trough: Id,
    view: (f32, f32),
    strip_y: f32,
    herd_cap: usize,
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
    think_cd: f32,
    name: &'static str,
    age: f32,
    lifespan: f32,
    hunger: f32,
    mother: Option<Id>,
    offspring: u32,
    escapes: u32,
    link: Option<PenLink>,
    route: Vec<(f32, f32)>,
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

    fn walk_to(&mut self, e: &mut Entity, to: (f32, f32), speed: f32) -> f32 {
        let (dx, dy) = (to.0 - e.pos.0, to.1 - e.pos.1);
        let dist = (dx * dx + (dy * 2.0) * (dy * 2.0)).sqrt();
        if dist < 0.7 {
            e.vel = (0.0, 0.0);
            return dist;
        }
        e.vel = (dx / dist * speed, dy / dist * speed * 0.5);
        let right = dx > 0.0;
        if right != self.right && dx.abs() > 0.5 {
            self.right = right;
            self.apply_pose(e);
        }
        dist
    }

    fn inside_pen(&self, e: &Entity) -> bool {
        let b = self.bounds;
        e.pos.0 >= b.0 && e.pos.0 <= b.2.max(b.0) && e.pos.1 >= b.1 && e.pos.1 <= b.3.max(b.1)
    }

    fn escape(&mut self, e: &mut Entity, cx: &mut Ctx, why: &str) {
        let sp = &SPECIES[self.species];
        e.state = MODE_ESCAPED;
        self.escapes += 1;
        self.sleeping = false;
        self.apply_pose(e);
        e.z = 18;
        let line = match sp.name {
            "TORTOISES" => format!("{} the tortoise has escaped. Nobody is in a hurry.", self.name),
            n if is_predator(n) => format!("ALERT! {} the {} is loose {why}!", self.name, singular(n)),
            n => format!("{} the {} got out {why}.", self.name, singular(n)),
        };
        cx.news(line, Some(e.pos));
        cx.add("rep", -4.0);
        cx.add("escapes", 1.0);
        self.target = e.pos;
    }

    /// Life on the run: predators stalk visitors, everything else just bolts around.
    fn on_the_run(&mut self, e: &mut Entity, cx: &mut Ctx, dt: f32) {
        let sp = &SPECIES[self.species];
        let r = e.rect();
        let caught = cx.snap.query(r.0 - 1, r.1 - 1, r.2 + 2, r.3 + 2).iter().any(|o| o.kind == "ranger");
        if caught {
            e.state = MODE_RETURNING;
            cx.news(format!("Rangers caught {} and are walking them home.", self.name), Some(e.pos));
            cx.spawn(emote("gotcha!", e.pos.0, e.pos.1 - 1.0));
            self.target = ((self.bounds.0 + self.bounds.2) / 2.0, (self.bounds.1 + self.bounds.3) / 2.0);
            return;
        }
        self.think_cd -= dt;
        if self.think_cd <= 0.0 {
            self.think_cd = cx.rng.range(1.0, 3.0);
            let (x, y) = (e.pos.0 as i32, e.pos.1 as i32);
            let prey = is_predator(sp.name)
                .then(|| cx.snap.query(x - 30, y - 14, 60, 28).into_iter().find(|o| o.kind == "visitor").map(|o| o.pos))
                .flatten();
            self.target = prey.unwrap_or_else(|| {
                (
                    (e.pos.0 + cx.rng.range(-30.0, 30.0)).clamp(1.0, cx.w - 12.0),
                    (e.pos.1 + cx.rng.range(-12.0, 12.0)).clamp(1.0, cx.h - 6.0),
                )
            });
            if prey.is_some() {
                cx.spawn(emote(sp.sound, e.pos.0, e.pos.1 - 1.0));
            }
        }
        let t = self.target;
        self.walk_to(e, t, sp.speed * 1.25);
    }

    fn update_baby(&mut self, e: &mut Entity, cx: &mut Ctx, mother: Id) {
        let sp = &SPECIES[self.species];
        if self.age > 3.0 {
            self.mother = None;
            self.faces = species_faces(sp);
            self.apply_pose(e);
            return;
        }
        if let Some(m) = cx.snap.get(mother) {
            if m.state != MODE_ESCAPED {
                let to = (m.pos.0 - 2.0, m.pos.1 + 1.0);
                let to = (to.0.clamp(self.bounds.0, self.bounds.2.max(self.bounds.0)), to.1.clamp(self.bounds.1, self.bounds.3.max(self.bounds.1)));
                if self.walk_to(e, to, sp.speed * 1.2) < 2.5 {
                    e.vel = (0.0, 0.0);
                }
                return;
            }
        }
        self.mother = None;
    }
}

impl Behavior for Animal {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, dt: f32) {
        let sp = &SPECIES[self.species];
        self.age += dt / ZOO_DAY;

        match e.state {
            MODE_ESCAPED => return self.on_the_run(e, cx, dt),
            MODE_RETURNING => {
                let t = self.target;
                if self.walk_to(e, t, sp.speed.max(5.0)) < 1.5 || self.inside_pen(e) {
                    e.state = MODE_NORMAL;
                    e.z = 30;
                    self.hunger = 0.2;
                    self.think_cd = 2.0;
                }
                return;
            }
            MODE_PARADE => {
                let Some(&next) = self.route.first() else {
                    e.state = MODE_RETURNING;
                    self.target = ((self.bounds.0 + self.bounds.2) / 2.0, (self.bounds.1 + self.bounds.3) / 2.0);
                    return;
                };
                if self.walk_to(e, next, sp.speed.max(3.0)) < 1.0 {
                    self.route.remove(0);
                }
                return;
            }
            _ => {}
        }

        let asleep_now = if is_nocturnal(sp.name) { cx.daylight > 0.75 } else { cx.daylight < 0.33 };
        if asleep_now != self.sleeping && e.state == MODE_NORMAL {
            self.sleeping = asleep_now;
            self.apply_pose(e);
            e.pos.1 = e.pos.1.min(self.bounds.3.max(self.bounds.1));
        }
        if self.sleeping {
            e.vel = (0.0, 0.0);
            if cx.rng.chance(0.002) {
                // Sleepers roll over to a free patch rather than lying on each other.
                let r = e.rect();
                let piled = cx.snap.query(r.0, r.1, r.2, r.3).iter().any(|o| o.kind == "animal" && o.id != e.id);
                if piled {
                    let b = self.bounds;
                    e.pos = (
                        (e.pos.0 + cx.rng.range(-5.0, 5.0)).clamp(b.0, b.2.max(b.0)),
                        (e.pos.1 + cx.rng.range(-2.0, 2.0)).clamp(b.1, b.3.max(b.1)),
                    );
                }
                cx.spawn(emote("z", e.pos.0 + e.w() - 1.0, e.pos.1 - 1.0));
            }
            return;
        }

        if let Some(mother) = self.mother {
            return self.update_baby(e, cx, mother);
        }

        self.speak_cd -= dt;
        if self.speak_cd <= 0.0 {
            let (x, y) = (e.pos.0 as i32, e.pos.1 as i32);
            let watched = cx
                .snap
                .query(x - 14, y - 8, 30, 18)
                .iter()
                .any(|o| o.kind == "visitor" || o.kind == "ped");
            if watched && cx.rng.chance(0.35) {
                cx.spawn(emote(sp.sound, e.pos.0, e.pos.1 - 1.0));
            }
            self.speak_cd = cx.rng.range(4.0, 12.0);
        }

        self.think_cd -= dt;
        if self.think_cd <= 0.0 {
            self.think_cd = cx.rng.range(0.4, 0.9);
            self.think(e, cx);
            if e.state != MODE_NORMAL && e.state != MODE_EATING {
                return;
            }
        }

        if let Some(link) = self.link {
            self.hunger = (self.hunger + dt / (ZOO_DAY * 3.0)).min(1.0);
            if e.state == MODE_EATING {
                e.vel = (0.0, 0.0);
                self.hunger -= dt * 0.25;
                if self.hunger < 0.1 || cx.snap.get(link.trough).is_none_or(|t| t.data <= 0.0) {
                    e.state = MODE_NORMAL;
                }
                return;
            }
            if self.hunger > 0.6 {
                if let Some(t) = cx.snap.get(link.trough).filter(|t| t.data > 0.02) {
                    let spot = (
                        t.pos.0.clamp(self.bounds.0, self.bounds.2.max(self.bounds.0)),
                        t.pos.1.clamp(self.bounds.1, self.bounds.3.max(self.bounds.1)),
                    );
                    if self.walk_to(e, spot, sp.speed) < 1.5 {
                        e.state = MODE_EATING;
                        cx.spawn(emote("*nom*", e.pos.0, e.pos.1 - 1.0));
                    }
                    return;
                }
            }
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
        e.vel = (dx / dist * sp.speed, dy / dist * sp.speed * 0.5);
        if sp.style == Style::Fly {
            e.vel.1 += (e.age * 6.0).sin() * 1.5;
        }
        let right = dx > 0.0;
        if right != self.right && dx.abs() > 0.5 {
            self.right = right;
            self.apply_pose(e);
        }
    }

    fn describe(&self, e: &Entity) -> Option<String> {
        let sp = &SPECIES[self.species];
        let doing = match e.state {
            MODE_ESCAPED => "ON THE LOOSE",
            MODE_RETURNING => "being walked home",
            MODE_EATING => "eating",
            MODE_PARADE => "on parade",
            _ if self.sleeping => "asleep",
            _ if self.mother.is_some() => "following mum",
            _ => "wandering",
        };
        let mut out = format!(
            "{} the {}\nage {:.1} days, {}\nhunger {:.0}%",
            self.name,
            singular(sp.name),
            self.age,
            doing,
            self.hunger * 100.0
        );
        if self.offspring > 0 {
            out += &format!("\n{} offspring", self.offspring);
        }
        if self.escapes > 0 {
            out += &format!("\n{} escapes", self.escapes);
        }
        Some(out)
    }
}

impl Animal {
    /// Slower decisions, staggered across the herd: spacing, parades, births, escapes and old age.
    fn think(&mut self, e: &mut Entity, cx: &mut Ctx) {
        let sp = &SPECIES[self.species];
        // Herd members shuffle apart instead of standing inside each other.
        let r = e.rect();
        let crowd = cx.snap.query(r.0, r.1, r.2, r.3);
        if let Some(o) = crowd.iter().find(|o| o.kind == "animal" && o.id != e.id) {
            let away = if e.pos.0 >= o.pos.0 { 1.0 } else { -1.0 };
            let b = self.bounds;
            self.target = (
                (e.pos.0 + away * cx.rng.range(4.0, 10.0)).clamp(b.0, b.2.max(b.0)),
                (e.pos.1 + cx.rng.range(-3.0, 3.0)).clamp(b.1, b.3.max(b.1)),
            );
            self.pause = 0.0;
        }

        let Some(link) = self.link else { return };
        if self.mother.is_some() {
            return;
        }

        if self.age > self.lifespan && cx.rng.chance(0.002) {
            cx.news(format!("{} the {} passed away peacefully, aged {:.0} days.", self.name, singular(sp.name), self.age), Some(e.pos));
            e.kill();
            return;
        }

        if cx.global("parade") as usize == link.pen + 1 && is_paraders(sp.name) {
            e.state = MODE_PARADE;
            let (vx, sy) = (link.view.0, link.strip_y);
            let far = if vx > cx.w / 2.0 { vx - 50.0 } else { vx + 50.0 };
            self.route = vec![(vx, sy), (far, sy), (vx, sy)];
            e.z = 18;
            return;
        }

        if cx.global("breach") as usize == link.pen + 1 && cx.rng.chance(0.08) {
            return self.escape(e, cx, "after lightning split the fence");
        }
        let starving = self.hunger > 0.95;
        let odds = if starving { 0.0006 } else { 0.000003 };
        if cx.rng.chance(odds) {
            let why = if starving { "looking for dinner" } else { "through a gap in the fence" };
            return self.escape(e, cx, why);
        }

        if self.age > 3.0 && self.hunger < 0.5 && cx.rng.chance(0.00012) {
            let mates = cx
                .snap
                .query(self.bounds.0 as i32, self.bounds.1 as i32, (self.bounds.2 - self.bounds.0) as i32 + 8, (self.bounds.3 - self.bounds.1) as i32 + 4)
                .iter()
                .filter(|o| o.kind == "animal")
                .count();
            if mates >= 2 && mates < link.herd_cap {
                self.offspring += 1;
                let faces = baby_faces(sp);
                let sprite = faces.awake[1].clone();
                let name = random_name(cx.rng);
                let baby = Animal {
                    species: self.species,
                    faces,
                    bounds: self.bounds,
                    target: e.pos,
                    pause: 0.0,
                    right: true,
                    sleeping: false,
                    speak_cd: 5.0,
                    think_cd: 1.0,
                    name,
                    age: 0.0,
                    lifespan: self.lifespan,
                    hunger: 0.0,
                    mother: Some(e.id),
                    offspring: 0,
                    escapes: 0,
                    link: self.link,
                    route: vec![],
                };
                cx.spawn(Entity::new("animal", sprite).at(e.pos.0, e.pos.1).z(29).with(baby));
                cx.news(format!("A baby {} named {} is born to {}!", singular(sp.name), name, self.name), Some(e.pos));
                cx.add("births", 1.0);
                cx.add("rep", 2.0);
            }
        }
    }
}

/// Food in a pen. Empties as the herd eats and refills when a keeper stops by.
struct Trough;

impl Behavior for Trough {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, dt: f32) {
        let r = e.rect();
        let near = cx.snap.query(r.0 - 8, r.1 - 5, r.2 + 16, r.3 + 10);
        let eaters = near.iter().filter(|o| o.kind == "animal" && o.state == MODE_EATING).count();
        e.data = (e.data - eaters as f32 * 0.02 * dt).max(0.0);
        if e.data < 0.5 && near.iter().any(|o| o.kind == "keeper" && o.state == 1) {
            e.data = 1.0;
        }
        e.frame = (e.data <= 0.02) as usize;
    }

    fn describe(&self, e: &Entity) -> Option<String> {
        Some(format!("feed trough\n{:.0}% full", e.data * 100.0))
    }
}

/// Chases one escaped animal, then walks off the map.
struct Ranger {
    target: Id,
}

impl Behavior for Ranger {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, _dt: f32) {
        let chase = cx.snap.get(self.target).filter(|t| t.state == MODE_ESCAPED).map(|t| t.pos);
        let to = chase.unwrap_or((BOULEVARD_W as f32 / 2.0, cx.h + 4.0));
        let (dx, dy) = (to.0 - e.pos.0, to.1 - e.pos.1);
        let d = (dx * dx + dy * dy * 4.0).sqrt().max(0.1);
        e.vel = (dx / d * 10.0, dy / d * 5.0);
    }

    fn describe(&self, _e: &Entity) -> Option<String> {
        Some("park ranger, net at the ready".to_string())
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
    name: &'static str,
    alarm_cd: f32,
    panicked: bool,
}

impl Walker {
    fn head_home(&mut self, e: &Entity) {
        let strip = self
            .strips
            .iter()
            .copied()
            .min_by(|a, b| (a - e.pos.1).abs().total_cmp(&(b - e.pos.1).abs()))
            .unwrap_or(e.pos.1);
        self.route = vec![(self.boulevard, strip, TRAVEL), (self.boulevard, self.world_h + 4.0, TRAVEL)];
        self.dwell = 0.0;
    }
}

impl Behavior for Walker {
    fn describe(&self, _e: &Entity) -> Option<String> {
        let role = if self.keeper { "keeper" } else { "visitor" };
        let mood = if self.panicked { "panicking!" } else { "having a nice day" };
        Some(format!("{}, {role}\n{mood}", self.name))
    }

    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, dt: f32) {
        e.state = 0;
        if !self.keeper && !self.panicked {
            self.alarm_cd -= dt;
            if self.alarm_cd <= 0.0 {
                self.alarm_cd = cx.rng.range(0.4, 0.9);
                let (x, y) = (e.pos.0 as i32, e.pos.1 as i32);
                let danger = cx.snap.query(x - 20, y - 8, 40, 16).iter().any(|o| o.kind == "animal" && o.state == MODE_ESCAPED);
                if danger {
                    self.panicked = true;
                    self.speed *= 2.0;
                    self.head_home(e);
                    let shout = *cx.rng.pick(&["AAAH!", "RUN!", "EEK!", "HELP!"]);
                    cx.spawn(emote(shout, e.pos.0, e.pos.1 - 1.0));
                    cx.add("rep", -0.3);
                } else if cx.global("rain") >= 2.0 && self.route.len() > 2 && cx.rng.chance(0.05) {
                    self.head_home(e);
                    cx.spawn(emote("so wet", e.pos.0, e.pos.1 - 1.0));
                }
            }
        }
        // The park closes at dusk; visitors walk back along the nearest strip and boulevard.
        if cx.daylight < 0.3 && !self.keeper && self.route.len() > 2 {
            self.head_home(e);
        }
        if self.dwell > 0.0 {
            self.dwell -= dt;
            e.vel = (0.0, 0.0);
            e.state = self.keeper as i32;
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
                        cx.add("cash", -15.0);
                    } else {
                        cx.add("rep", 0.05);
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

/// Seconds in a zoo day; animal ages and appetites are measured against it.
const ZOO_DAY: f32 = 150.0;
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
    weather: Weather,
    parade_until: (f32, f64),
    breach_until: (f32, f64),
    species_of_pen: Vec<usize>,
    /// Per pen: trough entity and when a keeper was last sent to it.
    troughs: Vec<(Id, f64)>,
    /// Escaped animals that already have rangers on the way.
    hunted: Vec<Id>,
    dispatch_cd: f32,
    last_parade_day: i64,
    show_cd: f32,
    rep_trend: Spark,
    trend_cd: f32,
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
            weather: Weather::default(),
            parade_until: (0.0, 0.0),
            breach_until: (0.0, 0.0),
            species_of_pen: vec![],
            troughs: vec![],
            hunted: vec![],
            dispatch_cd: 3.0,
            last_parade_day: -1,
            show_cd: 0.0,
            rep_trend: Spark::default(),
            trend_cd: 0.0,
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
    populate_pen(cx, rect, si, sign_bottom, None);
}

/// Builds a pen and its herd. With a viewing spot the pen joins the park's systems: it gets
/// a trough by the path and its animals eat, breed and escape. Returns the trough's id.
fn populate_pen(
    cx: &mut Ctx,
    rect: (i32, i32, i32, i32),
    si: usize,
    sign_bottom: bool,
    managed: Option<(usize, (f32, f32), f32)>,
) -> Option<Id> {
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

    let faces = species_faces(sp);
    let (aw, ah) = (faces.awake[1].w() as i32, faces.awake[1].h() as i32);
    let interior = ((rect.2 - 2) * (rect.3 - 2)).max(1);
    let herd = (interior as f32 / (aw * ah) as f32 / 3.6).round() as i32;
    let count = (herd + cx.rng.int(-1, 3)).clamp(2, 30);
    let link = managed.map(|(pen, view, strip_y)| {
        // The trough sits against the fence facing the path, within a keeper's reach.
        let ty = if sign_bottom { rect.1 + rect.3 - 2 } else { rect.1 + 1 };
        let mut full = Frame::blank(5, 1);
        full.text(0, 0, "[###]", Color::Rgb(226, 190, 90));
        let mut empty = Frame::blank(5, 1);
        empty.text(0, 0, "[___]", Color::Rgb(150, 110, 80));
        let mut trough = Entity::new("trough", Sprite::new(vec![full, empty]))
            .at(view.0 - 2.0, ty as f32)
            .z(35)
            .with(Trough);
        trough.data = 1.0;
        let trough = cx.spawn(trough);
        PenLink { pen, trough, view, strip_y, herd_cap: count as usize + 4 }
    });
    for _ in 0..count {
        let faces = faces.clone();
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
            think_cd: cx.rng.range(0.0, 2.0),
            name: random_name(cx.rng),
            age: cx.rng.range(3.0, 20.0),
            lifespan: cx.rng.range(30.0, 70.0),
            hunger: cx.rng.range(0.0, 0.5),
            mother: None,
            offspring: 0,
            escapes: 0,
            link,
            route: vec![],
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
    link.map(|l| l.trough)
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

    fn tools(&self) -> &'static [&'static str] {
        &["break the fence (click a pen)", "parade now", "thunderstorm"]
    }

    fn place(&mut self, cx: &mut Ctx, tool: usize, x: f32, y: f32) {
        match tool {
            0 => {
                let hit = self.pens.iter().position(|p| {
                    x >= p.x as f32 && x < (p.x + p.w) as f32 && y >= p.y as f32 && y < (p.y + p.h) as f32
                });
                if let Some(i) = hit {
                    self.breach_until = (i as f32 + 1.0, cx.time + 3.0);
                    let name = singular(SPECIES[self.species_of_pen[i]].name);
                    cx.news(format!("Someone has cut the fence of the {name} pen!"), Some((x, y)));
                }
            }
            1 => {
                let troupes: Vec<usize> =
                    (0..self.pens.len()).filter(|&i| is_paraders(SPECIES[self.species_of_pen[i]].name)).collect();
                if !troupes.is_empty() {
                    let i = *cx.rng.pick(&troupes);
                    self.parade_until = (i as f32 + 1.0, cx.time + 1.5);
                    cx.news("A special parade is on!", Some(self.pens[i].view));
                }
            }
            _ => self.weather.force_storm(cx),
        }
    }

    fn preferred_size(&self) -> Option<(i32, i32)> {
        Some((380, 140))
    }

    fn hud(&self, g: &Globals, daylight: f32) -> String {
        let v = |k| g.get(k).copied().unwrap_or(0.0) as i32;
        format!(
            "rep {}/100 {} ${} | visitors {} animals {} (+{} born) fed {} escapes {} | {}",
            v("rep").min(100),
            self.rep_trend.render(),
            v("cash"),
            v("visitors"),
            v("animals"),
            v("births"),
            v("fed"),
            v("escapes"),
            if daylight > 0.33 { "open" } else { "closed" }
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
            let strip_y = self.strips[pen.strip];
            let trough = populate_pen(cx, (pen.x, pen.y, pen.w, pen.h), si, sign_bottom, Some((i, pen.view, strip_y)));
            self.troughs.push((trough.unwrap_or(0), -1000.0));
        }
        self.species_of_pen = pens.iter().enumerate().map(|(i, _)| order[i % order.len()]).collect();
        cx.set("rep", 60.0);

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
            let kiosk = Sprite::mono(" .--. \n|ICE | \n|____|", Color::Rgb(230, 120, 140)).with_pixels(vec![crate::art::kiosk(6, 6)]);
            cx.spawn(Entity::new("kiosk", kiosk).at(x - 3.0, y).z(35));
        }
    }

    fn tick(&mut self, cx: &mut Ctx, dt: f32) {
        cx.set("visitors", cx.snap.count("visitor") as f32);
        cx.set("animals", cx.snap.count("animal") as f32);
        cx.set("rep", cx.global("rep").clamp(0.0, 100.0));
        // Signals stay up long enough for every staggered animal to think once.
        cx.set("parade", if cx.time < self.parade_until.1 { self.parade_until.0 } else { 0.0 });
        cx.set("breach", if cx.time < self.breach_until.1 { self.breach_until.0 } else { 0.0 });
        self.trend_cd -= dt;
        if self.trend_cd <= 0.0 {
            self.trend_cd = 20.0;
            self.rep_trend.push(cx.global("rep"));
        }
        self.run_events(cx, dt);
        self.spawn_cd -= dt;
        self.keeper_cd -= dt;
        let open = cx.daylight > 0.4;
        let cap = (cx.w * cx.h / 220.0) as usize;
        let blv = self.boulevard_x();
        if open && self.spawn_cd <= 0.0 && cx.snap.count("visitor") < cap {
            // A good reputation fills the park; a bad one or a loose lion empties it.
            let pull = (cx.global("rep") / 60.0).clamp(0.2, 2.0);
            let fear = if self.hunted.is_empty() { 1.0 } else { 3.0 };
            self.spawn_cd = cx.rng.range(0.15, 1.0) * fear / pull;
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
                name: random_name(cx.rng),
                alarm_cd: 0.5,
                panicked: false,
            });
            cx.spawn(e);
            cx.add("arrived", 1.0);
            cx.add("cash", 12.0);
        }
    }
}

impl Zoo {
    fn spawn_keeper(&self, cx: &mut Ctx, pen: &Pen) {
        let blv = self.boulevard_x();
        let mut route = vec![(blv, self.strips[0], TRAVEL)];
        route.extend(self.route_to(0, pen, cx));
        // Keepers feed from the spot right beside the trough.
        let n = route.len();
        route[n - 2].0 = pen.view.0;
        route[n - 1].0 = pen.view.0;
        route.push((blv, self.strips[pen.strip], TRAVEL));
        route.push((blv, cx.h + 4.0, TRAVEL));
        let e = person("keeper", Color::Rgb(60, 140, 70)).at(blv, cx.h - 1.0).with(Walker {
            route,
            speed: 7.0,
            dwell: 0.0,
            keeper: true,
            phase: 0.0,
            boulevard: blv,
            strips: self.strips.clone(),
            world_h: cx.h,
            name: random_name(cx.rng),
            alarm_cd: 0.0,
            panicked: false,
        });
        cx.spawn(e);
    }

    /// Everything that happens because of the state of the park rather than one animal.
    fn run_events(&mut self, cx: &mut Ctx, dt: f32) {
        let winter = false;
        if let Some(p) = self.weather.tick(cx, dt, winter) {
            let hit = self.pens.iter().position(|pen| {
                p.0 >= pen.x as f32 && p.0 < (pen.x + pen.w) as f32 && p.1 >= pen.y as f32 && p.1 < (pen.y + pen.h) as f32
            });
            if let Some(i) = hit {
                self.breach_until = (i as f32 + 1.0, cx.time + 1.5);
                let name = singular(SPECIES[self.species_of_pen[i]].name);
                cx.news(format!("Lightning strikes the {name} enclosure!"), Some(p));
            }
        }

        self.dispatch_cd -= dt;
        if self.dispatch_cd <= 0.0 {
            self.dispatch_cd = 1.0;
            // Rangers go after anything on the loose, two to a target.
            self.hunted.retain(|id| cx.snap.get(*id).is_some_and(|a| a.state == MODE_ESCAPED));
            let runaways: Vec<(Id, (f32, f32))> =
                cx.snap.of_kind("animal").filter(|a| a.state == MODE_ESCAPED).map(|a| (a.id, a.pos)).collect();
            for (id, pos) in runaways {
                if self.hunted.contains(&id) {
                    continue;
                }
                self.hunted.push(id);
                for i in 0..2 {
                    let ranger = Sprite::mono("R", Color::Rgb(255, 170, 60))
                        .with_pixels(vec![crate::art::person(Color::Rgb(230, 140, 40), Some(Color::Rgb(90, 70, 40)))]);
                    let spawn_y = (pos.1 + i as f32 * 6.0 - 3.0).clamp(1.0, cx.h - 2.0);
                    cx.spawn(
                        Entity::new("ranger", ranger)
                            .at(self.boulevard_x(), spawn_y)
                            .z(17)
                            .die_offscreen()
                            .with(Ranger { target: id }),
                    );
                }
            }

            // Keepers go where the food is low, oldest request first.
            let cap = (cx.w / 18.0) as usize;
            for i in 0..self.pens.len() {
                if cx.snap.count("keeper") >= cap {
                    break;
                }
                let (trough, last) = self.troughs[i];
                let low = cx.snap.get(trough).is_some_and(|t| t.data < 0.35);
                if low && cx.time - last > 90.0 {
                    self.troughs[i].1 = cx.time;
                    let pen = self.pens[i].clone();
                    self.spawn_keeper(cx, &pen);
                    break;
                }
            }
            let empties = self.troughs.iter().filter(|(t, _)| cx.snap.get(*t).is_some_and(|t| t.data <= 0.02)).count();
            if empties > 0 {
                cx.add("rep", -0.01 * empties as f32);
            }
        }

        // One show a day, mid-morning, if the park has anything that marches.
        let day = (cx.time / ZOO_DAY as f64) as i64;
        if day != self.last_parade_day && cx.daylight > 0.8 {
            self.last_parade_day = day;
            let troupes: Vec<usize> =
                (0..self.pens.len()).filter(|&i| is_paraders(SPECIES[self.species_of_pen[i]].name)).collect();
            if !troupes.is_empty() {
                let i = *cx.rng.pick(&troupes);
                self.parade_until = (i as f32 + 1.0, cx.time + 1.5);
                let name = SPECIES[self.species_of_pen[i]].name.to_lowercase();
                cx.news(format!("The daily parade begins: the {name} are marching!"), Some(self.pens[i].view));
                cx.add("rep", 3.0);
            }
        }

        // Popular parks end the evening with fireworks.
        self.show_cd -= dt;
        if self.show_cd <= 0.0 && cx.daylight < 0.25 && cx.global("rep") > 70.0 {
            self.show_cd = cx.rng.range(0.6, 2.0);
            let (x, y) = (cx.rng.range(10.0, cx.w - 10.0), cx.rng.range(5.0, cx.h / 2.0));
            firework(cx, x, y);
        }
    }
}
