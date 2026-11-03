use super::common::{floating_text, random_name, Spark};
use crate::engine::sprite::{std_palette, PixFrame};
use crate::engine::*;
use ratatui::style::Color;
use std::sync::Arc;

const WATER_Y: f32 = 1.0;

/// A species: two ASCII tail poses sharing one color mask, hand-drawn pixels, and how it behaves.
struct FishKind {
    name: &'static str,
    art: [&'static str; 2],
    mask: &'static str,
    pixels: &'static str,
    speed: f32,
    /// Relative abundance; small fish swim in large schools, big ones nearly alone.
    share: f32,
}

const FISH_KINDS: &[FishKind] = &[
    FishKind {
        name: "neon tetra",
        art: ["~=-o>", "-=-o>"],
        mask: "CBRWC",
        pixels: "C.BBkB\nCCRRRR",
        speed: 7.0,
        share: 2.2,
    },
    FishKind {
        name: "sardine",
        art: ["><o>", "}<o>"],
        mask: "wwWC",
        pixels: "s.CCk\nssSSS",
        speed: 6.5,
        share: 2.6,
    },
    FishKind {
        name: "clownfish",
        art: ["  ,_,_ \n>(|_|_o>", "  ,_,_ \n}(|_|_o>"],
        mask: "  OOOO \nOOWOWOWO",
        pixels: "....O.O..\nO.OWWOWOO\nOOOWWOWkO\nO..KOOOO.",
        speed: 4.0,
        share: 1.2,
    },
    FishKind {
        name: "blue tang",
        art: ["   __ \n>(/__o)", "   __ \n}(/__o)"],
        mask: "   BB \nYBKBBWB",
        pixels: "...BBB...\nY.BKKBBB.\nYYBBKKBkB\nY..BBBBB.",
        speed: 5.0,
        share: 1.0,
    },
    FishKind {
        name: "angelfish",
        art: ["  /|  \n>(|| o>\n  \\|  ", "  /|  \n}(|| o>\n  \\|  "],
        mask: "  YK  \nYYYKYWY\n  YK  ",
        pixels: "...KY..\n..YKYY.\nY.YKYYY\nYYYKYkY\nY.YKYY.\n...KY..",
        speed: 2.5,
        share: 0.6,
    },
    FishKind {
        name: "butterflyfish",
        art: [" ,-.\n>(|||o>\n `-'", " ,-.\n}(|||o>\n `-'"],
        mask: " YYY\nKYKYKWY\n YYY",
        pixels: "..YYY..\n.KYKYY.\nYKYKYkY\nYKYKYYY\n.KYKYY.\n..YYY..",
        speed: 3.0,
        share: 0.8,
    },
    FishKind {
        name: "pufferfish",
        art: [" .--. \n>( ::o)\n `--' ", " .--. \n}( ::o)\n `--' "],
        mask: " NNNN \nNNNNNWN\n WWWW ",
        pixels: "..TTTT..\n.TtTTtT.\nTTTTTTkT\nT.tTTTTT\n.WWWWWW.\n..WWWW..",
        speed: 1.8,
        share: 0.4,
    },
    FishKind {
        name: "barracuda",
        art: [">=======o=-", "}=======o=-"],
        mask: "KwwwwwwwWww",
        pixels: "s..sSSSSSSSkS\nss.SSSWWWWWWW",
        speed: 8.5,
        share: 0.4,
    },
];

const FISH_PALETTE: &[(char, Color)] = &[
    ('O', Color::Rgb(250, 130, 40)),
    ('W', Color::Rgb(248, 248, 244)),
    ('k', Color::Rgb(16, 16, 22)),
    ('K', Color::Rgb(34, 36, 50)),
    ('B', Color::Rgb(50, 110, 235)),
    ('Y', Color::Rgb(250, 215, 70)),
    ('R', Color::Rgb(235, 60, 70)),
    ('C', Color::Rgb(90, 225, 235)),
    ('S', Color::Rgb(190, 205, 215)),
    ('s', Color::Rgb(120, 135, 150)),
    ('T', Color::Rgb(205, 170, 95)),
    ('t', Color::Rgb(110, 84, 50)),
];

/// Second swim pose: the two tail columns kick up a pixel.
fn tail_kick(p: &PixFrame) -> PixFrame {
    let mut out = p.clone();
    for x in 0..p.w.min(2) {
        for y in 0..p.h {
            out.px[y * p.w + x] = p.px.get((y + 1) * p.w + x).copied().flatten();
        }
    }
    out
}

/// Left- and right-facing sprites for a species, each with a two-pose tail animation.
fn kind_sprites(kind: &FishKind) -> [Sprite; 2] {
    let frames = kind
        .art
        .iter()
        .map(|a| Frame::parse(a, Some(kind.mask), Color::White, &[Color::White; 9]))
        .collect();
    let px = PixFrame::parse(kind.pixels, FISH_PALETTE);
    let right = Sprite::new(frames).with_pixels(vec![tail_kick(&px), px]);
    [right.mirrored(), right]
}

/// Schooling fish: flocks with its breed, grazes plankton, flees sharks, breeds when fat.
struct Fish {
    sprites: [Sprite; 2],
    right: bool,
    speed: f32,
    energy: f32,
    think: f32,
    steer: (f32, f32),
    /// Direct push away from overlapping neighbors, in cells per second.
    shove: (f32, f32),
    fleeing: bool,
    name: &'static str,
    floor: Arc<Vec<f32>>,
}

impl Fish {
    fn decide(&mut self, e: &mut Entity, cx: &mut Ctx) {
        let (x, y) = (e.pos.0, e.pos.1);
        let around = cx.snap.query(x as i32 - 18, y as i32 - 8, 36, 16);
        let (mut sep, mut ali, mut coh, mut mates) = ((0.0f32, 0.0f32), (0.0f32, 0.0f32), (0.0f32, 0.0f32), 0.0f32);
        let mut flee: Option<(f32, f32)> = None;
        let mut snack: Option<(Id, (f32, f32))> = None;
        let (own_w, own_h) = (e.w(), e.h());
        let mut shove = (0.0f32, 0.0f32);
        for o in &around {
            let (dx, dy) = (o.pos.0 - x, (o.pos.1 - y) * 2.0);
            // Personal space scales with both sprites, so every fish stays readable.
            if o.kind == "fish" && o.id != e.id {
                let gap_x = (own_w + o.rect.2 as f32) / 2.0 + 1.5;
                let gap_y = own_h.max(1.0) + 1.0;
                let cdx = o.pos.0 + o.rect.2 as f32 / 2.0 - (x + own_w / 2.0);
                let cdy = o.pos.1 - y;
                let (ox, oy) = (1.0 - cdx.abs() / gap_x, 1.0 - cdy.abs() / gap_y);
                if ox > 0.0 && oy > 0.0 {
                    let side = |d: f32, id: bool| if d > 0.0 || (d == 0.0 && id) { -1.0 } else { 1.0 };
                    let lower = e.id < o.id;
                    // Slide apart along the axis needing the shorter move, usually vertical.
                    if oy * gap_y <= ox * gap_x {
                        shove.1 += side(cdy, lower) * oy * gap_y;
                    } else {
                        shove.0 += side(cdx, lower) * ox * gap_x;
                    }
                }
            }
            match o.kind {
                "shark" => flee = Some((-dx, -dy)),
                "plankton" if snack.is_none() && self.energy < 1.4 => snack = Some((o.id, o.pos)),
                "fish" if o.tags == e.tags && o.id != e.id && mates < 10.0 => {
                    mates += 1.0;
                    ali = (ali.0 + o.vel.0, ali.1 + o.vel.1);
                    coh = (coh.0 + dx, coh.1 + dy);
                    let d2 = (dx * dx + dy * dy).max(0.5);
                    if d2 < 100.0 {
                        sep = (sep.0 - dx / d2, sep.1 - dy / d2);
                    }
                }
                _ => {}
            }
        }
        let mut s = (e.vel.0, e.vel.1 * 2.0);
        if mates > 0.0 {
            s.0 += ali.0 / mates * 0.5 + coh.0 / mates * 0.05 + sep.0 * 14.0;
            s.1 += ali.1 / mates * 0.5 + coh.1 / mates * 0.05 + sep.1 * 14.0;
        }
        if let Some((id, p)) = snack {
            let (dx, dy) = (p.0 - x, (p.1 - y) * 2.0);
            // Hungrier fish break formation to feed.
            let pull = if self.energy < 0.6 { 1.2 } else { 0.4 };
            s.0 += dx * pull;
            s.1 += dy * pull;
            if dx * dx + dy * dy < 9.0 {
                cx.kill(id);
                self.energy += 0.22;
            }
        }
        // Walls and water limits turn the school around rather than letting it leave.
        let floor = self.floor.get(x.max(0.0) as usize).copied().unwrap_or(cx.h);
        if x < 6.0 {
            s.0 += 8.0;
        } else if x > cx.w - 12.0 {
            s.0 -= 8.0;
        }
        if y < WATER_Y + 4.0 {
            s.1 += 6.0;
        } else if y > floor - 5.0 {
            s.1 -= 8.0;
        }
        self.shove = shove;
        self.fleeing = flee.is_some();
        if let Some(f) = flee {
            s = (s.0 + f.0 * 2.0, s.1 + f.1 * 2.0);
        }
        s.0 += cx.rng.range(-1.0, 1.0);
        s.1 += cx.rng.range(-0.6, 0.6);
        let len = (s.0 * s.0 + s.1 * s.1).sqrt().max(0.01);
        let v = self.speed * if self.fleeing { 1.3 } else { 1.0 };
        self.steer = (s.0 / len * v, s.1 / len * v * 0.5);

        if self.energy > 1.5 && cx.global("fish") < cx.global("fish_cap") {
            self.energy -= 0.7;
            let (sprites, speed, floor, tags) = (self.sprites.clone(), self.speed, self.floor.clone(), e.tags);
            let name = random_name(cx.rng);
            spawn_school_member(cx, sprites, speed, floor, tags, x, y, name);
        }
    }
}

impl Behavior for Fish {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, dt: f32) {
        self.energy -= dt * 0.006;
        if self.energy <= 0.0 {
            cx.spawn(
                Entity::new("bones", Sprite::mono(">=<", Color::Rgb(220, 220, 210)))
                    .at(e.pos.0, e.pos.1)
                    .vel(0.0, 2.0)
                    .z(e.z)
                    .lifetime(5.0),
            );
            cx.add("starved", 1.0);
            e.kill();
            return;
        }
        self.think -= dt;
        if self.think <= 0.0 {
            self.think = cx.rng.range(0.15, 0.4);
            self.decide(e, cx);
        }
        let k = (dt * 3.0).min(1.0);
        e.vel.0 += (self.steer.0 - e.vel.0) * k;
        e.vel.1 += (self.steer.1 - e.vel.1) * k;
        e.pos.0 = (e.pos.0 + self.shove.0 * dt * 3.0).clamp(0.0, cx.w - e.w());
        e.pos.1 = (e.pos.1 + self.shove.1 * dt * 3.0).clamp(WATER_Y + 2.0, cx.h - e.h());
        let right = e.vel.0 > 0.0;
        if right != self.right {
            self.right = right;
            e.set_sprite(self.sprites[right as usize].clone());
        }
        if cx.rng.chance(0.002) {
            let bx = if right { e.pos.0 + e.w() } else { e.pos.0 - 1.0 };
            cx.spawn(bubble(bx, e.pos.1));
        }
    }

