use ratatui::style::Color;
use std::sync::{Arc, OnceLock};


/// One animation frame: a character grid with per-cell colors.
/// A cell is transparent when its glyph is a space and it has no background.
#[derive(Clone)]
pub struct Frame {
    pub w: usize,
    pub h: usize,
    pub ch: Vec<char>,
    pub fg: Vec<Color>,
    pub bg: Vec<Option<Color>>,
    /// Lazily rasterized pixel form, used by the half-block renderer.
    pix: OnceLock<Arc<PixFrame>>,
}

/// Square-pixel image; one terminal cell covers 1 pixel across and 2 down.
#[derive(Clone)]
pub struct PixFrame {
    pub w: usize,
    pub h: usize,
    pub px: Vec<Option<Color>>,
}

impl PixFrame {
    /// Builds a frame from rows of palette keys. Space and `.` are transparent.
    pub fn parse(rows: &str, palette: &[(char, Color)]) -> PixFrame {
        let lines: Vec<Vec<char>> = rows.lines().map(|l| l.chars().collect()).collect();
        let h = lines.len();
        let w = lines.iter().map(|l| l.len()).max().unwrap_or(0);
        let mut px = vec![None; w * h];
        for (y, l) in lines.iter().enumerate() {
            for (x, &c) in l.iter().enumerate() {
                px[y * w + x] = palette.iter().find(|(k, _)| *k == c).map(|(_, col)| *col);
            }
        }
        PixFrame { w, h, px }
    }

    pub fn mirrored(&self) -> PixFrame {
        let mut px = vec![None; self.w * self.h];
        for y in 0..self.h {
            for x in 0..self.w {
                px[y * self.w + (self.w - 1 - x)] = self.px[y * self.w + x];
            }
        }
        PixFrame { w: self.w, h: self.h, px }
    }
}

pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

/// Mask letters understood by sprite color masks.
pub fn std_palette(c: char) -> Option<Color> {
    Some(match c {
        'k' => rgb(40, 40, 46),
        'K' => rgb(110, 110, 120),
        'r' => rgb(190, 50, 50),
        'R' => rgb(255, 100, 100),
        'g' => rgb(50, 150, 60),
        'G' => rgb(120, 230, 110),
        'y' => rgb(190, 160, 40),
        'Y' => rgb(255, 235, 90),
        'b' => rgb(60, 90, 200),
        'B' => rgb(120, 160, 255),
        'm' => rgb(170, 70, 170),
        'M' => rgb(240, 130, 240),
        'c' => rgb(50, 170, 180),
        'C' => rgb(120, 240, 245),
        'w' => rgb(170, 170, 170),
        'W' => rgb(250, 250, 250),
        'o' => rgb(220, 120, 30),
        'O' => rgb(255, 170, 70),
        'n' => rgb(120, 80, 45),
        'N' => rgb(180, 130, 80),
        'p' => rgb(255, 150, 180),
        's' => rgb(220, 190, 150),
        _ => return None,
    })
}

impl Frame {
    /// Builds a frame from an art block and an optional same-shaped color mask.
    /// Digits `1`-`9` in the mask resolve through `digits`, letters through `std_palette`.
    pub fn parse(shape: &str, mask: Option<&str>, default: Color, digits: &[Color; 9]) -> Frame {
        let lines: Vec<Vec<char>> = shape.lines().map(|l| l.chars().collect()).collect();
        let mlines: Vec<Vec<char>> = mask
            .map(|m| m.lines().map(|l| l.chars().collect()).collect())
            .unwrap_or_default();
        let h = lines.len();
        let w = lines.iter().map(|l| l.len()).max().unwrap_or(0);
        let mut f = Frame::blank(w, h);
        for (y, line) in lines.iter().enumerate() {
            for (x, &c) in line.iter().enumerate() {
                let i = y * w + x;
                f.ch[i] = c;
                let m = mlines.get(y).and_then(|l| l.get(x)).copied().unwrap_or(' ');
                f.fg[i] = match m {
                    '1'..='9' => digits[(m as u8 - b'1') as usize],
                    _ => std_palette(m).unwrap_or(default),
                };
            }
        }
        f
    }

    pub fn blank(w: usize, h: usize) -> Frame {
        Frame {
            w,
            h,
            ch: vec![' '; w * h],
            fg: vec![Color::White; w * h],
            bg: vec![None; w * h],
            pix: OnceLock::new(),
        }
    }

    pub fn filled(w: usize, h: usize, ch: char, fg: Color, bg: Option<Color>) -> Frame {
        Frame { w, h, ch: vec![ch; w * h], fg: vec![fg; w * h], bg: vec![bg; w * h], pix: OnceLock::new() }
    }

    pub fn set(&mut self, x: usize, y: usize, ch: char, fg: Color, bg: Option<Color>) {
        if x < self.w && y < self.h {
            let i = y * self.w + x;
            self.ch[i] = ch;
            self.fg[i] = fg;
            self.bg[i] = bg;
        }
    }

    pub fn text(&mut self, x: usize, y: usize, s: &str, fg: Color) {
        for (i, c) in s.chars().enumerate() {
            self.set(x + i, y, c, fg, None);
        }
    }

