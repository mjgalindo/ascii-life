use super::entity::Id;
use super::world::World;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use rayon::prelude::*;

type Rgb = [u8; 3];

const NIGHT: [f32; 3] = [8.0, 10.0, 40.0];

/// Camera over the world. Positions are in world cells; `zoom` is how many world cells
/// one terminal cell spans (1 = native). `pixel` selects the half-block renderer.
#[derive(Clone, Copy)]
pub struct View {
    pub x: f32,
    pub y: f32,
    pub zoom: i32,
    pub pixel: bool,
}

pub const MAX_ZOOM: i32 = 16;

impl View {
    pub fn new() -> View {
        View { x: 0.0, y: 0.0, zoom: 1, pixel: false }
    }

    /// Keeps the camera inside the world, centering when the world is smaller than the screen.
    pub fn clamp(&mut self, world: &World, cols: i32, rows: i32) {
        let span_x = (cols * self.zoom) as f32;
        let span_y = (rows * self.zoom) as f32;
        let max_x = world.w as f32 - span_x;
        let max_y = world.h as f32 - span_y;
        self.x = if max_x < 0.0 { max_x / 2.0 } else { self.x.clamp(0.0, max_x) };
        self.y = if max_y < 0.0 { max_y / 2.0 } else { self.y.clamp(0.0, max_y) };
    }

    /// Changes zoom while keeping the world point under screen cell (cx, cy) fixed.
    pub fn zoom_at(&mut self, new_zoom: i32, cx: i32, cy: i32) {
        let nz = new_zoom.clamp(1, MAX_ZOOM);
        let wx = self.x + (cx * self.zoom) as f32;
        let wy = self.y + (cy * self.zoom) as f32;
        self.zoom = nz;
        self.x = wx - (cx * nz) as f32;
        self.y = wy - (cy * nz) as f32;
    }

    fn origin(&self) -> (i32, i32) {
        (self.x.floor() as i32, self.y.floor() as i32)
    }
}

fn to_rgb(c: Color) -> Rgb {
    match c {
        Color::Rgb(r, g, b) => [r, g, b],
        Color::Black => [0, 0, 0],
        Color::White => [255, 255, 255],
        Color::Gray => [170, 170, 170],
        Color::Red => [200, 60, 60],
        Color::Green => [60, 200, 60],
        Color::Blue => [60, 60, 200],
        Color::Yellow => [220, 200, 60],
        _ => [200, 200, 200],
    }
}

fn shade(c: Rgb, daylight: f32) -> Rgb {
    shade_by(c, (1.0 - daylight) * 0.55)
}

/// Glyphs darken less than what is behind them so text stays readable at night.
fn shade_ink(c: Rgb, daylight: f32) -> Rgb {
    shade_by(c, (1.0 - daylight) * 0.2)
}

fn shade_by(c: Rgb, k: f32) -> Rgb {
    if k <= 0.001 {
        return c;
    }
    [
        (c[0] as f32 * (1.0 - k) + NIGHT[0] * k) as u8,
        (c[1] as f32 * (1.0 - k) + NIGHT[1] * k) as u8,
        (c[2] as f32 * (1.0 - k) + NIGHT[2] * k) as u8,
    ]
}

fn lighten(c: Rgb) -> Rgb {
    [c[0] / 2 + 128, c[1] / 2 + 128, c[2] / 2 + 128]
}

fn color(c: Rgb) -> Color {
    Color::Rgb(c[0], c[1], c[2])
}

pub fn draw_world(world: &World, buf: &mut Buffer, area: Rect, view: &View, selected: Option<Id>) {
    let bg = world.scenario.as_ref().map_or([0, 0, 0], |s| to_rgb(s.background()));
    // Far-to-near: higher z is drawn first.
    let mut order: Vec<&_> = world.ents.iter().filter(|e| e.alive).collect();
    order.sort_by_key(|e| (-e.z, e.id));
    if view.pixel {
        draw_pixels(world, buf, area, view, selected, &order, bg);
    } else {
        draw_ascii(world, buf, area, view, selected, &order, bg);
    }
}

struct Cells {
    ch: Vec<char>,
    fg: Vec<Rgb>,
    bg: Vec<Rgb>,
    sel: Vec<bool>,
}

