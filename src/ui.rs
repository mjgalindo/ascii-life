//! Overlays drawn on top of the world: ticker, log, inspector, minimap and help.

use crate::engine::render::{draw_world, View};
use crate::engine::{Id, World};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

const PANEL_BG: Color = Color::Rgb(16, 18, 30);
const PANEL_FG: Color = Color::Rgb(215, 220, 235);
const ACCENT: Color = Color::Rgb(255, 205, 110);
const MINIMAP_W: i32 = 44;

const HELP: &[&str] = &[
    "ascii-zoo",
    "",
    "tab 1-6    switch world          r     new world",
    "space      pause                 + -   speed",
    "t          turbo (all CPU)       q     quit",
    "",
    "arrows/wasd or drag   pan        z x / wheel  zoom",
    "g          fit world             m     ascii / pixel",
    "",
    "click      inspect a creature    f     follow it",
    "c          director: camera chases the action",
    "j          jump to latest news   l     news log",
    "n          minimap               ?     this help",
    "a          add things: pick a number, click to place more",
];

/// Fills a titled box and writes the lines into it, clipped to the screen.
fn panel(buf: &mut Buffer, r: Rect, title: &str, lines: &[String]) {
    let screen = buf.area;
    let r = r.intersection(screen);
    if r.width < 4 || r.height < 2 {
        return;
    }
    for y in r.y..r.y + r.height {
        for x in r.x..r.x + r.width {
            let cell = &mut buf[(x, y)];
            cell.set_char(' ');
            cell.set_fg(PANEL_FG);
            cell.set_bg(PANEL_BG);
        }
    }
    let width = r.width as usize - 2;
    let title: String = title.chars().take(width).collect();
    buf.set_string(r.x + 1, r.y, title, Style::default().fg(ACCENT).bg(PANEL_BG));
    for (i, line) in lines.iter().take(r.height as usize - 1).enumerate() {
        let text: String = line.chars().take(width).collect();
        buf.set_string(r.x + 1, r.y + 1 + i as u16, text, Style::default().fg(PANEL_FG).bg(PANEL_BG));
    }
}

/// Elapsed simulation time as minutes and seconds.
fn stamp(time: f64) -> String {
    let m = (time / 60.0) as u64;
    format!("{m:>3}m{:02}", (time % 60.0) as u64)
}

pub fn draw_ticker(world: &World, buf: &mut Buffer, r: Rect) {
    let style = Style::default().fg(ACCENT).bg(Color::Rgb(26, 24, 34));
    for x in r.x..r.x + r.width {
        let cell = &mut buf[(x, r.y)];
        cell.set_char(' ');
        cell.set_style(style);
    }
    if let Some(n) = world.news.back() {
        let text: String = format!(" {} {}", stamp(n.time), n.text).chars().take(r.width as usize).collect();
        buf.set_string(r.x, r.y, text, style);
    }
}

pub fn draw_log(world: &World, buf: &mut Buffer, area: Rect) {
    let h = (area.height / 2).clamp(4, 24);
    let w = (area.width * 3 / 5).clamp(20, 90);
    let r = Rect { x: area.x, y: area.y + area.height.saturating_sub(h), width: w, height: h };
    let lines: Vec<String> =
        world.news.iter().rev().take(h as usize - 1).map(|n| format!("{} {}", stamp(n.time), n.text)).collect();
    panel(buf, r, "NEWS (l closes, j jumps)", &lines);
}

pub fn draw_tools(buf: &mut Buffer, area: Rect, tools: &[&str], armed: Option<usize>) {
    let mut lines: Vec<String> = tools
        .iter()
        .enumerate()
        .map(|(i, t)| format!("{}{} {t}", if armed == Some(i) { ">" } else { " " }, i + 1))
        .collect();
    lines.push(if armed.is_some() { "click to place more".to_string() } else { "press a number".to_string() });
    let w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0).max(18) as u16 + 3;
    let r = Rect { x: area.x, y: area.y, width: w, height: lines.len() as u16 + 1 };
    panel(buf, r, "ADD (a or esc closes)", &lines);
}

pub fn draw_help(buf: &mut Buffer, area: Rect) {
    let lines: Vec<String> = HELP.iter().map(|s| s.to_string()).collect();
    let w = 56.min(area.width);
    let h = (HELP.len() as u16 + 2).min(area.height);
    let r = Rect { x: area.x + (area.width - w) / 2, y: area.y + (area.height - h) / 2, width: w, height: h };
    panel(buf, r, "HELP", &lines);
}

pub fn draw_inspector(world: &World, buf: &mut Buffer, area: Rect, id: Id) {
    let Some(e) = world.ents.iter().find(|e| e.id == id) else { return };
    let mut lines: Vec<String> = e
        .behavior
        .as_ref()
        .and_then(|b| b.describe(e))
        .map(|d| d.lines().map(str::to_string).collect())
        .unwrap_or_default();
    lines.push(format!("at {:.0},{:.0}", e.pos.0, e.pos.1));
    let w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0).max(e.kind.len() + 2) as u16 + 3;
    let w = w.min(area.width);
    let r = Rect { x: area.x + area.width - w, y: area.y, width: w, height: lines.len() as u16 + 1 };
    panel(buf, r, e.kind, &lines);
}

/// Pixel-rendered overview in the corner with the camera's viewport outlined.
pub fn draw_minimap(world: &World, buf: &mut Buffer, area: Rect, view: &View) {
    let zoom = (world.w + MINIMAP_W - 1) / MINIMAP_W;
    let rows = (world.h + zoom - 1) / zoom;
    if rows + 1 >= area.height as i32 || MINIMAP_W + 2 >= area.width as i32 {
        return;
    }
    let r = Rect {
        x: area.x + area.width - MINIMAP_W as u16,
        y: area.y + area.height - rows as u16,
        width: MINIMAP_W as u16,
        height: rows as u16,
    };
    let map = View { x: 0.0, y: 0.0, zoom, pixel: true };
    draw_world(world, buf, r, &map, None);

    let (cols, vrows) = (area.width as i32 * view.zoom, area.height as i32 * view.zoom);
    let x0 = (view.x as i32 / zoom).clamp(0, MINIMAP_W - 1);
    let y0 = (view.y as i32 / zoom).clamp(0, rows - 1);
    let x1 = ((view.x as i32 + cols) / zoom).clamp(0, MINIMAP_W - 1);
    let y1 = ((view.y as i32 + vrows) / zoom).clamp(0, rows - 1);
    let mut mark = |x: i32, y: i32, c: char| {
        let cell = &mut buf[(r.x + x as u16, r.y + y as u16)];
        cell.set_char(c);
        cell.set_fg(Color::Rgb(255, 255, 255));
    };
    for x in x0..=x1 {
        mark(x, y0, '-');
        mark(x, y1, '-');
    }
    for y in y0..=y1 {
        mark(x0, y, '|');
        mark(x1, y, '|');
    }
}
