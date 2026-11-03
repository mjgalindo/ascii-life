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
    base_vx: f32,
}

impl Behavior for Swimmer {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, _dt: f32) {
        let near_shark = cx
            .snap
            .query(e.pos.0 as i32 - 14, e.pos.1 as i32 - 5, 30, 11)
            .iter()
            .any(|o| o.kind == "special" && o.state == 1);
        e.vel.0 = if near_shark { self.base_vx * 2.6 } else { self.base_vx };
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
        .with(Swimmer {
            phase: cx.rng.range(0.0, 6.0),
            wobble: cx.rng.range(0.2, 1.2),
            base_vx: if right { speed } else { -speed },
        });
    cx.spawn(e);
}

fn spawn_special(cx: &mut Ctx) {
    let right = cx.rng.chance(0.5);
    let dir = if right { 1.0 } else { -1.0 };
    let low = (cx.h - 10.0).max(7.0);
    match cx.rng.int(0, 8) {
        0 => {
            let sprite = Sprite::mono(SHARK, Color::Rgb(130, 150, 175)).facing(right);
            let w = sprite.w() as f32;
            let x = if right { -w } else { cx.w };
            let y = cx.rng.range(6.0, low);
            let mut shark = Entity::new("special", sprite)
                .at(x, y)
                .vel(dir * 9.0, 0.0)
                .z(4)
                .die_offscreen()
                .with(Shark);
            shark.state = 1;
            let id = cx.spawn(shark);
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
        4 => {
            // Fishing hook that reels a fish up when it bites.
            let x = cx.rng.range(8.0, (cx.w - 8.0).max(9.0));
            let depth = cx.rng.range(10.0, (cx.h - 6.0).max(11.0));
            let mut hook = Entity::new("special", Sprite::mono("|\n|\nJ", Color::Rgb(220, 220, 220)))
                .at(x, -3.0)
                .vel(0.0, 7.0)
                .z(3)
                .hitbox(0, 2, 2, 2)
                .with(Hook { depth, wait: 0.0, caught: None });
            hook.data = 0.0;
            let id = cx.spawn(hook);
            let line = Frame::filled(1, 40, '|', Color::Rgb(200, 200, 200), None);
            cx.spawn(Entity::new("line", Sprite::one(line)).z(3).follow(id, 0.0, -38.0));
        }
        5 => {
            for i in 0..3 {
                let sprite = Sprite::mono(DOLPHIN, Color::Rgb(120, 150, 200)).facing(right);
                let w = sprite.w() as f32;
                let x = if right { -w - i as f32 * 10.0 } else { cx.w + i as f32 * 10.0 };
                let mut d = Entity::new("special", sprite)
                    .at(x, 3.0)
                    .vel(dir * 12.0, 0.0)
                    .z(2)
                    .die_offscreen()
                    .with(Dolphin { phase: i as f32 * 0.9 });
                d.data = 0.0;
                cx.spawn(d);
            }
        }
        6 => {
            let sprite = Sprite::mono(BIG_FISH, Color::Rgb(90, 140, 120)).facing(right);
            let w = sprite.w() as f32;
            let y = cx.rng.range(5.0, low);
            cx.spawn(
                Entity::new("special", sprite)
                    .at(if right { -w } else { cx.w }, y)
                    .vel(dir * 5.0, 0.0)
                    .z(6)
                    .die_offscreen()
                    .with(Whale),
            );
        }
        7 => {
            let sprite = Sprite::new(vec![
                Frame::parse(MONSTER_A, None, Color::Rgb(90, 190, 90), &[Color::White; 9]),
                Frame::parse(MONSTER_B, None, Color::Rgb(90, 190, 90), &[Color::White; 9]),
            ])
            .facing(right);
            let w = sprite.w() as f32;
            cx.spawn(
                Entity::new("special", sprite)
                    .at(if right { -w } else { cx.w }, 0.0)
                    .vel(dir * 6.0, 0.0)
                    .z(5)
                    .fps(3.0)
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

const DOLPHIN: &str = "      __\n  \\ _/ o\\\n   \\___/>";
const BIG_FISH: &str = "     ____\n  __/ o  \\_\n >__      __>\n    \\____/";
const MONSTER_A: &str = "      __      __\n  .-.(  )-.-(  )-.\n_/   ' '   ' '   o>";
const MONSTER_B: &str = "  __      __\n.(  )-.-(  )-.\n    ' '   ' '  o>";
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

    fn hud(&self, g: &Globals, _daylight: f32) -> String {
        let v = |k| g.get(k).copied().unwrap_or(0.0) as i32;
        format!("eaten by sharks {} | hooked {}", v("eaten"), v("caught"))
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

/// Fish confined to a glass tank; turns around at the walls.
struct TankFish {
    bounds: (f32, f32, f32, f32),
    sprites: [Sprite; 2],
    right: bool,
    speed: f32,
    phase: f32,
}

impl Behavior for TankFish {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, _dt: f32) {
        let b = self.bounds;
        if (self.right && e.pos.0 >= b.2) || (!self.right && e.pos.0 <= b.0) {
            self.right = !self.right;
            e.set_sprite(self.sprites[self.right as usize].clone());
        }
        e.vel.0 = if self.right { self.speed } else { -self.speed };
        e.vel.1 = (e.age * 1.1 + self.phase).sin() * 0.7;
        e.pos.1 = e.pos.1.clamp(b.1, b.3);
        if cx.rng.chance(0.004) {
            let bx = e.pos.0 + if self.right { e.w() } else { -1.0 };
            cx.spawn(
                Entity::new("bubble", Sprite::mono("o", Color::Rgb(200, 235, 255)))
                    .at(bx, e.pos.1)
                    .vel(0.0, -3.0)
                    .z(7)
                    .lifetime(((e.pos.1 - b.1) / 3.0).max(0.2)),
            );
        }
    }
}

/// Glass tank with sand, swaying weed and a handful of fish. `rect` is (x, y, w, h).
pub fn tank(cx: &mut Ctx, rect: (i32, i32, i32, i32)) {
    let (x, y, w, h) = rect;
    if w < 8 || h < 5 {
        return;
    }
    let water = Color::Rgb(14, 62, 120);
    let glass = Color::Rgb(160, 220, 235);
    let sand = Color::Rgb(170, 146, 96);
    let mut f = Frame::filled(w as usize, h as usize, ' ', glass, Some(water));
    for xx in 0..w as usize {
        f.set(xx, 0, '=', glass, Some(water));
        f.set(xx, h as usize - 1, '=', glass, Some(sand));
        f.set(xx, h as usize - 2, if xx % 3 == 0 { '.' } else { ' ' }, Color::Rgb(120, 100, 60), Some(sand));
    }
    for yy in 0..h as usize {
        f.set(0, yy, '|', glass, Some(water));
        f.set(w as usize - 1, yy, '|', glass, Some(water));
    }
    cx.spawn(Entity::new("tank", Sprite::one(f)).at(x as f32, y as f32).z(40));

    for _ in 0..(w / 6) {
        let wx = cx.rng.int(x + 2, x + w - 3) as f32;
        let weed = Sprite::anim(&[("(\n )", "g \n g"), (" (\n)", " g\ng ")]);
        let rate = 1.0 + cx.rng.f();
        cx.spawn(Entity::new("seaweed", weed).at(wx, (y + h - 4) as f32).z(8).fps(rate));
    }

    let fish = (w / 5).clamp(2, 6);
    for _ in 0..fish {
        let (shape, mask) = *cx.rng.pick(&FISH_ART[..4]);
        let body = std_palette(*cx.rng.pick(TINTS)).unwrap();
        let mut digits = [Color::White; 9];
        digits[1] = body;
        let right = Sprite::art_d(shape, mask, &digits);
        let sprites = [right.mirrored(), right.clone()];
        let fw = right.w() as f32;
        let fh = right.h() as f32;
        let bounds = ((x + 1) as f32, (y + 1) as f32, (x + w - 1) as f32 - fw, (y + h - 3) as f32 - fh);
        let start = (cx.rng.range(bounds.0, bounds.2), cx.rng.range(bounds.1, bounds.3.max(bounds.1 + 0.1)));
        let dir = cx.rng.chance(0.5);
        let speed = cx.rng.range(2.0, 6.0);
        let phase = cx.rng.range(0.0, 6.0);
        cx.spawn(
            Entity::new("tankfish", sprites[dir as usize].clone())
                .at(start.0, start.1)
                .z(7)
                .with(TankFish { bounds, sprites, right: dir, speed, phase }),
        );
    }
}

/// Descends, lingers, then reels up; a fish touching the hook is hauled out of the water.
struct Hook {
    depth: f32,
    wait: f32,
    caught: Option<Id>,
}

impl Behavior for Hook {
    fn update(&mut self, e: &mut Entity, _cx: &mut Ctx, dt: f32) {
        match e.state {
            0 => {
                if e.pos.1 >= self.depth {
                    e.vel.1 = 0.0;
                    e.state = 1;
                }
            }
            1 => {
                self.wait += dt;
                if self.wait > 4.0 || self.caught.is_some() {
                    e.state = 2;
                    e.vel.1 = -6.0;
                }
            }
            _ => {
                if e.pos.1 < -4.0 {
                    e.kill();
                }
            }
        }
    }

    fn on_collide(&mut self, e: &mut Entity, other: &Info, cx: &mut Ctx) {
        if other.kind == "fish" && self.caught.is_none() && e.state < 2 {
            cx.kill(other.id);
            let sprite = Sprite::mono("><>", Color::Rgb(255, 200, 80));
            let id = cx.spawn(Entity::new("caught", sprite).z(3).follow(e.id, -1.0, 3.0));
            self.caught = Some(id);
            cx.add("caught", 1.0);
        }
    }
}

struct Dolphin {
    phase: f32,
}

impl Behavior for Dolphin {
    fn update(&mut self, e: &mut Entity, _cx: &mut Ctx, _dt: f32) {
        // Leaps above the waterline in a repeating arc.
        let t = e.age * 2.2 + self.phase;
        e.pos.1 = 5.0 - 5.0 * t.sin().max(0.0);
    }
}
