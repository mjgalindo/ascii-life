use crate::engine::sprite::std_palette;
use crate::engine::*;
use ratatui::style::Color;

const WATER_Y: f32 = 1.0;

const FISH_ART: &[(&str, &str)] = &[
    ("><>", "222"),
    ("><))'>", "222221"),
    ("  __\n><_o>", "  22\n22221"),
    ("   ,\n><(((o>", "   2\n222221 "),
    ("  \\\n>=(==o>\n  /", "  2\n222221\n  2"),
    (" /\\\n<  o)>\n \\/", " 22\n2 21 2\n 22"),
];

const TINTS: &[char] = &['R', 'Y', 'C', 'G', 'M', 'O', 'B', 'W', 'p'];

fn random_fish(cx: &mut Ctx, right: bool) -> Sprite {
    let (shape, mask) = *cx.rng.pick(FISH_ART);
    let body = std_palette(*cx.rng.pick(TINTS)).unwrap();
    let eye = Color::Rgb(255, 255, 255);
    let mut digits = [Color::White; 9];
    digits[0] = eye;
    digits[1] = body;
    Sprite::art_d(shape, mask, &digits).facing(right)
}

struct Swimmer {
    phase: f32,
    wobble: f32,
}

impl Behavior for Swimmer {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, _dt: f32) {
        e.vel.1 = (e.age * 1.3 + self.phase).sin() * self.wobble;
        if e.pos.1 < WATER_Y + 3.0 {
            e.vel.1 = e.vel.1.abs();
        }
        if e.pos.1 > cx.h - 5.0 {
            e.vel.1 = -e.vel.1.abs();
        }
        if cx.rng.chance(0.004) {
            let right = e.vel.0 > 0.0;
            let bx = if right { e.pos.0 + e.w() } else { e.pos.0 - 1.0 };
            cx.spawn(bubble(bx, e.pos.1));
        }
    }
}

fn bubble(x: f32, y: f32) -> Entity {
    Entity::new("bubble", Sprite::mono(".", Color::Rgb(180, 220, 255)))
        .at(x, y)
        .vel(0.0, -4.0)
        .z(1)
        .with(Bubble)
}

struct Bubble;
impl Behavior for Bubble {
    fn update(&mut self, e: &mut Entity, _cx: &mut Ctx, _dt: f32) {
        if e.pos.1 <= WATER_Y + 1.0 {
            e.kill();
        } else if e.age > 0.7 && e.state == 0 {
            e.state = 1;
            e.set_sprite(Sprite::mono("o", Color::Rgb(200, 235, 255)));
        } else if e.age > 1.6 && e.state == 1 {
            e.state = 2;
            e.set_sprite(Sprite::mono("O", Color::Rgb(220, 245, 255)));
        }
    }
}

struct Shark;
impl Behavior for Shark {}

/// Hitbox glued to a shark's mouth; removes fish it touches.
struct Teeth;
impl Behavior for Teeth {
    fn on_collide(&mut self, _e: &mut Entity, other: &Info, cx: &mut Ctx) {
        if other.kind == "fish" {
            cx.kill(other.id);
            cx.add("eaten", 1.0);
            cx.spawn(bubble(other.pos.0, other.pos.1));
            cx.spawn(bubble(other.pos.0 + 1.0, other.pos.1 + 1.0));
        }
    }
}

struct Whale;
impl Behavior for Whale {
    fn update(&mut self, e: &mut Entity, _cx: &mut Ctx, _dt: f32) {
        e.vel.1 = (e.age * 0.5).sin() * 0.8;
    }
}

fn spawn_fish(cx: &mut Ctx, initial: bool) {
    let right = cx.rng.chance(0.5);
    let sprite = random_fish(cx, right);
    let w = sprite.w() as f32;
    let speed = cx.rng.range(3.0, 12.0);
    let x = if initial {
        cx.rng.range(0.0, cx.w)
    } else if right {
        -w
    } else {
        cx.w
    };
    let y = cx.rng.range(WATER_Y + 4.0, (cx.h - 6.0).max(WATER_Y + 5.0));
    let z = cx.rng.int(3, 20);
    let e = Entity::new("fish", sprite)
        .at(x, y)
        .vel(if right { speed } else { -speed }, 0.0)
        .z(z)
        .hitbox(0, 0, w as i32, 1)
        .die_offscreen()
        .with(Swimmer { phase: cx.rng.range(0.0, 6.0), wobble: cx.rng.range(0.2, 1.2) });
    cx.spawn(e);
}

