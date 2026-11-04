//! Perspective view of a running world. The simulation stays two dimensional: this module
//! only decides where each entity sits in space, so it cannot affect determinism.
//!
//! Model space has X to the right, Y up and Z going away from the default camera, with one
//! unit per half-block pixel. A terminal cell is one unit wide and two tall, which is why
//! world `y` is doubled everywhere.

use super::entity::{Entity, Id};
use super::render::{View, color, lighten, light, shade, shade_ink, to_rgb};
use super::sprite::{Frame, PixFrame};
use super::world::{FastHasher, Space, World};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::collections::HashMap;
use std::sync::Arc;

type Rgb = [u8; 3];
type V3 = [f32; 3];

/// Half-width of the view is tan(FOV / 2) at unit distance.
const TAN_HALF_FOV: f32 = 0.577;
const NEAR: f32 = 0.5;
const MAX_FOG: f32 = 0.6;
pub const PITCH_RANGE: (f32, f32) = (0.05, 1.35);
/// Sprites at or beyond this layer are backdrops.
const BACKDROP_Z: i32 = 90;
/// Lit fraction of the box faces that do not carry the sprite.
const ROOF_LIGHT: f32 = 1.0;
const SIDE_LIGHT: f32 = 0.65;

const FLAT_KINDS: &[&str] = &["road", "ground", "terrain", "field", "pen", "trail"];
const BOX_KINDS: &[&str] = &["building", "house", "tank", "kiosk", "site"];

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: V3, k: f32) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn space(world: &World) -> Space {
    world.scenario.as_ref().map_or(Space::Ground, |s| s.space())
}

/// How far the water extends behind the glass in a tank world.
fn tank_depth(world: &World) -> f32 {
    (world.h as f32 * 2.0).min(world.w as f32 * 0.6)
}

/// Stable position in 0..1 from an id, so a creature keeps its depth for life.
fn hash01(id: Id) -> f32 {
    let h = id.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    ((h >> 40) & 0xFFFF) as f32 / 65536.0
}

struct Camera {
    pos: V3,
    fwd: V3,
    right: V3,
    up: V3,
    /// Ray slope per terminal column.
    step: f32,
    cols: f32,
    rows: f32,
    dist: f32,
}

impl Camera {
    fn new(world: &World, view: &View, cols: i32, rows: i32) -> Camera {
        let z = view.zoom.max(1) as f32;
        let (tx, ty) = (view.x + cols as f32 * z / 2.0, view.y + rows as f32 * z / 2.0);
        let target = match space(world) {
            Space::Ground => [tx, 0.0, -2.0 * ty],
            Space::Tank => [tx, -2.0 * ty, tank_depth(world) / 2.0],
        };
        let (sy, cy) = view.yaw.sin_cos();
        let (sp, cp) = view.pitch.sin_cos();
        let fwd = [sy * cp, -sp, cy * cp];
        let right = [cy, 0.0, -sy];
        let up = cross(fwd, right);
        let step = 2.0 * TAN_HALF_FOV / cols.max(1) as f32;
        // At this distance one column spans `zoom` world cells, matching the flat view.
        let dist = z / step;
        Camera { pos: sub(target, mul(fwd, dist)), fwd, right, up, step, cols: cols as f32, rows: rows as f32, dist }
    }

    /// Ray through a point given in fractional terminal cells.
    fn ray(&self, col: f32, row: f32) -> V3 {
        let a = mul(self.right, (col - self.cols / 2.0) * self.step);
        let b = mul(self.up, (self.rows / 2.0 - row) * 2.0 * self.step);
        add(add(self.fwd, a), b)
    }

    /// Fractional terminal cell and view depth of a point, or `None` behind the camera.
    fn project(&self, p: V3) -> Option<(f32, f32, f32)> {
        let d = sub(p, self.pos);
        let t = dot(d, self.fwd);
        if t < NEAR {
            return None;
        }
        let col = dot(d, self.right) / (t * self.step) + self.cols / 2.0;
        let row = self.rows / 2.0 - dot(d, self.up) / (t * 2.0 * self.step);
        Some((col, row, t))
    }
}

enum Tex<'a> {
    Pix(Arc<PixFrame>),
    Cells(&'a Frame),
    Solid(Rgb),
}