    fn describe(&self, e: &Entity) -> Option<String> {
        let mood = if self.fleeing { "fleeing!" } else if self.energy > 1.0 { "well fed" } else if self.energy < 0.3 { "starving" } else { "grazing" };
        let species = FISH_KINDS[e.tags as usize].name;
        Some(format!("{} the {species}\n{mood}, energy {:.0}%", self.name, self.energy * 100.0))
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_school_member(cx: &mut Ctx, sprites: [Sprite; 2], speed: f32, floor: Arc<Vec<f32>>, tags: u32, x: f32, y: f32, name: &'static str) {
    let right = cx.rng.chance(0.5);
    let sprite = sprites[right as usize].clone();
    let w = sprite.w() as i32;
    let z = cx.rng.int(3, 20);
    let energy = cx.rng.range(0.5, 1.0);
    let think = cx.rng.range(0.0, 0.4);
    let e = Entity::new("fish", sprite)
        .at(x, y)
        .vel(if right { speed } else { -speed }, 0.0)
        .z(z)
        .tags(tags)
        .fps(speed * 0.8)
        .hitbox(0, 0, w, 1)
        .with(Fish { sprites, right, speed, energy, think, steer: (0.0, 0.0), shove: (0.0, 0.0), fleeing: false, name, floor });
    cx.spawn(e);
}

/// Hunts the nearest fish until full, then leaves the map.
struct Shark {
    sprites: [Sprite; 2],
    right: bool,
    meals: u32,
    /// Meals and seconds before the shark loses interest and swims away.
    appetite: u32,
    patience: f32,
    think: f32,
    prey: Option<Id>,
    name: &'static str,
}

impl Behavior for Shark {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, dt: f32) {
        self.think -= dt;
        let leaving = self.meals >= self.appetite || e.age > self.patience;
        if self.think <= 0.0 {
            self.think = 0.3;
            if leaving {
                self.prey = None;
                e.die_offscreen = true;
            } else if cx.snap.get(self.prey.unwrap_or(0)).is_none() {
                let (x, y) = (e.cx() as i32, e.cy() as i32);
                let dist = |o: &Info| (o.pos.0 - x as f32).abs() + (o.pos.1 - y as f32).abs() * 2.0;
                self.prey = cx
                    .snap
                    .query(x - 45, y - 18, 90, 36)
                    .into_iter()
                    .filter(|o| o.kind == "fish")
                    .min_by(|a, b| dist(a).total_cmp(&dist(b)))
                    .map(|o| o.id);
            }
        }
        let prey = self.prey.and_then(|id| cx.snap.get(id));
        let (mut vx, mut vy) = if self.right { (9.0, 0.0) } else { (-9.0, 0.0) };
        if let Some(p) = prey {
            let (dx, dy) = (p.pos.0 - e.cx(), p.pos.1 - e.cy());
            let d = (dx * dx + dy * dy * 4.0).sqrt().max(0.1);
            vx = dx / d * 18.0;
            vy = dy / d * 9.0;
            if d < 5.0 {
                cx.kill(p.id);
                self.meals += 1;
                self.prey = None;
                cx.add("eaten", 1.0);
                cx.spawn(floating_text("CHOMP", Color::Rgb(255, 150, 150), p.pos.0, p.pos.1 - 1.0));
                cx.spawn(bubble(p.pos.0, p.pos.1));
            }
        } else if !leaving {
            // Patrol: turn back at the walls while nothing is in sight.
            if e.pos.0 < 2.0 {
                self.right = true;
            } else if e.pos.0 > cx.w - e.w() - 2.0 {
                self.right = false;
            }
        }
        e.vel = (vx, vy);
        self.right = vx > 0.0;
        let want = &self.sprites[self.right as usize];
        if !Arc::ptr_eq(&e.sprite.frames, &want.frames) {
            e.set_sprite(want.clone());
        }
    }

    fn describe(&self, _e: &Entity) -> Option<String> {
        Some(format!("{} the shark\n{} fish eaten", self.name, self.meals))
    }
}

/// Pulses upward and sinks, drifting with the current.
struct Jelly {
    phase: f32,
}
impl Behavior for Jelly {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, _dt: f32) {
        let beat = ((e.age + self.phase) * 1.6).sin();
        e.vel = ((e.age * 0.2 + self.phase).sin() * 0.8, if beat > 0.5 { -2.4 } else { 0.7 });
        if e.pos.1 < WATER_Y + 4.0 {
            e.vel.1 = 1.0;
        }
        if e.pos.1 > cx.h * 0.8 {
            e.vel.1 = -2.4;
        }
    }
}

