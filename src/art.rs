//! Procedural pixel art. Sprites are drawn with a few shape primitives so every
//! creature shares lighting and outline rules, then attached to the ASCII sprites.

use crate::engine::sprite::{PixFrame, rgb};
use ratatui::style::Color;

#[derive(Clone)]
pub struct Canvas {
    pub w: i32,
    pub h: i32,
    px: Vec<Option<Color>>,
}

fn parts(c: Color) -> (f32, f32, f32) {
    match c {
        Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
        _ => (200.0, 200.0, 200.0),
    }
}

fn scaled(c: Color, k: f32) -> Color {
    let (r, g, b) = parts(c);
    let f = |v: f32| (v * k).clamp(0.0, 255.0) as u8;
    rgb(f(r), f(g), f(b))
}

impl Canvas {
    pub fn new(w: i32, h: i32) -> Canvas {
        Canvas { w, h, px: vec![None; (w * h) as usize] }
    }

    pub fn get(&self, x: i32, y: i32) -> Option<Color> {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return None;
        }
        self.px[(y * self.w + x) as usize]
    }

    pub fn set(&mut self, x: i32, y: i32, c: Color) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.px[(y * self.w + x) as usize] = Some(c);
        }
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Color) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.set(xx, yy, c);
            }
        }
    }

    /// Filled ellipse centered at (cx, cy) in pixel-center coordinates.
    pub fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, c: Color) {
        for y in 0..self.h {
            for x in 0..self.w {
                let (dx, dy) = ((x as f32 + 0.5 - cx) / rx, (y as f32 + 0.5 - cy) / ry);
                if dx * dx + dy * dy <= 1.0 {
                    self.set(x, y, c);
                }
            }
        }
    }

    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: Color) {
        let n = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
        for i in 0..=n {
            let t = i as f32 / n as f32;
            self.set(
                x0 + ((x1 - x0) as f32 * t).round() as i32,
                y0 + ((y1 - y0) as f32 * t).round() as i32,
                c,
            );
        }
    }

    /// Lightens upper edges and darkens lower edges so flat shapes read as volumes.
    pub fn relight(&mut self) {
        let src = self.clone();
        for y in 0..self.h {
            for x in 0..self.w {
                let Some(c) = src.get(x, y) else { continue };
                if src.get(x, y + 1).is_none() {
                    self.set(x, y, scaled(c, 0.72));
                } else if src.get(x, y - 1).is_none() {
                    self.set(x, y, scaled(c, 1.18));
                }
            }
        }
    }

    /// Paints empty pixels touching the shape; the shape should leave a one-pixel margin.
    pub fn outline(&mut self, c: Color) {
        let src = self.clone();
        for y in 0..self.h {
            for x in 0..self.w {
                if src.get(x, y).is_some() {
                    continue;
                }
                let touching = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .any(|(dx, dy)| src.get(x + dx, y + dy).is_some());
                if touching {
                    self.set(x, y, c);
                }
            }
        }
    }

    pub fn frame(&self) -> PixFrame {
        PixFrame { w: self.w as usize, h: self.h as usize, px: self.px.clone() }
    }

    /// Lying-down variant: keeps the lower part of the shape, dimmed, bottom-aligned.
    pub fn lying(&self) -> Canvas {
        let mut top = self.h;
        for y in 0..self.h {
            if (0..self.w).any(|x| self.get(x, y).is_some()) {
                top = y;
                break;
            }
        }
        let body_h = self.h - top;
        let cut = body_h * 2 / 5;
        let mut out = Canvas::new(self.w, self.h);
        let keep_from = top + cut;
        for y in keep_from..self.h {
            for x in 0..self.w {
                if let Some(c) = self.get(x, y) {
                    out.set(x, y - cut, scaled(c, 0.85));
                }
            }
        }
        // Shift the remaining body down so it rests on the ground line.
        let mut grounded = Canvas::new(self.w, self.h);
        for y in 0..self.h {
            for x in 0..self.w {
                if let Some(c) = out.get(x, y) {
                    grounded.set(x, y + cut.min(self.h - 1 - y), c);
                }
            }
        }
        grounded
    }
}

const OUT: Color = rgb(34, 24, 26);
const EYE: Color = rgb(18, 14, 18);
const WHITE: Color = rgb(240, 240, 236);