fn spawn_special(cx: &mut Ctx) {
    let right = cx.rng.chance(0.5);
    let dir = if right { 1.0 } else { -1.0 };
    let low = (cx.h - 10.0).max(7.0);
    match cx.rng.int(0, 4) {
        0 => {
            let sprite = Sprite::mono(SHARK, Color::Rgb(130, 150, 175)).facing(right);
            let w = sprite.w() as f32;
            let x = if right { -w } else { cx.w };
            let y = cx.rng.range(6.0, low);
            let id = cx.spawn(
                Entity::new("special", sprite)
                    .at(x, y)
                    .vel(dir * 9.0, 0.0)
                    .z(4)
                    .die_offscreen()
                    .with(Shark),
            );
            let nose = if right { w - 4.0 } else { 0.0 };
            cx.spawn(
                Entity::new("teeth", Sprite::one(Frame::blank(1, 1)))
                    .hitbox(0, 0, 4, 3)
                    .follow(id, nose, 1.0)
                    .with(Teeth),
            );
        }
        1 => {
            let frames: Vec<Frame> = WHALE_SPOUT
                .iter()
                .map(|spout| {
                    let art = format!("{spout}{WHALE}");
                    Frame::parse(&art, None, Color::Rgb(70, 110, 190), &[Color::White; 9])
                })
                .collect();
            let sprite = Sprite::new(frames).facing(right);
            let w = sprite.w() as f32;
            cx.spawn(
                Entity::new("special", sprite)
                    .at(if right { -w } else { cx.w }, 0.0)
                    .vel(dir * 4.0, 0.0)
                    .z(5)
                    .fps(3.0)
                    .die_offscreen()
                    .with(Whale),
            );
        }
        2 => {
            let sprite = Sprite::art(SHIP, SHIP_MASK).facing(right);
            let w = sprite.w() as f32;
            cx.spawn(
                Entity::new("special", sprite)
                    .at(if right { -w } else { cx.w }, 0.0)
                    .vel(dir * 6.0, 0.0)
                    .z(2)
                    .die_offscreen(),
            );
        }
        _ => {
            let duck = Sprite::mono("_(o)>", Color::Rgb(240, 220, 120)).facing(right);
            let w = duck.w() as f32;
            let leader = cx.spawn(
                Entity::new("special", duck.clone())
                    .at(if right { -w } else { cx.w }, 0.0)
                    .vel(dir * 5.0, 0.0)
                    .z(2)
                    .die_offscreen(),
            );
            for i in 1..4 {
                let off = -dir * 7.0 * i as f32;
                cx.spawn(Entity::new("duck", duck.clone()).z(2).follow(leader, off, 0.0));
            }
        }
    }
}

const SHARK: &str = "        |\\\n   ___/  \\____\n  /_  _       \\>\n    `-.___.--'";
const WHALE: &str = "      .-----:\n    .'       `.\n ,_/      (o)   \\\n \\`-.___________/";
const WHALE_SPOUT: [&str; 3] = [
    "          \n          \n          \n",
    "          \n    .     \n    :     \n",
    "   . .    \n    \\|/   \n     |    \n",
];
const SHIP: &str = "     |    |    |\n    )_)  )_)  )_)\n   )___))___))___)\\\n   )____)____)_____)\\\\\n _____|____|____|____\\\\\\__\n \\                  /";
const SHIP_MASK: &str = "     W    W    W\n    WWW  WWW  WWW\n   WWWWWWWWWWWWWWWW\n   WWWWWWWWWWWWWWWWWWW\n nnnnnnnnnnnnnnnnnnnnnnnnn\n nnnnnnnnnnnnnnnnnn";

#[derive(Default)]
pub struct Aquarium {
    next_special: f32,
}

impl Scenario for Aquarium {
    fn name(&self) -> &'static str {
        "aquarium"
    }

    fn background(&self) -> Color {
        Color::Rgb(8, 40, 90)
    }

    fn setup(&mut self, cx: &mut Ctx) {
        for _ in 0..(cx.w * cx.h / 50.0) as i32 {
            spawn_fish(cx, true);
        }
    }

    fn tick(&mut self, cx: &mut Ctx, dt: f32) {
        if cx.snap.count("special") == 0 {
            self.next_special -= dt;
            if self.next_special <= 0.0 {
                spawn_special(cx);
                self.next_special = cx.rng.range(4.0, 14.0);
            }
        }
        let target = (cx.w * cx.h / 50.0) as usize;
        if cx.snap.count("fish") < target && cx.rng.chance(0.2) {
            spawn_fish(cx, false);
        }
    }
}