fn draw_ascii(
    world: &World,
    buf: &mut Buffer,
    area: Rect,
    view: &View,
    selected: Option<Id>,
    order: &[&super::entity::Entity],
    bg: Rgb,
) {
    let z = view.zoom.max(1);
    let (cols, rows) = (area.width as i32, area.height as i32);
    let (cw, ch) = (cols * z, rows * z);
    let n = (cw * ch) as usize;
    let mut c = Cells { ch: vec![' '; n], fg: vec![[255; 3]; n], bg: vec![bg; n], sel: vec![false; n] };
    let (wx0, wy0) = view.origin();

    for e in order {
        let f = &e.sprite.frames[e.frame.min(e.sprite.frames.len() - 1)];
        let ox = e.pos.0.round() as i32 - wx0;
        let oy = e.pos.1.round() as i32 - wy0;
        if ox >= cw || oy >= ch || ox + f.w as i32 <= 0 || oy + f.h as i32 <= 0 {
            continue;
        }
        let is_sel = Some(e.id) == selected;
        for fy in 0..f.h {
            let y = oy + fy as i32;
            if y < 0 || y >= ch {
                continue;
            }
            for fx in 0..f.w {
                let x = ox + fx as i32;
                if x < 0 || x >= cw {
                    continue;
                }
                let i = fy * f.w + fx;
                let g = f.ch[i];
                let b = f.bg[i];
                if g == ' ' && b.is_none() {
                    continue;
                }
                let di = (y * cw + x) as usize;
                if g != ' ' {
                    c.ch[di] = g;
                    c.fg[di] = to_rgb(f.fg[i]);
                }
                if let Some(b) = b {
                    c.bg[di] = to_rgb(b);
                }
                if is_sel && g != ' ' {
                    c.sel[di] = true;
                }
            }
        }
    }

    for sy in 0..rows {
        for sx in 0..cols {
            let (glyph, fg, cbg, sel) = if z == 1 {
                let i = (sy * cw + sx) as usize;
                (c.ch[i], c.fg[i], c.bg[i], c.sel[i])
            } else {
                downsample_block(&c, cw, sx * z, sy * z, z)
            };
            let cell = &mut buf[(area.x + sx as u16, area.y + sy as u16)];
            cell.set_char(glyph);
            cell.set_fg(color(shade_ink(fg, world.daylight)));
            cell.set_bg(color(shade(cbg, world.daylight)));
            if sel {
                cell.set_style(Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD));
            }
        }
    }
}

/// Collapses a z-by-z block into one cell: averaged colors, glyph chosen by how much ink it holds.
fn downsample_block(c: &Cells, cw: i32, x0: i32, y0: i32, z: i32) -> (char, Rgb, Rgb, bool) {
    let mut bg = [0u32; 3];
    let mut fg = [0u32; 3];
    let (mut ink, mut sel) = (0u32, false);
    for y in y0..y0 + z {
        for x in x0..x0 + z {
            let i = (y * cw + x) as usize;
            for (sum, v) in bg.iter_mut().zip(c.bg[i]) {
                *sum += v as u32;
            }
            if c.ch[i] != ' ' {
                ink += 1;
                for (sum, v) in fg.iter_mut().zip(c.fg[i]) {
                    *sum += v as u32;
                }
                sel |= c.sel[i];
            }
        }
    }
    let total = (z * z) as u32;
    let avg_bg = [(bg[0] / total) as u8, (bg[1] / total) as u8, (bg[2] / total) as u8];
    if ink == 0 {
        return (' ', [255; 3], avg_bg, false);
    }
    let avg_fg = [(fg[0] / ink) as u8, (fg[1] / ink) as u8, (fg[2] / ink) as u8];
    let frac = ink as f32 / total as f32;
    let g = if frac < 0.08 {
        '.'
    } else if frac < 0.25 {
        ':'
    } else if frac < 0.5 {
        '+'
    } else if frac < 0.8 {
        '*'
    } else {
        '#'
    };
    (g, avg_fg, avg_bg, sel)
}