/// Pixel art for a species given its footprint in cells (pixels are w by 2h).
pub fn animal(name: &str, w: i32, h: i32) -> Option<Canvas> {
    let mut c = Canvas::new(w, h * 2);
    let (wf, hf) = (w as f32, (h * 2) as f32);
    match name {
        "LION" => {
            let tan = rgb(214, 160, 76);
            let mane = rgb(140, 84, 38);
            c.ellipse(wf * 0.40, hf * 0.50, wf * 0.31, hf * 0.27, tan);
            for x in [1, 3, 5, 6] {
                c.rect(x, (hf * 0.62) as i32, 1, (hf * 0.38) as i32, scaled(tan, 0.85));
            }
            c.ellipse(wf * 0.76, hf * 0.42, wf * 0.22, hf * 0.40, mane);
            c.ellipse(wf * 0.82, hf * 0.50, wf * 0.12, hf * 0.22, scaled(tan, 1.1));
            c.set(w - 3, (hf * 0.38) as i32, EYE);
            c.set(w - 1, (hf * 0.50) as i32, EYE);
            c.line(0, (hf * 0.45) as i32, 1, (hf * 0.30) as i32, tan);
            c.set(0, (hf * 0.25) as i32, mane);
        }
        "ELEPHANT" => {
            let grey = rgb(140, 142, 156);
            c.ellipse(wf * 0.42, hf * 0.45, wf * 0.38, hf * 0.34, grey);
            for x in [2, 4, 7, 8] {
                c.rect(x, (hf * 0.60) as i32, 2, (hf * 0.40) as i32, scaled(grey, 0.9));
            }
            c.ellipse(wf * 0.82, hf * 0.34, wf * 0.16, hf * 0.28, scaled(grey, 1.05));
            c.ellipse(wf * 0.72, hf * 0.30, wf * 0.09, hf * 0.22, scaled(grey, 0.8));
            c.rect(w - 2, (hf * 0.45) as i32, 1, (hf * 0.45) as i32, grey);
            c.set(w - 3, (hf * 0.82) as i32, WHITE);
            c.set(w - 3, (hf * 0.30) as i32, EYE);
        }
        "GIRAFFE" => {
            let yel = rgb(232, 190, 80);
            let spot = rgb(150, 98, 40);
            c.ellipse(wf * 0.38, hf * 0.62, wf * 0.34, hf * 0.15, yel);
            c.rect((wf * 0.55) as i32, (hf * 0.12) as i32, 2, (hf * 0.45) as i32, yel);
            c.ellipse(wf * 0.74, hf * 0.12, wf * 0.2, hf * 0.07, yel);
            for x in [1, 2, 5, 6] {
                c.rect(x, (hf * 0.70) as i32, 1, (hf * 0.30) as i32, scaled(yel, 0.88));
            }
            for (x, y) in [(2, 0.60), (4, 0.64), (3, 0.70), (5, 0.60)] {
                c.set(x, (hf * y) as i32, spot);
            }
            c.set((wf * 0.57) as i32, (hf * 0.35) as i32, spot);
            c.set((wf * 0.57) as i32, (hf * 0.50) as i32, spot);
            c.set(w - 2, (hf * 0.10) as i32, EYE);
            c.set((wf * 0.62) as i32, 0, spot);
        }
        "PENGUINS" => {
            c.ellipse(wf * 0.5, hf * 0.55, wf * 0.38, hf * 0.40, rgb(34, 38, 52));
            c.ellipse(wf * 0.55, hf * 0.62, wf * 0.22, hf * 0.30, WHITE);
            c.set((wf * 0.72) as i32, (hf * 0.30) as i32, WHITE);
            c.set((wf * 0.78) as i32, (hf * 0.30) as i32, EYE);
            c.set(w - 1, (hf * 0.38) as i32, rgb(240, 150, 40));
            c.rect(1, h * 2 - 1, 1, 1, rgb(240, 150, 40));
            c.rect(w - 2, h * 2 - 1, 1, 1, rgb(240, 150, 40));
        }
        "MONKEYS" => {
            let brown = rgb(150, 100, 56);
            c.ellipse(wf * 0.42, hf * 0.58, wf * 0.26, hf * 0.28, brown);
            c.ellipse(wf * 0.68, hf * 0.34, wf * 0.17, hf * 0.22, brown);
            c.ellipse(wf * 0.72, hf * 0.40, wf * 0.10, hf * 0.13, rgb(222, 184, 140));
            c.set((wf * 0.75) as i32, (hf * 0.30) as i32, EYE);
            c.line(0, (hf * 0.35) as i32, 1, (hf * 0.75) as i32, brown);
            c.set(0, (hf * 0.25) as i32, brown);
            c.set(2, h * 2 - 1, scaled(brown, 0.8));
            c.set(4, h * 2 - 1, scaled(brown, 0.8));
        }
        "SNAKES" => {
            let green = rgb(72, 168, 74);
            for x in 0..w - 2 {
                let y = (hf * 0.6 + (x as f32 * 0.9).sin() * hf * 0.22) as i32;
                c.set(x, y, green);
                c.set(x, y + 1, scaled(green, 0.8));
            }
            c.ellipse(wf - 2.0, hf * 0.4, 1.8, 1.3, scaled(green, 1.1));
            c.set(w - 2, (hf * 0.3) as i32, EYE);
            c.set(w - 1, (hf * 0.5) as i32, rgb(230, 60, 80));
        }
        "FLAMINGOS" => {
            let pink = rgb(246, 140, 176);
            c.ellipse(wf * 0.42, hf * 0.50, wf * 0.38, hf * 0.18, pink);
            c.line((wf * 0.6) as i32, (hf * 0.45) as i32, w - 2, (hf * 0.12) as i32, pink);
            c.set(w - 1, (hf * 0.12) as i32, rgb(250, 230, 220));
            c.set(w - 2, (hf * 0.08) as i32, EYE);
            c.line(1, (hf * 0.62) as i32, 1, h * 2 - 1, scaled(pink, 0.85));
            c.line(2, (hf * 0.62) as i32, 2, h * 2 - 1, scaled(pink, 0.85));
        }
        "PARROTS" => {
            let red = rgb(226, 62, 58);
            c.ellipse(wf * 0.45, hf * 0.55, wf * 0.34, hf * 0.30, red);
            c.set(w - 2, (hf * 0.25) as i32, rgb(250, 220, 90));
            c.set(w - 1, (hf * 0.40) as i32, rgb(250, 220, 90));
            c.set(0, h * 2 - 1, rgb(60, 110, 230));
            c.set(1, (hf * 0.70) as i32, rgb(60, 110, 230));
            c.set(w - 3, (hf * 0.30) as i32, EYE);
        }
        "BEARS" => {
            let brown = rgb(112, 74, 46);
            c.ellipse(wf * 0.40, hf * 0.52, wf * 0.33, hf * 0.30, brown);
            for x in [1, 3, 5, 6] {
                c.rect(x, (hf * 0.64) as i32, 2, (hf * 0.36) as i32, scaled(brown, 0.9));
            }
            c.ellipse(wf * 0.78, hf * 0.40, wf * 0.19, hf * 0.28, scaled(brown, 1.1));
            c.set((wf * 0.70) as i32, (hf * 0.14) as i32, brown);
            c.set((wf * 0.84) as i32, (hf * 0.14) as i32, brown);
            c.ellipse(wf * 0.88, hf * 0.48, wf * 0.08, hf * 0.12, rgb(196, 150, 104));
            c.set((wf * 0.80) as i32, (hf * 0.34) as i32, EYE);
            c.set(w - 1, (hf * 0.44) as i32, EYE);
        }
        "ZEBRAS" => {
            c.ellipse(wf * 0.42, hf * 0.50, wf * 0.34, hf * 0.28, WHITE);
            for x in [1, 3, 5, 7] {
                c.rect(x, (hf * 0.62) as i32, 1, (hf * 0.38) as i32, rgb(214, 214, 214));
            }
            c.ellipse(wf * 0.80, hf * 0.34, wf * 0.16, hf * 0.28, WHITE);
            c.set((wf * 0.82) as i32, (hf * 0.34) as i32, EYE);
            c.set(w - 1, (hf * 0.45) as i32, rgb(110, 110, 110));
            for x in (1..w - 3).step_by(2) {
                for y in 0..h * 2 {
                    if c.get(x, y).is_some() {
                        c.set(x, y, rgb(40, 40, 46));
                    }
                }
            }
        }
        "CROCODILES" => {
            let green = rgb(66, 124, 62);
            c.ellipse(wf * 0.46, hf * 0.62, wf * 0.42, hf * 0.22, green);
            c.rect(w - 4, (hf * 0.52) as i32, 4, (hf * 0.24) as i32, scaled(green, 1.1));
            c.line(0, (hf * 0.70) as i32, 2, (hf * 0.58) as i32, scaled(green, 0.9));
            c.set(w - 5, (hf * 0.46) as i32, rgb(240, 220, 90));
            for x in (2..w - 5).step_by(2) {
                c.set(x, (hf * 0.38) as i32, scaled(green, 0.9));
            }
            for x in [3, 5, 8, 10] {
                c.set(x, h * 2 - 1, scaled(green, 0.8));
            }
        }
        "KANGAROOS" => {
            let tan = rgb(206, 140, 80);
            c.ellipse(wf * 0.44, hf * 0.60, wf * 0.28, hf * 0.28, tan);
            c.ellipse(wf * 0.66, hf * 0.28, wf * 0.14, hf * 0.18, scaled(tan, 1.08));
            c.set((wf * 0.62) as i32, (hf * 0.06) as i32, tan);
            c.set((wf * 0.74) as i32, (hf * 0.28) as i32, EYE);
            c.rect(2, (hf * 0.78) as i32, 3, (hf * 0.22) as i32, scaled(tan, 0.85));
            c.line(0, (hf * 0.9) as i32, 1, (hf * 0.7) as i32, scaled(tan, 0.9));
            c.rect(w - 2, (hf * 0.45) as i32, 1, (hf * 0.22) as i32, scaled(tan, 0.9));
        }
        "TIGER" => {
            let orange = rgb(232, 128, 40);
            c.ellipse(wf * 0.42, hf * 0.50, wf * 0.34, hf * 0.27, orange);
            for x in [1, 3, 5, 6] {
                c.rect(x, (hf * 0.62) as i32, 1, (hf * 0.38) as i32, scaled(orange, 0.85));
            }
            c.ellipse(wf * 0.80, hf * 0.42, wf * 0.19, hf * 0.34, scaled(orange, 1.05));
            c.ellipse(wf * 0.88, hf * 0.52, wf * 0.10, hf * 0.14, WHITE);
            c.set(w - 3, (hf * 0.36) as i32, EYE);
            c.set(w - 1, (hf * 0.50) as i32, EYE);
            c.line(0, (hf * 0.45) as i32, 1, (hf * 0.28) as i32, orange);
            for x in (2..w - 3).step_by(2) {
                for y in 0..h * 2 {
                    if c.get(x, y) == Some(orange) || c.get(x, y) == Some(scaled(orange, 0.85)) {
                        c.set(x, y, rgb(40, 28, 24));
                    }
                }
            }
        }
        "HIPPO" => {
            let hide = rgb(150, 124, 150);
            c.ellipse(wf * 0.42, hf * 0.52, wf * 0.40, hf * 0.34, hide);
            for x in [2, 3, 7, 8] {
                c.rect(x, (hf * 0.70) as i32, 2, (hf * 0.30) as i32, scaled(hide, 0.85));
            }
            c.ellipse(wf * 0.80, hf * 0.50, wf * 0.22, hf * 0.32, scaled(hide, 1.08));
            c.rect(w - 4, (hf * 0.62) as i32, 4, 1, rgb(214, 130, 140));
            c.set((wf * 0.74) as i32, (hf * 0.28) as i32, hide);
            c.set((wf * 0.78) as i32, (hf * 0.36) as i32, EYE);
            c.set(w - 2, (hf * 0.46) as i32, EYE);
        }
        "RHINO" => {
            let grey = rgb(150, 150, 160);
            c.ellipse(wf * 0.40, hf * 0.52, wf * 0.36, hf * 0.30, grey);
            for x in [1, 3, 6, 7] {
                c.rect(x, (hf * 0.66) as i32, 2, (hf * 0.34) as i32, scaled(grey, 0.85));
            }
            c.ellipse(wf * 0.82, hf * 0.50, wf * 0.19, hf * 0.28, scaled(grey, 1.05));
            c.line(w - 2, (hf * 0.34) as i32, w - 3, (hf * 0.12) as i32, rgb(232, 226, 210));
            c.set((wf * 0.72) as i32, (hf * 0.22) as i32, grey);
            c.set((wf * 0.80) as i32, (hf * 0.42) as i32, EYE);
        }
        "WOLVES" => {
            let fur = rgb(132, 134, 146);
            c.ellipse(wf * 0.40, hf * 0.52, wf * 0.32, hf * 0.22, fur);
            for x in [2, 3, 6, 7] {
                c.rect(x, (hf * 0.62) as i32, 1, (hf * 0.38) as i32, scaled(fur, 0.85));
            }
            c.ellipse(wf * 0.76, hf * 0.40, wf * 0.14, hf * 0.22, scaled(fur, 1.08));
            c.rect(w - 3, (hf * 0.44) as i32, 3, 2, rgb(190, 190, 198));
            c.set((wf * 0.72) as i32, (hf * 0.10) as i32, fur);
            c.set((wf * 0.82) as i32, (hf * 0.10) as i32, fur);
            c.set((wf * 0.78) as i32, (hf * 0.34) as i32, EYE);
            c.set(w - 1, (hf * 0.46) as i32, EYE);
            c.line(0, (hf * 0.72) as i32, 2, (hf * 0.48) as i32, scaled(fur, 0.9));
        }
        "DEER" => {
            let tan = rgb(190, 130, 76);
            c.ellipse(wf * 0.40, hf * 0.52, wf * 0.28, hf * 0.18, tan);
            for x in [2, 3, 5, 6] {
                c.rect(x, (hf * 0.58) as i32, 1, (hf * 0.42) as i32, scaled(tan, 0.85));
            }
            c.line((wf * 0.58) as i32, (hf * 0.44) as i32, (wf * 0.72) as i32, (hf * 0.20) as i32, tan);
            c.ellipse(wf * 0.78, hf * 0.20, wf * 0.14, hf * 0.10, scaled(tan, 1.08));
            let antler = rgb(220, 200, 160);
            c.line((wf * 0.74) as i32, (hf * 0.12) as i32, (wf * 0.66) as i32, 0, antler);
            c.line((wf * 0.80) as i32, (hf * 0.12) as i32, (wf * 0.88) as i32, 0, antler);
            c.set((wf * 0.80) as i32, (hf * 0.20) as i32, EYE);
            for (x, y) in [(3, 0.42), (5, 0.48), (2, 0.50)] {
                c.set(x, (hf * y) as i32, WHITE);
            }
        }
        "TORTOISES" => {
            let shell = rgb(102, 128, 62);
            c.ellipse(wf * 0.42, hf * 0.50, wf * 0.34, hf * 0.40, shell);
            for (x, y) in [(0.30, 0.40), (0.42, 0.30), (0.54, 0.42), (0.40, 0.55), (0.30, 0.62)] {
                c.set((wf * x) as i32, (hf * y) as i32, rgb(66, 88, 40));
            }
            c.ellipse(wf * 0.86, hf * 0.62, wf * 0.10, hf * 0.18, rgb(168, 176, 98));
            c.set(w - 2, (hf * 0.56) as i32, EYE);
            for x in [2, 3, 7, 8] {
                c.set(x, h * 2 - 1, rgb(150, 160, 90));
            }
        }
        "OSTRICHES" => {
            let dark = rgb(54, 52, 60);
            c.ellipse(wf * 0.40, hf * 0.50, wf * 0.34, hf * 0.22, dark);
            c.ellipse(wf * 0.24, hf * 0.46, wf * 0.14, hf * 0.14, WHITE);
            c.line((wf * 0.62) as i32, (hf * 0.44) as i32, (wf * 0.74) as i32, (hf * 0.12) as i32, rgb(234, 190, 170));
            c.ellipse(wf * 0.78, hf * 0.10, wf * 0.12, hf * 0.07, rgb(238, 200, 180));
            c.set(w - 1, (hf * 0.10) as i32, rgb(240, 160, 50));
            c.set((wf * 0.76) as i32, (hf * 0.08) as i32, EYE);
            c.line(3, (hf * 0.66) as i32, 3, h * 2 - 1, rgb(234, 190, 170));
            c.line(4, (hf * 0.66) as i32, 5, h * 2 - 1, rgb(234, 190, 170));
        }
        _ => return None,
    }
    c.relight();
    c.outline(OUT);
    Some(c)
}

