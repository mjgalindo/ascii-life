//! Pieces shared by several worlds: names, weather, fireworks and sparklines.

use crate::engine::sprite::PixFrame;
use crate::engine::*;
use ratatui::style::Color;

const NAMES: &[&str] = &[
    "Ada", "Bram", "Cleo", "Dot", "Edna", "Finn", "Greta", "Hugo", "Ivy", "Jasper", "Kit", "Lena", "Milo", "Nell",
    "Otis", "Pip", "Quill", "Rosa", "Sol", "Tess", "Ugo", "Vera", "Wren", "Xan", "Yara", "Zed", "Mabel", "Bruno",
    "Olga", "Ned", "Fern", "Gus", "Hattie", "Iggy", "Juno", "Kurt", "Lupe", "Moss", "Nora", "Oskar",
];

pub fn random_name(rng: &mut Rng) -> &'static str {
    *rng.pick(NAMES)
}

#[derive(Clone, Copy, PartialEq)]
pub enum Sky {
    Clear,
    Rain,
    Storm,
    Snow,
}

/// World-wide weather. Publishes the `rain` global (0 clear, 1 wet, 2 storm, 3 snow)
/// so behaviors can react, and the `flash` global that brightens the renderer.
pub struct Weather {
    pub sky: Sky,
    until: f64,
    thunder: f32,
    flash: f32,
}

impl Default for Weather {
    fn default() -> Self {
        Weather { sky: Sky::Clear, until: 60.0, thunder: 0.0, flash: 0.0 }
    }
}

impl Weather {
    /// Starts a storm right now, whatever the forecast said.
    pub fn force_storm(&mut self, cx: &mut Ctx) {
        self.sky = Sky::Storm;
        self.until = cx.time + 45.0;
        self.thunder = 0.0;
        cx.news("A thunderstorm rolls in!", None);
    }

    /// Advances the weather; returns where lightning struck, if it did this tick.
    pub fn tick(&mut self, cx: &mut Ctx, dt: f32, winter: bool) -> Option<(f32, f32)> {
        if cx.time >= self.until {
            let (next, secs) = match (cx.rng.f(), winter) {
                (r, true) if r < 0.75 => (Sky::Snow, 90.0),
                (r, false) if r < 0.22 => (Sky::Rain, 70.0),
                (r, false) if r < 0.36 => (Sky::Storm, 45.0),
                _ => (Sky::Clear, 120.0),
            };
            if next != self.sky {
                let line = match next {
                    Sky::Clear => "The skies clear.",
                    Sky::Rain => "It starts to rain.",
                    Sky::Storm => "A thunderstorm rolls in!",
                    Sky::Snow => "Snow begins to fall.",
                };
                cx.news(line, None);
            }
            self.sky = next;
            self.until = cx.time + (secs * cx.rng.range(0.6, 1.4)) as f64;
        }

        let drops = match self.sky {
            Sky::Clear => 0.0,
            Sky::Rain => 0.5,
            Sky::Storm => 0.9,
            Sky::Snow => 0.25,
        };
        for _ in 0..(cx.w / 12.0) as i32 {
            if cx.rng.chance(drops) {
                let x = cx.rng.range(0.0, cx.w);
                let e = if self.sky == Sky::Snow {
                    let drift = cx.rng.range(-1.5, 1.5);
                    Entity::new("snow", droplet("*", rgb(255, 255, 255))).vel(drift, 3.5)
                } else {
                    let lean = if self.sky == Sky::Storm { -5.0 } else { -1.0 };
                    Entity::new("rain", droplet("|", rgb(110, 140, 205))).vel(lean, 24.0)
                };
                cx.spawn(e.at(x, -1.0).z(1).die_offscreen());
            }
        }
        let code = match self.sky {
            Sky::Clear => 0.0,
            Sky::Rain => 1.0,
            Sky::Storm => 2.0,
            Sky::Snow => 3.0,
        };
        cx.set("rain", code);

        self.flash = (self.flash - dt * 3.0).max(0.0);
        let mut strike = None;
        if self.sky == Sky::Storm {
            self.thunder -= dt;
            if self.thunder <= 0.0 {
                self.thunder = cx.rng.range(4.0, 14.0);
                self.flash = 1.0;
                let p = (cx.rng.range(0.0, cx.w), cx.rng.range(0.0, cx.h));
                bolt(cx, p);
                strike = Some(p);
            }
        }
        cx.set("flash", self.flash);
        strike
    }
}

/// Falling particle that stays one small pixel in pixel mode instead of a full cell.
fn droplet(glyph: &str, color: Color) -> Sprite {
    Sprite::mono(glyph, color).with_pixels(vec![PixFrame { w: 1, h: 1, px: vec![Some(color)] }])
}

/// Jagged lightning column from the top of the world down to the strike.
fn bolt(cx: &mut Ctx, p: (f32, f32)) {
    let mut x = p.0;
    let mut y = p.1;
    while y > 0.0 {
        let ch = if cx.rng.chance(0.5) { "/" } else { "\\" };
        cx.spawn(Entity::new("spark", Sprite::mono(ch, rgb(255, 255, 210))).at(x, y).z(0).lifetime(0.25));
        x += cx.rng.range(-1.5, 1.5);
        y -= 1.0;
    }
}

/// One shell's worth of sparks, bursting outward and falling.
pub fn firework(cx: &mut Ctx, x: f32, y: f32) {
    let colors = [rgb(255, 90, 90), rgb(255, 220, 90), rgb(120, 220, 255), rgb(200, 130, 255), rgb(130, 255, 150)];
    let c = *cx.rng.pick(&colors);
    for i in 0..24 {
        let a = i as f32 / 24.0 * std::f32::consts::TAU;
        let s = cx.rng.range(5.0, 11.0);
        let glyph = if i % 2 == 0 { "*" } else { "+" };
        let life = cx.rng.range(0.7, 1.5);
        cx.spawn(
            Entity::new("spark", Sprite::mono(glyph, c))
                .at(x, y)
                .vel(a.cos() * s, a.sin() * s * 0.5 + 1.0)
                .z(0)
                .lifetime(life),
        );
    }
}

pub fn floating_text(text: &str, color: Color, x: f32, y: f32) -> Entity {
    Entity::new("emote", Sprite::text(text, color)).at(x, y).vel(0.0, -1.2).z(5).lifetime(1.8)
}

/// Fixed-length history rendered as a unicode bar sparkline.
#[derive(Default)]
pub struct Spark {
    values: Vec<f32>,
}

impl Spark {
    const LEN: usize = 16;

    pub fn push(&mut self, v: f32) {
        if self.values.len() == Spark::LEN {
            self.values.remove(0);
        }
        self.values.push(v);
    }

    pub fn render(&self) -> String {
        const BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
        let max = self.values.iter().copied().fold(1.0f32, f32::max);
        self.values.iter().map(|v| BARS[((v / max * 7.0) as usize).min(7)]).collect()
    }
}