/// Scuttles along the sea floor, pausing to wave its claws.
struct Crab {
    floor: Arc<Vec<f32>>,
    cd: f32,
}
impl Behavior for Crab {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, dt: f32) {
        self.cd -= dt;
        if self.cd <= 0.0 {
            self.cd = cx.rng.range(1.0, 5.0);
            e.vel.0 = *cx.rng.pick(&[-3.5, 0.0, 3.5]);
        }
        if e.pos.0 < 1.0 {
            e.vel.0 = 3.5;
        } else if e.pos.0 > cx.w - 6.0 {
            e.vel.0 = -3.5;
        }
        let f = self.floor.get(e.pos.0.max(0.0) as usize).copied().unwrap_or(cx.h);
        e.pos.1 = f - 1.0;
    }
}

/// Deep lurker whose lure blinks on and off.
struct Angler {
    home: f32,
}
impl Behavior for Angler {
    fn update(&mut self, e: &mut Entity, _cx: &mut Ctx, _dt: f32) {
        e.vel.1 = (self.home - e.pos.1) * 0.3;
        e.vel.0 = (e.age * 0.15).sin() * 1.5;
    }
}

/// Merchant ship that springs a leak and goes down to become a reef.
struct Sinker {
    floor: Arc<Vec<f32>>,
    sinking: bool,
    sink_at: f32,
    hulk: Sprite,
}
impl Behavior for Sinker {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, _dt: f32) {
        if !self.sinking && e.age > self.sink_at {
            self.sinking = true;
            e.die_offscreen = false;
            e.vel = (e.vel.0 * 0.15, 2.6);
            cx.news("MAYDAY! A ship is going down!", Some(e.pos));
            cx.add("wrecks", 1.0);
        }
        if self.sinking {
            if cx.rng.chance(0.3) {
                let dx = cx.rng.range(0.0, e.w());
                cx.spawn(bubble(e.pos.0 + dx, e.pos.1));
            }
            let f = self.floor.get(e.cx().max(0.0) as usize).copied().unwrap_or(cx.h);
            if e.pos.1 + e.h() >= f {
                cx.spawn(Entity::new("seaweed", self.hulk.clone()).at(e.pos.0, f - self.hulk.h() as f32 + 1.0).z(70));
                cx.spawn(
                    Entity::new("chest", Sprite::mono("[$]", Color::Rgb(255, 210, 90)))
                        .at(e.pos.0 + e.w() + 1.0, f - 1.0)
                        .z(30)
                        .with(Chest { cd: 5.0 }),
                );
                cx.news("The wreck settles on the sea bed. There is treasure aboard.", Some(e.pos));
                e.kill();
            }
        }
    }
}