const SKIN: Color = rgb(238, 196, 150);
const GLASS: Color = rgb(150, 208, 240);

/// Standing person, 1 pixel wide and 4 tall (one cell by two).
pub fn person(shirt: Color, hat: Option<Color>) -> PixFrame {
    let mut c = Canvas::new(1, 4);
    c.set(0, 0, hat.unwrap_or(SKIN));
    c.set(0, 1, shirt);
    c.set(0, 2, scaled(shirt, 0.85));
    c.set(0, 3, rgb(50, 56, 84));
    c.frame()
}

/// Distant pedestrian: head and body only.
pub fn walker(shirt: Color) -> PixFrame {
    let mut c = Canvas::new(1, 2);
    c.set(0, 0, SKIN);
    c.set(0, 1, shirt);
    c.frame()
}

/// Road vehicle facing east (`dir` 0), south (1), west (2) or north (3).
/// `siren` paints a flashing light on the roof.
pub fn vehicle(dir: usize, w: i32, h: i32, body: Color, siren: Option<Color>) -> PixFrame {
    let horizontal = dir.is_multiple_of(2);
    let (len, wid) = if horizontal { (w, h * 2) } else { (h * 2, w) };
    // Drawn facing "forward" along the length axis, then rotated into place.
    let mut c = Canvas::new(len, wid);
    let roof = scaled(body, 1.2);
    let dark = rgb(30, 30, 36);
    for x in 0..len {
        for y in 0..wid {
            c.set(x, y, body);
        }
    }
    if wid >= 2 && horizontal {
        // Side view: roofline on top, wheels on the bottom row.
        for x in 1..len - 1 {
            c.set(x, 0, roof);
        }
        c.set(0, 0, rgb(0, 0, 0));
        c.px_clear(0, 0);
        c.set(len - 1, 0, GLASS);
        if len >= 6 {
            for x in 2..len - 2 {
                c.set(x, 0, if x % 2 == 0 { GLASS } else { roof });
            }
        }
        c.set(1, wid - 1, dark);
        c.set(len - 2, wid - 1, dark);
        if len >= 6 {
            c.set(len / 2, wid - 1, dark);
        }
    } else {
        // Top view: wheels at the corners, glass at the front.
        for y in 0..wid {
            c.set(1, y, roof);
        }
        c.set(0, 0, dark);
        c.set(2, 0, dark);
        c.set(0, wid - 1, dark);
        c.set(2, wid - 1, dark);
        c.set(0, wid - 2, GLASS);
        c.set(1, wid - 2, GLASS);
        c.set(2, wid - 2, GLASS);
    }
    if let Some(s) = siren {
        if horizontal {
            c.set(len / 2, 0, s);
        } else {
            c.set(1, wid / 2, s);
        }
    }
    // Rotate so "forward" points where the car is heading.
    let mut out = Canvas::new(w, h * 2);
    for y in 0..out.h {
        for x in 0..out.w {
            let src = match dir {
                0 => c.get(x, y),
                2 => c.get(len - 1 - x, y),
                1 => c.get(x, y),
                _ => c.get(x, wid - 1 - y),
            };
            if let Some(col) = src {
                out.set(x, y, col);
            }
        }
    }
    out.frame()
}