fn draw_pixels(
    world: &World,
    buf: &mut Buffer,
    area: Rect,
    view: &View,
    selected: Option<Id>,
    order: &[&super::entity::Entity],
    bg: Rgb,
) {
    let z = view.zoom.max(1);
    let (cols, rows) = (area.width as i32, area.height as i32);
    let (pw, ph) = (cols * z, rows * 2 * z);
    let mut px: Vec<Rgb> = vec![bg; (pw * ph) as usize];
    let (wx0, wy0) = view.origin();

    // Text sprites are drawn afterwards as glyphs, never as pixels.
    for e in order.iter().filter(|e| !e.sprite.overlay) {
        let f = e.sprite.pixel_frame(e.frame);
        // Vertical motion is resolved to half cells.
        let ox = e.pos.0.round() as i32 - wx0;
        let oy = (e.pos.1 * 2.0).round() as i32 - wy0 * 2;
        if ox >= pw || oy >= ph || ox + f.w as i32 <= 0 || oy + f.h as i32 <= 0 {
            continue;
        }
        let is_sel = Some(e.id) == selected;
        for fy in 0..f.h {
            let y = oy + fy as i32;
            if y < 0 || y >= ph {
                continue;
            }
            for fx in 0..f.w {
                let x = ox + fx as i32;
                if x < 0 || x >= pw {
                    continue;
                }
                if let Some(col) = f.px[fy * f.w + fx] {
                    let rgb = to_rgb(col);
                    px[(y * pw + x) as usize] = if is_sel { lighten(rgb) } else { rgb };
                }
            }
        }
    }

    // Average z-by-z pixel blocks, then pack vertical pairs into half-block cells.
    let sample = |sx: i32, sy: i32| -> Rgb {
        if z == 1 {
            return px[(sy * pw + sx) as usize];
        }
        let mut sum = [0u32; 3];
        for y in sy * z..sy * z + z {
            for x in sx * z..sx * z + z {
                let p = px[(y * pw + x) as usize];
                for k in 0..3 {
                    sum[k] += p[k] as u32;
                }
            }
        }
        let n = (z * z) as u32;
        [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8]
    };
    let daylight = world.daylight;
    let cells: Vec<(Rgb, Rgb)> = (0..rows * cols)
        .into_par_iter()
        .map(|i| {
            let (sx, sy) = (i % cols, i / cols);
            (shade(sample(sx, sy * 2), daylight), shade(sample(sx, sy * 2 + 1), daylight))
        })
        .collect();
    for sy in 0..rows {
        for sx in 0..cols {
            let (top, bottom) = cells[(sy * cols + sx) as usize];
            let cell = &mut buf[(area.x + sx as u16, area.y + sy as u16)];
            cell.set_char('▀');
            cell.set_fg(color(top));
            cell.set_bg(color(bottom));
        }
    }

    // Text sprites are legible only as real glyphs; they are skipped when zoomed out.
    if z == 1 {
        for e in order.iter().filter(|e| e.sprite.overlay) {
            let f = &e.sprite.frames[e.frame.min(e.sprite.frames.len() - 1)];
            let (ox, oy) = (e.pos.0.round() as i32 - wx0, e.pos.1.round() as i32 - wy0);
            for fy in 0..f.h as i32 {
                for fx in 0..f.w as i32 {
                    let (x, y) = (ox + fx, oy + fy);
                    let g = f.ch[(fy as usize) * f.w + fx as usize];
                    if g == ' ' || x < 0 || y < 0 || x >= cols || y >= rows {
                        continue;
                    }
                    let cell = &mut buf[(area.x + x as u16, area.y + y as u16)];
                    let (top, bottom) = cells[(y * cols + x) as usize];
                    let mid = [
                        ((top[0] as u16 + bottom[0] as u16) / 2) as u8,
                        ((top[1] as u16 + bottom[1] as u16) / 2) as u8,
                        ((top[2] as u16 + bottom[2] as u16) / 2) as u8,
                    ];
                    cell.set_char(g);
                    cell.set_fg(color(shade_ink(to_rgb(f.fg[(fy as usize) * f.w + fx as usize]), world.daylight)));
                    cell.set_bg(color(mid));
                }
            }
        }
    }
}

pub fn hud_style() -> Style {
    Style::default().fg(Color::Rgb(180, 180, 190)).bg(Color::Rgb(20, 20, 28))
}

/// Frontmost visible entity under terminal cell (col, row); large backdrop layers are ignored.
pub fn pick(world: &World, view: &View, col: i32, row: i32) -> Option<Id> {
    let z = view.zoom.max(1);
    let (wx0, wy0) = view.origin();
    let (bx, by) = (wx0 + col * z, wy0 + row * z);
    world
        .ents
        .iter()
        .filter(|e| e.alive && e.z < 90)
        .filter(|e| {
            let f = &e.sprite.frames[e.frame.min(e.sprite.frames.len() - 1)];
            let (ex, ey) = (e.pos.0.round() as i32, e.pos.1.round() as i32);
            if z == 1 {
                let (fx, fy) = (bx - ex, by - ey);
                return fx >= 0
                    && fy >= 0
                    && (fx as usize) < f.w
                    && (fy as usize) < f.h
                    && f.ch[fy as usize * f.w + fx as usize] != ' ';
            }
            ex < bx + z && bx < ex + f.w as i32 && ey < by + z && by < ey + f.h as i32
        })
        .min_by_key(|e| (e.z, e.id))
        .map(|e| e.id)
}