    /// Pixel rendition: each glyph becomes chunky pixels shaped by how much of the cell it fills.
    pub fn to_pix(&self) -> Arc<PixFrame> {
        self.pix
            .get_or_init(|| {
                let mut px = vec![None; self.w * self.h * 2];
                for y in 0..self.h {
                    for x in 0..self.w {
                        let i = y * self.w + x;
                        let (top, bottom) = glyph_pixels(self.ch[i]);
                        let bg = self.bg[i];
                        let (t, b) = (
                            if top { Some(self.fg[i]) } else { bg },
                            if bottom { Some(self.fg[i]) } else { bg },
                        );
                        px[(y * 2) * self.w + x] = t;
                        px[(y * 2 + 1) * self.w + x] = b;
                    }
                }
                Arc::new(PixFrame { w: self.w, h: self.h * 2, px })
            })
            .clone()
    }

    /// Horizontal mirror with directional glyphs swapped, so one sprite serves both facings.
    pub fn mirrored(&self) -> Frame {
        let mut out = Frame::blank(self.w, self.h);
        for y in 0..self.h {
            for x in 0..self.w {
                let src = y * self.w + x;
                let dst = y * self.w + (self.w - 1 - x);
                out.ch[dst] = flip_glyph(self.ch[src]);
                out.fg[dst] = self.fg[src];
                out.bg[dst] = self.bg[src];
            }
        }
        out
    }
}

/// Which vertical halves of a cell a glyph inks.
fn glyph_pixels(c: char) -> (bool, bool) {
    match c {
        ' ' => (false, false),
        '`' | '\'' | '"' | '^' | '*' => (true, false),
        '_' | '.' | ',' | 'v' => (false, true),
        _ => (true, true),
    }
}

fn flip_glyph(c: char) -> char {
    match c {
        '/' => '\\',
        '\\' => '/',
        '(' => ')',
        ')' => '(',
        '<' => '>',
        '>' => '<',
        '[' => ']',
        ']' => '[',
        '{' => '}',
        '}' => '{',
        'b' => 'd',
        'd' => 'b',
        'p' => 'q',
        'q' => 'p',
        '`' => '\'',
        '\'' => '`',
        other => other,
    }
}

/// A shared, cheaply clonable set of animation frames.
#[derive(Clone)]
pub struct Sprite {
    pub frames: Arc<Vec<Frame>>,
    /// Hand-drawn pixel art; when absent the ASCII frames are rasterized.
    pub pixels: Option<Arc<Vec<PixFrame>>>,
}

impl Sprite {
    pub fn new(frames: Vec<Frame>) -> Sprite {
        Sprite { frames: Arc::new(frames), pixels: None }
    }

    pub fn one(frame: Frame) -> Sprite {
        Sprite::new(vec![frame])
    }

    /// Single-frame sprite from art and a mask using the standard palette.
    pub fn art(shape: &str, mask: &str) -> Sprite {
        Sprite::one(Frame::parse(shape, Some(mask), Color::White, &[Color::White; 9]))
    }

    /// Like `art`, but mask digits `1`-`9` map to the given colors (e.g. random fish tints).
    pub fn art_d(shape: &str, mask: &str, digits: &[Color; 9]) -> Sprite {
        Sprite::one(Frame::parse(shape, Some(mask), Color::White, digits))
    }

    /// Single-frame sprite with every glyph in one color.
    pub fn mono(shape: &str, color: Color) -> Sprite {
        Sprite::one(Frame::parse(shape, None, color, &[Color::White; 9]))
    }

    /// Multi-frame sprite; each frame is `(shape, mask)`.
    pub fn anim(frames: &[(&str, &str)]) -> Sprite {
        Sprite::new(
            frames
                .iter()
                .map(|(s, m)| Frame::parse(s, Some(m), Color::White, &[Color::White; 9]))
                .collect(),
        )
    }

    pub fn mirrored(&self) -> Sprite {
        let mut s = Sprite::new(self.frames.iter().map(|f| f.mirrored()).collect());
        s.pixels = self.pixels.as_ref().map(|p| Arc::new(p.iter().map(|f| f.mirrored()).collect()));
        s
    }

    /// Attaches hand-drawn pixel frames (one per ASCII frame) for the pixel renderer.
    pub fn with_pixels(mut self, frames: Vec<PixFrame>) -> Sprite {
        self.pixels = Some(Arc::new(frames));
        self
    }

    /// Pixel frame for animation frame `i`, falling back to rasterized ASCII.
    pub fn pixel_frame(&self, i: usize) -> Arc<PixFrame> {
        match &self.pixels {
            Some(p) if !p.is_empty() => Arc::new(p[i.min(p.len() - 1)].clone()),
            _ => self.frames[i.min(self.frames.len() - 1)].to_pix(),
        }
    }

    /// Returns the sprite facing right when `right` is true, otherwise mirrored.
    /// Source art is assumed to face right.
    pub fn facing(&self, right: bool) -> Sprite {
        if right { self.clone() } else { self.mirrored() }
    }

    pub fn w(&self) -> usize {
        self.frames[0].w
    }

    pub fn h(&self) -> usize {
        self.frames[0].h
    }
}