impl Canvas {
    fn px_clear(&mut self, x: i32, y: i32) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.px[(y * self.w + x) as usize] = None;
        }
    }
}

/// Side-view fish facing east.
pub fn fish(w: i32, h: i32, body: Color, eye: bool) -> Canvas {
    let mut c = Canvas::new(w, h * 2);
    let (wf, hf) = (w as f32, (h * 2) as f32);
    c.ellipse(wf * 0.58, hf * 0.5, wf * 0.42, hf * 0.34, body);
    let tail = scaled(body, 0.9);
    let tail_w = ((w as f32 * 0.28) as i32).max(1);
    for x in 0..tail_w {
        let spread = ((tail_w - x) as f32 * 0.5 * hf / (tail_w as f32 + 1.0)).ceil() as i32;
        for y in (hf as i32 / 2 - spread).max(0)..(hf as i32 / 2 + spread + 1).min(h * 2) {
            c.set(x, y, tail);
        }
    }
    if eye && w >= 4 {
        c.set(w - 2, ((hf * 0.38) as i32).max(0), EYE);
    }
    // One-cell fish are only two pixels tall; shading and outlines would swallow them.
    if h > 1 {
        c.relight();
        c.outline(OUT);
    } else {
        for x in 0..w {
            if let Some(col) = c.get(x, 1) {
                c.set(x, 1, scaled(col, 0.8));
            }
        }
    }
    c
}