/// Rectangle in space. `u` runs along sprite columns and `v` down sprite rows; they must be
/// perpendicular.
struct Quad<'a> {
    p: V3,
    u: V3,
    v: V3,
    tex: Tex<'a>,
    /// Paints transparent texels with this color, for sprites used as solid walls.
    backing: Option<Rgb>,
    id: Id,
    selected: bool,
    pickable: bool,
    /// Lower phases paint first; within phase 1 quads go far to near.
    phase: u8,
    /// Ground decals are coplanar, so paint order alone decides between them.
    depth_test: bool,
    depth: f32,
}

impl Quad<'_> {
    /// Glyph, ink and background at texture position (u, v); `None` when transparent.
    fn sample(&self, u: f32, v: f32) -> Option<(char, Rgb, Option<Rgb>)> {
        let found = match &self.tex {
            Tex::Solid(c) => Some((' ', *c, Some(*c))),
            Tex::Pix(f) => {
                let (x, y) = (((u * f.w as f32) as usize).min(f.w - 1), ((v * f.h as f32) as usize).min(f.h - 1));
                f.px[y * f.w + x].map(|c| {
                    let c = to_rgb(c);
                    (' ', c, Some(c))
                })
            }
            Tex::Cells(f) => {
                let (x, y) = (((u * f.w as f32) as usize).min(f.w - 1), ((v * f.h as f32) as usize).min(f.h - 1));
                let i = y * f.w + x;
                let (g, b) = (f.ch[i], f.bg[i]);
                (g != ' ' || b.is_some()).then(|| (g, to_rgb(f.fg[i]), b.map(to_rgb)))
            }
        };
        match (found, self.backing) {
            (None, Some(c)) => Some((' ', c, Some(c))),
            (Some((g, fg, None)), Some(c)) => Some((g, fg, Some(c))),
            (other, _) => other,
        }
    }

    /// Where a ray from `from` crosses this quad: distance along the ray and texture position.
    fn hit(&self, from: V3, ray: V3) -> Option<(f32, f32, f32)> {
        let n = cross(self.u, self.v);
        let denom = dot(n, ray);
        if denom.abs() < 1e-9 {
            return None;
        }
        let t = dot(n, sub(self.p, from)) / denom;
        if t < NEAR {
            return None;
        }
        let rel = sub(add(from, mul(ray, t)), self.p);
        let (u, v) = (dot(rel, self.u) / dot(self.u, self.u), dot(rel, self.v) / dot(self.v, self.v));
        ((0.0..1.0).contains(&u) && (0.0..1.0).contains(&v)).then_some((t, u, v))
    }
}

fn average(f: &PixFrame) -> Rgb {
    let (mut sum, mut n) = ([0u32; 3], 0u32);
    for c in f.px.iter().flatten() {
        let c = to_rgb(*c);
        for k in 0..3 {
            sum[k] += c[k] as u32;
        }
        n += 1;
    }
    let n = n.max(1);
    [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8]
}

fn scaled(c: Rgb, k: f32) -> Rgb {
    [(c[0] as f32 * k) as u8, (c[1] as f32 * k) as u8, (c[2] as f32 * k) as u8]
}

struct Scene<'a> {
    cam: &'a Camera,
    pixel: bool,
    selected: Option<Id>,
    parents: HashMap<Id, &'a Entity, std::hash::BuildHasherDefault<FastHasher>>,
    quads: Vec<Quad<'a>>,
}