/// Burps bubbles now and then.
struct Chest {
    cd: f32,
}
impl Behavior for Chest {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, dt: f32) {
        self.cd -= dt;
        if self.cd <= 0.0 {
            self.cd = cx.rng.range(4.0, 12.0);
            for i in 0..4 {
                cx.spawn(bubble(e.pos.0 + 1.0, e.pos.1 - i as f32));
            }
        }
    }
}

/// Swims down to a chest, grabs the gold and heads back up, leaving a stream of bubbles.
struct Diver {
    goal: (f32, f32),
    has_gold: bool,
}
impl Behavior for Diver {
    fn update(&mut self, e: &mut Entity, cx: &mut Ctx, _dt: f32) {
        let to = if self.has_gold { (e.pos.0, -6.0) } else { self.goal };
        let (dx, dy) = (to.0 - e.pos.0, to.1 - e.pos.1);
        let d = (dx * dx + dy * dy * 4.0).sqrt().max(0.1);
        e.vel = (dx / d * 5.0, dy / d * 2.5);
        if d < 2.0 && !self.has_gold {
            self.has_gold = true;
            cx.news("A diver surfaces with gold from the wreck!", Some(e.pos));
            cx.add("treasure", 1.0);
            e.die_offscreen = true;
        }
        if cx.rng.chance(0.05) {
            cx.spawn(bubble(e.pos.0, e.pos.1));
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

struct Whale;
impl Behavior for Whale {
    fn update(&mut self, e: &mut Entity, _cx: &mut Ctx, _dt: f32) {
        e.vel.1 = (e.age * 0.5).sin() * 0.8;
    }
}

fn spawn_special(cx: &mut Ctx, floor: &Arc<Vec<f32>>) {
    let right = cx.rng.chance(0.5);
    let dir = if right { 1.0 } else { -1.0 };
    let low = (cx.h - 10.0).max(7.0);
    match cx.rng.int(0, 8) {
        0 => {
            let sub = Sprite::mono(SUBMARINE, Color::Rgb(220, 190, 70)).facing(right);
            let w = sub.w() as f32;
            let y = cx.rng.range(10.0, low);
            cx.spawn(
                Entity::new("special", sub)
                    .at(if right { -w } else { cx.w }, y)
                    .vel(dir * 5.0, 0.0)
                    .z(4)
                    .die_offscreen()
                    .with(Whale),
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
            let (ww, wh) = (frames[0].w as i32, frames[0].h as i32);
            let pix: Vec<_> = (0..3).map(|i| crate::art::whale(ww, wh, 6, i).frame()).collect();
            let sprite = Sprite::new(frames).with_pixels(pix).facing(right);
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
            let base = Sprite::art(SHIP, SHIP_MASK);
            let (sw, sh) = (base.w() as i32, base.h() as i32);
            let sprite = base.with_pixels(vec![crate::art::ship(sw, sh).frame()]).facing(right);
            let w = sprite.w() as f32;
            let mut ship = Entity::new("special", sprite)
                .at(if right { -w } else { cx.w }, 0.0)
                .vel(dir * 6.0, 0.0)
                .z(2)
                .die_offscreen();
            if cx.rng.chance(0.3) {
                let sink_at = cx.rng.range(5.0, cx.w / 6.0);
                let hulk = Sprite::mono(WRECK, Color::Rgb(96, 84, 70));
                ship = ship.with(Sinker { floor: floor.clone(), sinking: false, sink_at, hulk });
            }
            cx.spawn(ship);
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
                let base = Sprite::mono(DOLPHIN, Color::Rgb(120, 150, 200));
                let (dw, dh) = (base.w() as i32, base.h() as i32);
                let sprite = base.with_pixels(vec![crate::art::dolphin(dw, dh).frame()]).facing(right);
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
            let base = Sprite::mono(BIG_FISH, Color::Rgb(90, 140, 120));
            let (bw, bh) = (base.w() as i32, base.h() as i32);
            let sprite = base
                .with_pixels(vec![crate::art::fish(bw, bh, Color::Rgb(96, 150, 128), true).frame()])
                .facing(right);
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
            let fa = Frame::parse(MONSTER_A, None, Color::Rgb(90, 190, 90), &[Color::White; 9]);
            let fb = Frame::parse(MONSTER_B, None, Color::Rgb(90, 190, 90), &[Color::White; 9]);
            let (mw, mh) = (fa.w as i32, fa.h as i32);
            let sprite = Sprite::new(vec![fa, fb])
                .with_pixels(vec![
                    crate::art::sea_monster(mw, mh, 0.0).frame(),
                    crate::art::sea_monster(mw, mh, 2.0).frame(),
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
            let duck = Sprite::mono("_(o)>", Color::Rgb(240, 220, 120));
            let duck = duck.with_pixels(vec![crate::art::duck(5, 1).frame()]).facing(right);
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

const SUBMARINE: &str = "    __|__\n __/  o  \\___\n(_o__o__o____)>";
const WRECK: &str = "    |  /\n  __|_/___\n /  x    x\\__\n/_%___x___%__/";
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
    floor: Arc<Vec<f32>>,
    shark_cd: f32,
    diver_cd: f32,
    trend_cd: f32,
    fish_trend: Spark,
    plankton_trend: Spark,
}

impl Aquarium {
    /// Water that darkens with depth, a wavy surface and a sandy, uneven bottom.
    fn backdrop(&self, w: i32, h: i32) -> Frame {
        let mut f = Frame::blank(w as usize, h as usize);
        for y in 0..h {
            let t = y as f32 / h as f32;
            let water = rgb((10.0 - 8.0 * t) as u8, (70.0 - 54.0 * t) as u8, (130.0 - 86.0 * t) as u8);
            for x in 0..w {
                let (ux, uy) = (x as usize, y as usize);
                let bed = self.floor[ux];
                if y as f32 >= bed {
                    let speck = (x * 7 + y * 13) % 11 == 0;
                    let sand = rgb(134, 118, 82);
                    f.set(ux, uy, if speck { '.' } else { ' ' }, rgb(100, 86, 60), Some(sand));
                } else if y <= 1 {
                    let c = if (x + y) % 5 == 0 { '~' } else { ' ' };
                    f.set(ux, uy, c, rgb(200, 230, 255), Some(rgb(34, 104, 168)));
                } else {
                    let caustic = y < h / 4 && (x * 3 + y * 7) % 37 == 0;
                    f.set(ux, uy, if caustic { '`' } else { ' ' }, rgb(90, 150, 200), Some(water));
                }
            }
        }
        f
    }

    fn spawn_kind_member(&self, cx: &mut Ctx, tags: u32) {
        let kind = &FISH_KINDS[tags as usize];
        let (x, y) = (cx.rng.range(4.0, cx.w - 8.0), cx.rng.range(WATER_Y + 5.0, cx.h * 0.75));
        let name = random_name(cx.rng);
        spawn_school_member(cx, kind_sprites(kind), kind.speed, self.floor.clone(), tags, x, y, name);
    }
}

impl Scenario for Aquarium {
    fn name(&self) -> &'static str {
        "aquarium"
    }

    fn fits_terminal(&self) -> bool {
        true
    }

    fn tools(&self) -> &'static [&'static str] {
        &["hungry shark", "school of fish", "plankton bloom", "jellyfish", "crab", "doomed ship"]
    }

    fn place(&mut self, cx: &mut Ctx, tool: usize, x: f32, y: f32) {
        match tool {
            0 => {
                // A released shark stays until it has done real damage.
                let name = spawn_shark(cx, x - 8.0, y - 2.0, 40, 400.0);
                cx.news(format!("You released {name} the shark. Good luck, fish."), Some((x, y)));
            }
            1 => {
                let tag = cx.rng.int(0, FISH_KINDS.len() as i32) as u32;
                let kind = &FISH_KINDS[tag as usize];
                let sprites = kind_sprites(kind);
                for _ in 0..12 {
                    let (fx, fy) = (
                        (x + cx.rng.range(-10.0, 10.0)).clamp(1.0, cx.w - 12.0),
                        (y + cx.rng.range(-4.0, 4.0)).clamp(WATER_Y + 3.0, cx.h - 6.0),
                    );
                    let name = random_name(cx.rng);
                    spawn_school_member(cx, sprites.clone(), kind.speed, self.floor.clone(), tag, fx, fy, name);
                }
                cx.news(format!("You added a school of {}.", kind.name), Some((x, y)));
            }
            2 => {
                for _ in 0..60 {
                    let (px, py) = (x + cx.rng.range(-18.0, 18.0), y + cx.rng.range(-7.0, 7.0));
                    spawn_plankton(cx, px, py.max(WATER_Y + 2.0));
                }
            }
            3 => spawn_jelly(cx, x, y),
            4 => spawn_crab(cx, x, self.floor.clone()),
            _ => {
                let base = Sprite::art(SHIP, SHIP_MASK);
                let (sw, sh) = (base.w() as i32, base.h() as i32);
                let sprite = base.with_pixels(vec![crate::art::ship(sw, sh).frame()]);
                let hulk = Sprite::mono(WRECK, Color::Rgb(96, 84, 70));
                let sinker = Sinker { floor: self.floor.clone(), sinking: false, sink_at: 1.5, hulk };
                cx.spawn(Entity::new("special", sprite).at(x - sw as f32 / 2.0, 0.0).vel(3.0, 0.0).z(2).with(sinker));
            }
        }
    }

    fn background(&self) -> Color {
        Color::Rgb(8, 40, 90)
    }

    fn day_length(&self) -> Option<f32> {
        Some(240.0)
    }

    fn hud(&self, g: &Globals, _daylight: f32) -> String {
        let v = |k| g.get(k).copied().unwrap_or(0.0) as i32;
        format!(
            "fish {} {} plankton {} {} sharks {} | eaten {} hooked {} starved {} wrecks {} gold {}",
            v("fish"),
            self.fish_trend.render(),
            v("plankton"),
            self.plankton_trend.render(),
            v("sharks"),
            v("eaten"),
            v("caught"),
            v("starved"),
            v("wrecks"),
            v("treasure"),
        )
    }

    fn setup(&mut self, cx: &mut Ctx) {
        let (w, h) = (cx.w as i32, cx.h as i32);
        let (a, b, c) = (cx.rng.range(0.0, 6.0), cx.rng.range(0.0, 6.0), cx.h);
        let floor: Vec<f32> = (0..w)
            .map(|x| {
                let x = x as f32;
                (c - 5.0 - 2.5 * (x * 0.031 + a).sin() - 1.5 * (x * 0.087 + b).sin()).round()
            })
            .collect();
        self.floor = Arc::new(floor);
        cx.spawn(Entity::new("ground", Sprite::one(self.backdrop(w, h))).z(100));
        cx.set("fish_cap", cx.w * cx.h / FISH_DENSITY);

        // Forests of swaying kelp and little coral heads on the bottom.
        for _ in 0..(w / 4) {
            let x = cx.rng.int(0, w - 2);
            let tall = cx.rng.int(3, (h / 3).clamp(4, 11)) as usize;
            let (s1, m1): (Vec<&str>, Vec<&str>) = (0..tall).map(|i| if i % 2 == 0 { ("(", "g") } else { (" )", " g") }).unzip();
            let (s2, m2): (Vec<&str>, Vec<&str>) = (0..tall).map(|i| if i % 2 == 0 { (" )", " g") } else { ("(", "g") }).unzip();
            let (a1, a2) = (s1.join("\n"), m1.join("\n"));
            let (b1, b2) = (s2.join("\n"), m2.join("\n"));
            let weed = Sprite::anim(&[(&a1, &a2), (&b1, &b2)]);
            let y = self.floor[x as usize] - tall as f32;
            let rate = cx.rng.range(0.7, 1.6);
            cx.spawn(Entity::new("seaweed", weed).at(x as f32, y).z(cx_depth(x)).fps(rate));
        }
        let coral = ["\\|/", "(@)", "/Y\\", "{#}", ".o."];
        let tints = ['R', 'p', 'O', 'M', 'Y'];
        for _ in 0..(w / 7) {
            let x = cx.rng.int(0, w - 4);
            let art = *cx.rng.pick(&coral);
            let tint = std_palette(*cx.rng.pick(&tints)).unwrap();
            let y = self.floor[x as usize] - 1.0;
            cx.spawn(Entity::new("seaweed", Sprite::mono(art, tint)).at(x as f32, y).z(60));
        }

        // Each species swims as its own school, sized by how common it is.
        let total: f32 = FISH_KINDS.iter().map(|k| k.share).sum();
        let stock = cx.w * cx.h / FISH_DENSITY * 0.7;
        for (tag, kind) in FISH_KINDS.iter().enumerate() {
            let sprites = kind_sprites(kind);
            let n = (stock * kind.share / total).round().max(1.0) as i32;
            let home = (cx.rng.range(10.0, cx.w - 14.0), cx.rng.range(5.0, cx.h * 0.65));
            for _ in 0..n {
                let x = (home.0 + cx.rng.range(-14.0, 14.0)).clamp(1.0, cx.w - 12.0);
                let y = home.1 + cx.rng.range(-4.0, 4.0);
                let name = random_name(cx.rng);
                spawn_school_member(cx, sprites.clone(), kind.speed, self.floor.clone(), tag as u32, x, y, name);
            }
        }

        for _ in 0..(w / 30).max(2) {
            let x = cx.rng.range(2.0, cx.w - 8.0);
            spawn_crab(cx, x, self.floor.clone());
        }
        for _ in 0..(w / 40).max(1) {
            let (x, y) = (cx.rng.range(5.0, cx.w - 8.0), cx.rng.range(cx.h * 0.2, cx.h * 0.7));
            spawn_jelly(cx, x, y);
        }
        for _ in 0..(w / 70).max(1) {
            let x = cx.rng.range(5.0, cx.w - 12.0);
            let home = cx.h - cx.rng.range(10.0, 16.0);
            let angler = Sprite::anim(&[("  ,o\n<(@ )=<", "  YY\nKKKKKKK"), ("  ,.\n<(@ )=<", "  KK\nKKKKKKK")]);
            cx.spawn(Entity::new("angler", angler).at(x, home).z(12).fps(1.2).with(Angler { home }));
        }
        self.shark_cd = 40.0;
        self.diver_cd = 30.0;
        self.next_special = 5.0;
    }

    fn tick(&mut self, cx: &mut Ctx, dt: f32) {
        let fish = cx.snap.count("fish");
        let plankton = cx.snap.count("plankton");
        cx.set("fish", fish as f32);
        cx.set("plankton", plankton as f32);
        cx.set("sharks", cx.snap.count("shark") as f32);

        self.trend_cd -= dt;
        if self.trend_cd <= 0.0 {
            self.trend_cd = 15.0;
            self.fish_trend.push(fish as f32);
            self.plankton_trend.push(plankton as f32);
        }

        // Sunlight drives the food chain: plankton blooms near the surface by day.
        let cap = (cx.w * cx.h / 60.0) as usize;
        let rate = (cx.w * cx.h / 2000.0) * (0.15 + cx.daylight);
        if plankton < cap {
            for _ in 0..rate as i32 + cx.rng.chance(rate.fract()) as i32 {
                let (x, y) = (cx.rng.range(0.0, cx.w), cx.rng.range(WATER_Y + 3.0, cx.h * 0.8));
                spawn_plankton(cx, x, y);
            }
        }

        // Plenty of fish draws sharks; a collapsed population sends them elsewhere.
        self.shark_cd -= dt;
        if self.shark_cd <= 0.0 {
            self.shark_cd = cx.rng.range(15.0, 45.0);
            // Natural sharks only come to a healthy population, so small tanks can recover.
            let healthy = fish as f32 > cx.global("fish_cap") * 0.6;
            if healthy && cx.snap.count("shark") < (fish / 90).max(1) {
                let from_left = cx.rng.chance(0.5);
                let x = if from_left { -30.0 } else { cx.w };
                let y = cx.rng.range(6.0, cx.h - 18.0);
                let name = spawn_shark(cx, x, y, 6, 150.0);
                cx.news(format!("{name} the shark has picked up a scent."), Some((x.max(0.0), y)));
            }
        }

        // Schools that died out reseed from open water so every breed keeps a chance.
        if fish < (cx.w * cx.h / 400.0) as usize && cx.rng.chance(0.2) {
            let tag = cx.rng.int(0, FISH_KINDS.len() as i32) as u32;
            self.spawn_kind_member(cx, tag);
        }

        // Treasure attracts divers.
        self.diver_cd -= dt;
        if self.diver_cd <= 0.0 {
            self.diver_cd = cx.rng.range(30.0, 90.0);
            let chests: Vec<(f32, f32)> = cx.snap.of_kind("chest").map(|c| c.pos).collect();
            if !chests.is_empty() && cx.snap.count("diver") == 0 {
                let goal = *cx.rng.pick(&chests);
                let diver = Sprite::mono(" o\n/|=\n/ \\", Color::Rgb(230, 200, 90));
                cx.spawn(Entity::new("diver", diver).at(goal.0, -3.0).z(7).with(Diver { goal, has_gold: false }));
            }
        }

        if cx.snap.count("special") < 3 {
            self.next_special -= dt;
            if self.next_special <= 0.0 {
                let floor = self.floor.clone();
                spawn_special(cx, &floor);
                self.next_special = cx.rng.range(4.0, 14.0);
            }
        }
    }
}

/// Returns the name given to the new shark.
fn spawn_shark(cx: &mut Ctx, x: f32, y: f32, appetite: u32, patience: f32) -> &'static str {
    let base = Sprite::mono(SHARK, Color::Rgb(130, 150, 175));
    let (sw, sh) = (base.w() as i32, base.h() as i32);
    let right = base.with_pixels(vec![crate::art::shark(sw, sh).frame()]);
    let sprites = [right.mirrored(), right];
    let facing = cx.rng.chance(0.5);
    let name = random_name(cx.rng);
    let behavior = Shark { sprites: sprites.clone(), right: facing, meals: 0, appetite, patience, think: 0.0, prey: None, name };
    cx.spawn(Entity::new("shark", sprites[facing as usize].clone()).at(x, y).z(4).with(behavior));
    name
}

fn spawn_plankton(cx: &mut Ctx, x: f32, y: f32) {
    let tint = Color::Rgb(70, 150, 105);
    let dot = Sprite::mono(if cx.rng.chance(0.5) { "." } else { "," }, tint)
        .with_pixels(vec![PixFrame { w: 1, h: 1, px: vec![Some(tint)] }]);
    let drift = (cx.rng.range(-0.5, 0.5), cx.rng.range(0.1, 0.5));
    cx.spawn(Entity::new("plankton", dot).at(x, y).vel(drift.0, drift.1).z(22).lifetime(70.0));
}

fn spawn_jelly(cx: &mut Ctx, x: f32, y: f32) {
    let phase = cx.rng.range(0.0, 6.0);
    let jelly = Sprite::anim(&[(" .-. \n(   )\n ||| ", " MMM \nMMMMM\n MMM "), (" .-. \n(   )\n/ | \\", " MMM \nMMMMM\nM M M")]);
    cx.spawn(Entity::new("jelly", jelly).at(x, y).z(9).fps(1.6).with(Jelly { phase }));
}

fn spawn_crab(cx: &mut Ctx, x: f32, floor: Arc<Vec<f32>>) {
    let cd = cx.rng.range(0.0, 3.0);
    let crab = Sprite::anim(&[("V(..)V", "RRRRRR"), ("v(..)v", "RRRRRR")]);
    let y = cx.h - 6.0;
    cx.spawn(Entity::new("crab", crab).at(x, y).z(25).fps(3.0).with(Crab { floor, cd }));
}

/// World cells per fish at the population cap.
const FISH_DENSITY: f32 = 45.0;

/// Mixes kelp in front of and behind the swimming fish.
fn cx_depth(x: i32) -> i32 {
    if x % 3 == 0 { 14 } else { 60 }
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
        // Only the species small enough to look right behind glass.
        let kind = &FISH_KINDS[cx.rng.int(0, 4) as usize];
        let sprites = kind_sprites(kind);
        let right = sprites[1].clone();
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
                .fps(4.0)
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

    fn wants_collisions(&self) -> bool {
        true
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