pub fn shark(w: i32, h: i32) -> Canvas {
    let mut c = Canvas::new(w, h * 2);
    let (wf, hf) = (w as f32, (h * 2) as f32);
    let grey = rgb(116, 138, 168);
    c.ellipse(wf * 0.55, hf * 0.62, wf * 0.42, hf * 0.22, grey);
    c.ellipse(wf * 0.58, hf * 0.74, wf * 0.30, hf * 0.10, rgb(206, 218, 230));
    for i in 0..(hf * 0.30) as i32 {
        c.set((wf * 0.46) as i32 + i / 2, (hf * 0.38) as i32 + i, grey);
        c.set((wf * 0.46) as i32 + i / 2 + 1, (hf * 0.38) as i32 + i, grey);
    }
    for i in 0..(hf * 0.35) as i32 {
        c.set(i / 3, (hf * 0.35) as i32 + i, scaled(grey, 0.9));
        c.set(i / 3 + 1, (hf * 0.35) as i32 + i, scaled(grey, 0.9));
    }
    c.set(w - 4, (hf * 0.55) as i32, EYE);
    c.line(w - 6, (hf * 0.72) as i32, w - 1, (hf * 0.68) as i32, rgb(60, 40, 50));
    c.relight();
    c.outline(OUT);
    c
}