impl<'a> Scene<'a> {
    fn tex(&self, e: &'a Entity) -> Tex<'a> {
        if self.pixel {
            Tex::Pix(e.sprite.pixel_frame(e.frame))
        } else {
            Tex::Cells(&e.sprite.frames[e.frame.min(e.sprite.frames.len() - 1)])
        }
    }

    /// The entity at the top of a `follow` chain; compound creatures share its placement.
    fn root(&self, e: &'a Entity) -> &'a Entity {
        let mut at = e;
        for _ in 0..4 {
            match at.follow.and_then(|f| self.parents.get(&f.parent)) {
                Some(p) => at = p,
                None => break,
            }
        }
        at
    }

    fn base(&self, e: &Entity, tex: Tex<'a>, p: V3, u: V3, v: V3) -> Quad<'a> {
        Quad {
            p,
            u,
            v,
            tex,
            backing: None,
            id: e.id,
            selected: self.selected == Some(e.id),
            pickable: e.z < BACKDROP_Z,
            phase: 1,
            depth_test: true,
            depth: 0.0,
        }
    }

    /// Camera-facing sprite whose bottom center rests on `foot`.
    fn billboard(&mut self, e: &'a Entity, foot: V3) {
        let (w, h) = (e.w(), e.h() * 2.0);
        // Cheap rejection before touching the texture; most of a large world is off screen.
        let Some((col, row, t)) = self.cam.project(foot) else { return };
        let span = w.max(h) / (t * self.cam.step);
        if col < -span || col > self.cam.cols + span || row < -span || row > self.cam.rows + span {
            return;
        }
        let u = mul(self.cam.right, w);
        let v = mul(self.cam.up, -h);
        let p = add(sub(foot, mul(u, 0.5)), mul(self.cam.up, h));
        let tex = self.tex(e);
        let mut q = self.base(e, tex, p, u, v);
        q.depth = t;
        self.quads.push(q);
    }

    fn ground_entity(&mut self, e: &'a Entity) {
        let (x, y, w, h) = (e.pos.0, e.pos.1, e.w(), e.h());
        if e.z >= BACKDROP_Z || FLAT_KINDS.contains(&e.kind) {
            let tex = self.tex(e);
            let mut q = self.base(e, tex, [x, 0.0, -2.0 * y], [w, 0.0, 0.0], [0.0, 0.0, -2.0 * h]);
            q.phase = 0;
            q.depth_test = false;
            q.depth = -(e.z as f32);
            self.quads.push(q);
        } else if BOX_KINDS.contains(&e.kind) {
            self.building(e);
        } else {
            let root = self.root(e);
            let root_foot = root.pos.1 + root.h();
            // Followers keep their offset as height, so a rider stays above its mount.
            let lift = ((root_foot - (y + h)) * 2.0).max(0.0);
            self.billboard(e, [x + w / 2.0, lift, -2.0 * root_foot]);
        }
    }

    /// Solid block on the sprite's footprint, with the sprite as its front wall.
    fn building(&mut self, e: &'a Entity) {
        let (x, y, w, d) = (e.pos.0, e.pos.1, e.w(), e.h() * 2.0);
        let height = d;
        let z0 = -2.0 * y;
        let z1 = z0 - d;
        let wall = average(&e.sprite.pixel_frame(e.frame));
        let faces: [(V3, V3, V3, f32); 4] = [
            ([x, height, z0], [w, 0.0, 0.0], [0.0, 0.0, -d], ROOF_LIGHT),
            ([x, height, z0], [0.0, 0.0, -d], [0.0, -height, 0.0], SIDE_LIGHT),
            ([x + w, height, z1], [0.0, 0.0, d], [0.0, -height, 0.0], SIDE_LIGHT),
            ([x + w, height, z0], [-w, 0.0, 0.0], [0.0, -height, 0.0], SIDE_LIGHT),
        ];
        let centre = [x + w / 2.0, height / 2.0, z0 - d / 2.0];
        let Some((_, _, t)) = self.cam.project(centre) else { return };
        for (p, u, v, shade) in faces {
            let mut q = self.base(e, Tex::Solid(scaled(wall, shade)), p, u, v);
            q.depth = t;
            self.quads.push(q);
        }
        let tex = self.tex(e);
        let mut front = self.base(e, tex, [x, height, z1], [w, 0.0, 0.0], [0.0, -height, 0.0]);
        front.backing = Some(wall);
        front.depth = t;
        self.quads.push(front);
    }

    fn tank_entity(&mut self, e: &'a Entity, depth_span: f32) {
        let (x, y, w, h) = (e.pos.0, e.pos.1, e.w(), e.h());
        if e.z >= BACKDROP_Z {
            let tex = self.tex(e);
            let mut q = self.base(e, tex, [x, -2.0 * y, depth_span], [w, 0.0, 0.0], [0.0, -2.0 * h, 0.0]);
            q.phase = 0;
            self.quads.push(q);
            self.sea_bed(e, depth_span);
        } else {
            let z = hash01(self.root(e).id) * depth_span * 0.9 + depth_span * 0.05;
            self.billboard(e, [x + w / 2.0, -2.0 * (y + h), z]);
        }
    }

    /// Carries the bottom of the back wall forward through the tank, as a stepped bed.
    fn sea_bed(&mut self, e: &'a Entity, depth_span: f32) {
        let f = e.sprite.pixel_frame(e.frame);
        if f.w == 0 || f.h == 0 {
            return;
        }
        let bottom = (e.pos.1 * 2.0) + f.h as f32;
        let at = |x: usize, y: usize| f.px[y * f.w + x].map(to_rgb);
        let Some(sand) = at(0, f.h - 1) else { return };
        let near = |c: Option<Rgb>| c.is_some_and(|c| (0..3).all(|k| (c[k] as i32 - sand[k] as i32).abs() < 48));
        // Rows from the top of the frame where the sand starts, per column.
        let surface: Vec<usize> = (0..f.w)
            .map(|x| {
                let mut y = f.h;
                while y > 0 && near(at(x, y - 1)) {
                    y -= 1;
                }
                y
            })
            .collect();
        let x0 = e.pos.0;
        let mut start = 0;
        while start < f.w {
            let mut end = start + 1;
            while end < f.w && surface[end] == surface[start] {
                end += 1;
            }
            let top = -(e.pos.1 * 2.0 + surface[start] as f32);
            let (xa, width) = (x0 + start as f32, (end - start) as f32);
            let mut floor = self.base(e, Tex::Solid(sand), [xa, top, 0.0], [width, 0.0, 0.0], [0.0, 0.0, depth_span]);
            floor.phase = 0;
            self.quads.push(floor);
            let drop = bottom + top;
            if drop > 0.0 {
                let cut = scaled(sand, SIDE_LIGHT);
                let mut face = self.base(e, Tex::Solid(cut), [xa, top, 0.0], [width, 0.0, 0.0], [0.0, -drop, 0.0]);
                face.phase = 0;
                self.quads.push(face);
            }
            if end < f.w {
                let other = -(e.pos.1 * 2.0 + surface[end] as f32);
                let (high, rise) = (top.max(other), (top - other).abs());
                let x = x0 + end as f32;
                let cut = scaled(sand, SIDE_LIGHT);
                let mut riser = self.base(e, Tex::Solid(cut), [x, high, 0.0], [0.0, 0.0, depth_span], [0.0, -rise, 0.0]);
                riser.phase = 0;
                self.quads.push(riser);
            }
            start = end;
        }
    }
}

/// Every surface in the world, in paint order.
fn build_scene<'a>(world: &'a World, cam: &'a Camera, pixel: bool, selected: Option<Id>) -> Vec<Quad<'a>> {
    let parents = world.ents.iter().filter(|e| e.alive).map(|e| (e.id, e)).collect();
    let mut s = Scene { cam, pixel, selected, parents, quads: Vec::with_capacity(world.ents.len()) };
    let kind = space(world);
    let depth_span = tank_depth(world);
    if matches!(kind, Space::Ground) {
        let bg = world.scenario.as_ref().map_or([0, 0, 0], |sc| to_rgb(sc.background()));
        let (w, d) = (world.w as f32, world.h as f32 * 2.0);
        s.quads.push(Quad {
            p: [0.0, 0.0, 0.0],
            u: [w, 0.0, 0.0],
            v: [0.0, 0.0, -d],
            tex: Tex::Solid(bg),
            backing: None,
            id: 0,
            selected: false,
            pickable: false,
            phase: 0,
            depth_test: false,
            depth: f32::NEG_INFINITY,
        });
    }
    for e in world.ents.iter().filter(|e| e.alive) {
        // Pixel mode draws text sprites afterwards as real glyphs.
        if pixel && e.sprite.overlay {
            continue;
        }
        match kind {
            Space::Ground => s.ground_entity(e),
            Space::Tank => s.tank_entity(e, depth_span),
        }
    }
    let mut quads = s.quads;
    quads.sort_by(|a, b| {
        a.phase.cmp(&b.phase).then_with(|| {
            if a.phase == 0 {
                a.depth.total_cmp(&b.depth).then(a.id.cmp(&b.id))
            } else {
                b.depth.total_cmp(&a.depth).then(a.id.cmp(&b.id))
            }
        })
    });
    quads
}

struct Target {
    w: i32,
    h: i32,
    ch: Vec<char>,
    fg: Vec<Rgb>,
    bg: Vec<Rgb>,
    depth: Vec<f32>,
}

fn mix(a: Rgb, b: Rgb, k: f32) -> Rgb {
    [
        (a[0] as f32 * (1.0 - k) + b[0] as f32 * k) as u8,
        (a[1] as f32 * (1.0 - k) + b[1] as f32 * k) as u8,
        (a[2] as f32 * (1.0 - k) + b[2] as f32 * k) as u8,
    ]
}

/// Fills the part of the target a quad covers. `rows_per_cell` is the target's vertical
/// resolution: 2 for half-block pixels, 1 for text.
fn raster(q: &Quad, cam: &Camera, t: &mut Target, rows_per_cell: f32, haze: Rgb, fog_span: f32) {
    let corners = [q.p, add(q.p, q.u), add(q.p, q.v), add(add(q.p, q.u), q.v)];
    let (mut x0, mut x1, mut y0, mut y1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    let mut clipped = false;
    for c in corners {
        match cam.project(c) {
            Some((col, row, _)) => {
                x0 = x0.min(col);
                x1 = x1.max(col);
                y0 = y0.min(row * rows_per_cell);
                y1 = y1.max(row * rows_per_cell);
            }
            None => clipped = true,
        }
    }
    // A corner behind the camera has no projection, so the whole screen must be searched.
    let (xa, xb, ya, yb) = if clipped {
        (0, t.w, 0, t.h)
    } else {
        ((x0.floor() as i32).max(0), (x1.ceil() as i32).min(t.w), (y0.floor() as i32).max(0), (y1.ceil() as i32).min(t.h))
    };
    for py in ya..yb {
        let row = (py as f32 + 0.5) / rows_per_cell;
        for px in xa..xb {
            let ray = cam.ray(px as f32 + 0.5, row);
            let Some((d, u, v)) = q.hit(cam.pos, ray) else { continue };
            let i = (py * t.w + px) as usize;
            if q.depth_test && d > t.depth[i] {
                continue;
            }
            let Some((g, fg, bg)) = q.sample(u, v) else { continue };
            let fog = ((d - cam.dist) / fog_span).clamp(0.0, MAX_FOG);
            let finish = |c: Rgb| {
                let c = if q.selected { lighten(c) } else { c };
                mix(c, haze, fog)
            };
            t.depth[i] = d;
            if g != ' ' {
                t.ch[i] = g;
                t.fg[i] = finish(fg);
            } else if bg.is_some() {
                t.ch[i] = ' ';
            }
            if let Some(b) = bg {
                t.bg[i] = finish(b);
            }
        }
    }
}

fn haze_of(world: &World) -> Rgb {
    match space(world) {
        Space::Tank => scaled(world.scenario.as_ref().map_or([0, 0, 0], |s| to_rgb(s.background())), 0.5),
        Space::Ground => [132, 168, 204],
    }
}

fn fog_span_of(world: &World) -> f32 {
    match space(world) {
        Space::Tank => tank_depth(world) * 1.5,
        Space::Ground => (world.w as f32).max(world.h as f32 * 2.0) * 1.5,
    }
}

pub fn draw_world_3d(world: &World, buf: &mut Buffer, area: Rect, view: &View, selected: Option<Id>) {
    let (cols, rows) = (area.width as i32, area.height as i32);
    if cols == 0 || rows == 0 {
        return;
    }
    let cam = Camera::new(world, view, cols, rows);
    let quads = build_scene(world, &cam, view.pixel, selected);
    let rows_per_cell = if view.pixel { 2 } else { 1 };
    let (w, h) = (cols, rows * rows_per_cell);
    let n = (w * h) as usize;
    let haze = haze_of(world);
    let mut t = Target { w, h, ch: vec![' '; n], fg: vec![[255; 3]; n], bg: vec![haze; n], depth: vec![f32::MAX; n] };
    let fog_span = fog_span_of(world);
    for q in &quads {
        raster(q, &cam, &mut t, rows_per_cell as f32, haze, fog_span);
    }
    let daylight = light(world);
    for sy in 0..rows {
        for sx in 0..cols {
            let cell = &mut buf[(area.x + sx as u16, area.y + sy as u16)];
            if view.pixel {
                let i = (sy * 2 * w + sx) as usize;
                cell.set_char('▀');
                cell.set_fg(color(shade(t.bg[i], daylight)));
                cell.set_bg(color(shade(t.bg[i + w as usize], daylight)));
            } else {
                let i = (sy * w + sx) as usize;
                cell.set_char(t.ch[i]);
                cell.set_fg(color(shade_ink(t.fg[i], daylight)));
                cell.set_bg(color(shade(t.bg[i], daylight)));
            }
        }
    }
    if view.pixel {
        draw_labels(world, buf, area, &cam, &t, daylight);
    }
}

/// Text sprites stay flat and readable, and only appear close up where they fit.
fn draw_labels(world: &World, buf: &mut Buffer, area: Rect, cam: &Camera, t: &Target, daylight: f32) {
    let kind = space(world);
    let depth_span = tank_depth(world);
    for e in world.ents.iter().filter(|e| e.alive && e.sprite.overlay) {
        let p = match kind {
            Space::Ground => [e.pos.0, 0.0, -2.0 * e.pos.1],
            Space::Tank => [e.pos.0, -2.0 * e.pos.1, hash01(e.id) * depth_span],
        };
        let Some((col, row, d)) = cam.project(p) else { continue };
        if d > cam.dist * 1.2 {
            continue;
        }
        let f = &e.sprite.frames[e.frame.min(e.sprite.frames.len() - 1)];
        let (ox, oy) = (col.round() as i32, row.round() as i32);
        for fy in 0..f.h as i32 {
            for fx in 0..f.w as i32 {
                let (x, y) = (ox + fx, oy + fy);
                let i = fy as usize * f.w + fx as usize;
                if f.ch[i] == ' ' || x < 0 || y < 0 || x >= t.w || y >= area.height as i32 {
                    continue;
                }
                // Buildings in front of the anchor hide the label.
                if t.depth[(y * 2 * t.w + x) as usize] < d - 1.0 {
                    continue;
                }
                let cell = &mut buf[(area.x + x as u16, area.y + y as u16)];
                cell.set_char(f.ch[i]);
                cell.set_fg(color(shade_ink(to_rgb(f.fg[i]), daylight)));
            }
        }
    }
}

/// Nearest visible entity under a terminal cell.
pub fn pick_3d(world: &World, view: &View, cols: i32, rows: i32, col: i32, row: i32) -> Option<Id> {
    let cam = Camera::new(world, view, cols, rows);
    let quads = build_scene(world, &cam, view.pixel, None);
    let ray = cam.ray(col as f32 + 0.5, row as f32 + 0.5);
    quads
        .iter()
        .filter_map(|q| {
            let (d, u, v) = q.hit(cam.pos, ray)?;
            q.sample(u, v)?;
            Some((d, q))
        })
        .filter(|(_, q)| q.pickable)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, q)| q.id)
}

/// World position under a terminal cell: on the ground, or mid-water in a tank.
pub fn unproject(world: &World, view: &View, cols: i32, rows: i32, col: i32, row: i32) -> Option<(f32, f32)> {
    let cam = Camera::new(world, view, cols, rows);
    let ray = cam.ray(col as f32 + 0.5, row as f32 + 0.5);
    let (axis, plane) = match space(world) {
        Space::Ground => (1, 0.0),
        Space::Tank => (2, tank_depth(world) / 2.0),
    };
    if ray[axis].abs() < 1e-6 {
        return None;
    }
    let t = (plane - cam.pos[axis]) / ray[axis];
    if t < NEAR {
        return None;
    }
    let hit = add(cam.pos, mul(ray, t));
    let y = match space(world) {
        Space::Ground => -hit[2] / 2.0,
        Space::Tank => -hit[1] / 2.0,
    };
    Some((hit[0].clamp(0.0, world.w as f32 - 1.0), y.clamp(0.0, world.h as f32 - 1.0)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenarios;

    fn tilted() -> View {
        View { x: 20.0, y: 10.0, zoom: 1, pixel: true, three_d: true, yaw: 0.4, pitch: 0.5 }
    }

    #[test]
    fn projection_inverts_rays() {
        let world = World::new(200, 60, 1, scenarios::make(1));
        let cam = Camera::new(&world, &tilted(), 100, 40);
        let p = add(cam.pos, mul(cam.ray(31.5, 12.25), 200.0));
        let (col, row, _) = cam.project(p).unwrap();
        assert!((col - 31.5).abs() < 0.01 && (row - 12.25).abs() < 0.01);
    }

    #[test]
    fn screen_center_unprojects_to_view_center() {
        for scenario in [0, 1] {
            let world = World::new(200, 60, 1, scenarios::make(scenario));
            let (x, y) = unproject(&world, &tilted(), 100, 40, 50, 20).unwrap();
            assert!((x - 70.0).abs() < 3.0 && (y - 30.0).abs() < 3.0, "{scenario}: {x},{y}");
        }
    }

    #[test]
    fn every_world_draws_in_both_modes() {
        for scenario in 0..scenarios::NAMES.len() {
            let mut world = World::new(200, 60, 3, scenarios::make(scenario));
            for _ in 0..30 {
                world.tick(1.0 / 30.0);
            }
            for pixel in [true, false] {
                let view = View { pixel, ..tilted() };
                let area = Rect::new(0, 0, 80, 24);
                let mut buf = Buffer::empty(area);
                draw_world_3d(&world, &mut buf, area, &view, None);
                assert!(pick_3d(&world, &view, 80, 24, 40, 12).is_none_or(|id| id != 0));
            }
        }
    }
}