/// Whale facing east; `spout` 0..=2 raises water over the back inside the top `spout_rows` pixels.
pub fn whale(w: i32, h: i32, spout_rows: i32, spout: i32) -> Canvas {
    let mut c = Canvas::new(w, h * 2);
    let body_h = h * 2 - spout_rows;
    let (wf, hf) = (w as f32, body_h as f32);
    let blue = rgb(70, 112, 190);
    let oy = spout_rows as f32;
    c.ellipse(wf * 0.55, oy + hf * 0.55, wf * 0.42, hf * 0.42, blue);
    c.ellipse(wf * 0.6, oy + hf * 0.78, wf * 0.32, hf * 0.16, rgb(190, 210, 240));
    for i in 0..(hf * 0.5) as i32 {
        c.set(i / 2, spout_rows + (hf * 0.15) as i32 + i, scaled(blue, 0.9));
        c.set(i / 2 + 2, spout_rows + (hf * 0.15) as i32 + i, scaled(blue, 0.9));
    }
    c.set(w - 4, spout_rows + (hf * 0.45) as i32, EYE);
    c.relight();
    c.outline(OUT);
    let drop = rgb(200, 230, 255);
    let top = spout_rows;
    for i in 0..spout {
        let x = (wf * 0.62) as i32;
        c.set(x, top - 1 - i * 2, drop);
        c.set(x, top - 2 - i * 2, drop);
        if i == spout - 1 {
            c.set(x - 1, top - 3 - i * 2, drop);
            c.set(x + 1, top - 3 - i * 2, drop);
        }
    }
    c
}

pub fn dolphin(w: i32, h: i32) -> Canvas {
    let mut c = Canvas::new(w, h * 2);
    let (wf, hf) = (w as f32, (h * 2) as f32);
    let grey = rgb(118, 150, 206);
    c.ellipse(wf * 0.5, hf * 0.6, wf * 0.42, hf * 0.28, grey);
    c.ellipse(wf * 0.55, hf * 0.72, wf * 0.3, hf * 0.12, rgb(214, 226, 242));
    c.rect(w - 3, (hf * 0.62) as i32, 3, 1, grey);
    c.set((wf * 0.4) as i32, (hf * 0.25) as i32, scaled(grey, 0.9));
    c.set((wf * 0.4) as i32 + 1, (hf * 0.32) as i32, scaled(grey, 0.9));
    c.line(0, (hf * 0.3) as i32, 1, (hf * 0.55) as i32, scaled(grey, 0.9));
    c.set(w - 3, (hf * 0.5) as i32, EYE);
    c.relight();
    c.outline(OUT);
    c
}

pub fn ship(w: i32, h: i32) -> Canvas {
    let mut c = Canvas::new(w, h * 2);
    let (wf, hf) = (w as f32, (h * 2) as f32);
    let wood = rgb(130, 84, 48);
    c.rect(1, (hf * 0.72) as i32, w - 2, (hf * 0.20) as i32, wood);
    c.ellipse(wf * 0.5, hf * 0.9, wf * 0.46, hf * 0.12, scaled(wood, 0.9));
    let sail = rgb(244, 240, 226);
    for (mast, top, half) in [(wf * 0.25, 0.08, 4), (wf * 0.52, 0.0, 5), (wf * 0.78, 0.1, 4)] {
        let mx = mast as i32;
        c.rect(mx, (hf * top) as i32, 1, (hf * 0.74) as i32 - (hf * top) as i32, rgb(90, 60, 40));
        for row in 0..((hf * 0.50) as i32) {
            let width = ((row as f32 / (hf * 0.50)) * half as f32).ceil() as i32 + 1;
            for dx in 1..=width {
                c.set(mx + dx, (hf * top) as i32 + 2 + row, sail);
            }
        }
    }
    c.set((wf * 0.52) as i32 + 1, 0, rgb(220, 60, 60));
    c.relight();
    c.outline(OUT);
    c
}

pub fn duck(w: i32, h: i32) -> Canvas {
    let mut c = Canvas::new(w, h * 2);
    c.ellipse(w as f32 * 0.45, h as f32 * 2.0 * 0.62, w as f32 * 0.4, 0.8, rgb(246, 220, 90));
    c.set(w - 2, 0, rgb(246, 220, 90));
    c.set(w - 1, 1, rgb(240, 140, 40));
    c
}

pub fn sea_monster(w: i32, h: i32, phase: f32) -> Canvas {
    let mut c = Canvas::new(w, h * 2);
    let green = rgb(84, 184, 96);
    for x in 0..w - 3 {
        let y = ((h * 2) as f32 * 0.55 + ((x as f32 * 0.7) + phase).sin() * (h as f32 * 0.8)) as i32;
        c.set(x, y, green);
        c.set(x, y + 1, scaled(green, 0.8));
    }
    c.ellipse(w as f32 - 2.5, 1.8, 2.2, 1.6, scaled(green, 1.1));
    c.set(w - 2, 1, EYE);
    c.relight();
    c.outline(OUT);
    c
}

/// Cottage: red roof, lit or dark windows. Footprint 9x5 cells.
pub fn house(roof: Color, wall: Color, night: bool) -> PixFrame {
    let (w, h) = (9, 10);
    let mut c = Canvas::new(w, h);
    for y in 0..5 {
        let inset = 4 - y;
        for x in inset..w - inset {
            c.set(x, y, roof);
        }
    }
    c.rect(0, 4, w, 1, scaled(roof, 0.8));
    c.rect(1, 5, w - 2, 5, wall);
    let win = if night { rgb(255, 224, 110) } else { rgb(110, 150, 205) };
    c.rect(2, 6, 2, 2, win);
    c.rect(5, 6, 2, 2, win);
    c.rect(4, 8, 1, 2, rgb(96, 62, 40));
    c.set(7, 1, rgb(90, 80, 80));
    c.set(7, 2, rgb(90, 80, 80));
    c.relight();
    c.outline(OUT);
    c.frame()
}

/// Tree by season (0 spring, 1 summer, 2 autumn, 3 winter). Footprint 5x3 cells.
pub fn tree(season: usize) -> PixFrame {
    let mut c = Canvas::new(5, 6);
    let trunk = rgb(104, 72, 44);
    c.rect(2, 4, 1, 2, trunk);
    match season {
        3 => {
            c.line(2, 4, 0, 1, trunk);
            c.line(2, 4, 4, 1, trunk);
            c.line(2, 4, 2, 0, trunk);
            for (x, y) in [(0, 1), (4, 1), (2, 0), (1, 2), (3, 2)] {
                c.set(x, y, rgb(240, 244, 252));
            }
        }
        s => {
            let leaf = [rgb(126, 206, 104), rgb(54, 150, 62), rgb(226, 124, 42)][s];
            c.ellipse(2.5, 2.4, 2.4, 2.3, leaf);
            c.set(1, 2, scaled(leaf, 1.15));
            c.set(3, 1, scaled(leaf, 0.85));
        }
    }
    c.relight();
    c.outline(OUT);
    c.frame()
}

/// Shop front: roof band, wall, door and windows that glow at `night`. Size in pixels.
pub fn shop(w: i32, h: i32, roof: Color, wall: Color, night: bool) -> PixFrame {
    let mut c = Canvas::new(w, h);
    let roof_h = (h * 2 / 5).max(2);
    for y in 0..roof_h {
        let inset = (roof_h - 1 - y).min(w / 3);
        c.rect(inset, y, w - inset * 2, 1, if y % 2 == 0 { roof } else { scaled(roof, 0.82) });
    }
    c.rect(1, roof_h, w - 2, h - roof_h, wall);
    let glass = if night { rgb(255, 225, 110) } else { GLASS };
    let mut x = 2;
    while x + 2 < w - 1 {
        c.rect(x, roof_h + 1, 2, 2.min(h - roof_h - 1), glass);
        x += 4;
    }
    c.rect(w / 2 - 1, h - 3, 2, 3, rgb(70, 46, 30));
    c.frame()
}

/// Striped-awning snack stand. Size in pixels.
pub fn kiosk(w: i32, h: i32) -> PixFrame {
    let mut c = Canvas::new(w, h);
    let awning = (h / 2).max(2);
    for x in 0..w {
        let col = if (x / 2) % 2 == 0 { rgb(236, 96, 120) } else { rgb(250, 240, 235) };
        c.rect(x, 0, 1, awning, col);
    }
    c.rect(1, awning, w - 2, h - awning, rgb(196, 150, 104));
    c.rect(2, awning, w - 4, 1, rgb(60, 44, 34));
    c.frame()
}

/// Cut tree stump.
pub fn stump() -> PixFrame {
    let mut c = Canvas::new(5, 6);
    c.rect(1, 4, 3, 2, rgb(116, 84, 54));
    c.rect(1, 4, 3, 1, rgb(190, 156, 110));
    c.frame()
}
