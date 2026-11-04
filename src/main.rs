mod art;
mod engine;
mod scenarios;
mod ui;

use engine::render::{draw_world, hud_style, pick, View, MAX_ZOOM};
use engine::render3d::{pick_3d, unproject, PITCH_RANGE};
use engine::world::Space;
use engine::World;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    MouseButton, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Paragraph;
use std::time::{Duration, Instant};

const FRAME: Duration = Duration::from_millis(33);
/// Fixed simulation step; speed-ups run more steps, never longer ones.
const STEP: f32 = 1.0 / 30.0;
/// Share of each frame that fast-forward may spend simulating.
const SIM_BUDGET: Duration = Duration::from_millis(26);
/// Smallest world the scenarios can lay out.
const MIN_WORLD: (i32, i32) = (60, 30);
/// Screen-fitted worlds may go smaller, since their layout scales with the terminal.
const MIN_FITTED: (i32, i32) = (40, 16);
/// Quiet time after a terminal resize before a fitted world is rebuilt.
const RESIZE_SETTLE: Duration = Duration::from_millis(300);
/// Seconds the director stays on a subject before looking for another.
const DIRECTOR_SHOT: f32 = 14.0;
/// Radians the 3D camera orbits per key press.
const ORBIT_STEP: f32 = 0.12;

fn seed() -> u64 {
    if let Some(v) = std::env::var("SEED").ok().and_then(|v| v.parse().ok()) {
        return v;
    }
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| d.as_nanos() as u64)
}

fn build(index: usize, w: i32, h: i32) -> World {
    World::new(w.max(MIN_WORLD.0), h.max(MIN_WORLD.1), seed(), scenarios::make(index))
}

/// World for a scenario: an explicit size wins, then the scenario's own preference,
/// then twice the terminal.
fn new_world(index: usize, forced: Option<(i32, i32)>, term: (i32, i32)) -> World {
    let scenario = scenarios::make(index);
    if forced.is_none() && scenario.fits_terminal() {
        return World::new(term.0.max(MIN_FITTED.0), term.1.max(MIN_FITTED.1), seed(), scenario);
    }
    let size = forced
        .or_else(|| scenario.preferred_size())
        .unwrap_or(((term.0 * 2).max(200), (term.1 * 2).max(60)));
    World::new(size.0.max(MIN_WORLD.0), size.1.max(MIN_WORLD.1), seed(), scenario)
}

fn scenario_index(name: &str) -> Option<usize> {
    scenarios::NAMES.iter().position(|n| n.starts_with(name))
}

/// Resolves a world name or exits with the list of valid ones.
fn scenario_or_exit(name: &str) -> usize {
    scenario_index(name).unwrap_or_else(|| {
        eprintln!("unknown world '{name}'; choose from: {}", scenarios::NAMES.join(", "));
        std::process::exit(2);
    })
}

fn parse_size(s: &str) -> Option<(i32, i32)> {
    let (w, h) = s.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

/// Value of `--flag value` anywhere in the arguments.
fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

/// Positional arguments with flags and their values removed.
fn positional(args: &[String]) -> Vec<String> {
    let mut out = vec![];
    let mut skip = false;
    for a in args {
        if skip {
            skip = false;
        } else if a == "--world" {
            skip = true;
        } else if !a.starts_with("--") {
            out.push(a.clone());
        }
    }
    out
}

fn view_from_env() -> View {
    let mut v = View::new();
    if let Ok(s) = std::env::var("VIEW") {
        let n: Vec<f32> = s.split(',').filter_map(|p| p.parse().ok()).collect();
        if n.len() >= 3 {
            v.x = n[0];
            v.y = n[1];
            v.zoom = n[2] as i32;
        }
    }
    v.pixel = std::env::var("PIXEL").is_ok();
    if let Ok(s) = std::env::var("CAM3D") {
        let n: Vec<f32> = s.split(',').filter_map(|p| p.parse().ok()).collect();
        v.three_d = true;
        if n.len() >= 2 {
            v.yaw = n[0];
            v.pitch = n[1];
        }
    }
    v
}

/// Headless mode: `--dump <scenario> <seconds> [cols rows]` renders one frame.
/// `WORLD=WxH`, `VIEW=x,y,zoom`, `PIXEL=1`, `CAM3D=yaw,pitch` and `OUT=file` (cell dump for tools/render.py) are optional.
fn dump(args: &[String]) {
    let Some(name) = args.first() else {
        eprintln!("usage: --dump <world> [seconds] [cols] [rows]");
        std::process::exit(2);
    };
    let index = scenario_or_exit(name);
    let secs: f32 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(10.0);
    let cols: u16 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(100);
    let rows: u16 = args.get(3).and_then(|a| a.parse().ok()).unwrap_or(30);
    let (ww, wh) = std::env::var("WORLD")
        .ok()
        .and_then(|s| parse_size(&s))
        .unwrap_or((cols as i32, rows as i32));
    let mut world = build(index, ww, wh);
    for _ in 0..(secs * 30.0) as i32 {
        world.tick(STEP);
    }
    let mut view = view_from_env();
    view.clamp(&world, cols as i32, rows as i32);
    let area = Rect::new(0, 0, cols, rows);
    let mut buf = ratatui::buffer::Buffer::empty(area);
    draw_world(&world, &mut buf, area, &view, None);
    if let Ok(path) = std::env::var("OUT") {
        let mut out = format!("{cols} {rows}\n");
        let rgb = |c: Color| match c {
            Color::Rgb(r, g, b) => format!("{r},{g},{b}"),
            _ => "0,0,0".to_string(),
        };
        for y in 0..rows {
            for x in 0..cols {
                let c = &buf[(x, y)];
                out.push_str(&format!("{}\t{}\t{}\n", c.symbol(), rgb(c.fg), rgb(c.bg)));
            }
        }
        std::fs::write(path, out).unwrap();
    } else {
        for y in 0..rows {
            let row: String = (0..cols).map(|x| buf[(x, y)].symbol().to_string()).collect();
            println!("{}", row.trim_end());
        }
    }
    println!("-- {} ents, globals {:?}", world.ents.len(), world.globals);
    for n in world.news.iter().rev().take(12) {
        println!("   {:>7.0}s {}", n.time, n.text);
    }
    let fish: Vec<_> = world.ents.iter().filter(|e| e.kind == "fish").collect();
    if !fish.is_empty() {
        let hit = |a: &engine::Entity, b: &engine::Entity| {
            a.pos.0 < b.pos.0 + b.w() && b.pos.0 < a.pos.0 + a.w() && a.pos.1 < b.pos.1 + b.h() && b.pos.1 < a.pos.1 + a.h()
        };
        let overlapped = fish.iter().filter(|a| fish.iter().any(|b| a.id != b.id && hit(a, b))).count();
        println!("-- fish overlapping another: {overlapped}/{}", fish.len());
    }
}

/// Headless throughput test: `--bench <scenario> <sim-seconds> [w h]`.
fn bench(args: &[String]) {
    let Some(name) = args.first() else {
        eprintln!("usage: --bench <world> [sim-seconds] [w] [h]");
        std::process::exit(2);
    };
    let index = scenario_or_exit(name);
    let secs: f32 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(60.0);
    let w: i32 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(400);
    let h: i32 = args.get(3).and_then(|a| a.parse().ok()).unwrap_or(120);
    let mut world = build(index, w, h);
    let steps = (secs * 30.0) as u64;
    let start = Instant::now();
    for _ in 0..steps {
        world.tick(STEP);
    }
    let el = start.elapsed().as_secs_f64();
    println!(
        "{} {}x{}: {} entities, {} ticks in {:.2}s = {:.0} ticks/s ({:.0}x realtime), {} threads",
        world.scenario_name(),
        world.w,
        world.h,
        world.ents.len(),
        steps,
        el,
        steps as f64 / el,
        steps as f64 / el / 30.0,
        rayon::current_num_threads()
    );
    // Drawing cost of a full-screen frame, at native zoom and fitted to the whole world.
    let (cols, rows) = (200u16, 60u16);
    let area = Rect::new(0, 0, cols, rows);
    let fit = (world.w / cols as i32).max(world.h / rows as i32).max(1) + 1;
    for (label, zoom, pixel, three_d) in [("ascii z1", 1, false, false), ("pixel z1", 1, true, false), ("pixel fit", fit, true, false), ("3d z1", 1, true, true), ("3d fit", fit, true, true)] {
        let view = View { zoom, pixel, three_d, ..View::new() };
        let mut buf = ratatui::buffer::Buffer::empty(area);
        let start = Instant::now();
        for _ in 0..10 {
            draw_world(&world, &mut buf, area, &view, None);
        }
        println!("  draw {label}: {:.1} ms", start.elapsed().as_secs_f64() * 100.0);
    }
}

/// The locale in effect, following POSIX precedence.
fn locale() -> String {
    ["LC_ALL", "LC_CTYPE", "LANG"]
        .iter()
        .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
        .unwrap_or_default()
}

/// Pixel mode needs 24-bit color and the half-block glyph. Terminals cannot be queried for
/// these without a slow round trip, so this reads what the environment advertises.
fn pixel_capable(colorterm: &str, term: &str, term_program: &str, locale: &str) -> bool {
    let upper = locale.to_uppercase();
    let utf8 = upper.contains("UTF-8") || upper.contains("UTF8");
    let truecolor = matches!(colorterm.to_lowercase().as_str(), "truecolor" | "24bit")
        || term.contains("truecolor")
        || term.contains("24bit")
        || ["kitty", "alacritty", "wezterm", "foot", "ghostty"].iter().any(|t| term.contains(t))
        || matches!(term_program, "iTerm.app" | "WezTerm" | "vscode" | "ghostty");
    utf8 && truecolor && term != "linux" && term != "dumb"
}

/// Random creature on the move for the director to watch.
fn pick_subject(world: &mut World) -> Option<engine::Id> {
    let moving: Vec<engine::Id> = world
        .ents
        .iter()
        .filter(|e| e.alive && !e.scenery && e.behavior.is_some() && (e.vel.0 != 0.0 || e.vel.1 != 0.0))
        .map(|e| e.id)
        .collect();
    (!moving.is_empty()).then(|| *world.rng.pick(&moving))
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| a == "--bench") {
        bench(&args[1..]);
        return Ok(());
    }
    if args.first().is_some_and(|a| a == "--dump") {
        dump(&args[1..]);
        return Ok(());
    }
    let pos = positional(&args);
    let mut index = pos.first().map_or(0, |a| scenario_or_exit(a));

    let mut terminal = ratatui::init();
    execute!(std::io::stdout(), EnableMouseCapture)?;
    let size = terminal.size()?;
    let forced = flag(&args, "--world").and_then(|s| parse_size(&s));
    let term = (size.width as i32, (size.height as i32 - 2).max(1));
    let mut world = new_world(index, forced, term);
    let mut view = View::new();
    let var = |k: &str| std::env::var(k).unwrap_or_default();
    let capable = pixel_capable(&var("COLORTERM"), &var("TERM"), &var("TERM_PROGRAM"), &locale());
    view.pixel = args.iter().any(|a| a == "--pixel") || (capable && !args.iter().any(|a| a == "--ascii"));
    let mut selected: Option<engine::Id> = None;
    let mut follow = false;
    let mut drag: Option<(i32, i32, bool)> = None;
    let mut paused = false;
    let mut speed = 1.0f32;
    let mut turbo = false;
    let mut debt = 0.0f32;
    let mut effective = 1.0f32;
    let mut last = Instant::now();
    let (mut show_log, mut show_map, mut show_help) = (false, false, false);
    let mut director = false;
    let mut shot = 0.0f32;
    let mut subject: Option<engine::Id> = None;
    let mut news_seen = 0.0f64;
    let mut director_hold: Option<(f32, f32)> = None;
    let mut resized: Option<Instant> = None;
    let mut menu_open = false;
    let mut armed: Option<usize> = None;

    let result = (|| -> std::io::Result<()> {
        loop {
            let size = terminal.size()?;
            let (cols, rows) = (size.width as i32, (size.height as i32 - 2).max(1));
            // Worlds fitted to the screen are rebuilt once a resize has settled.
            let fitted = forced.is_none() && world.scenario.as_ref().is_some_and(|s| s.fits_terminal());
            if resized.is_some_and(|t| t.elapsed() >= RESIZE_SETTLE) {
                resized = None;
                if fitted && (world.w, world.h) != (cols.max(MIN_FITTED.0), rows.max(MIN_FITTED.1)) {
                    world = new_world(index, forced, (cols, rows));
                    selected = None;
                    follow = false;
                    subject = None;
                    news_seen = 0.0;
                }
            }
            if selected.is_some_and(|id| !world.ents.iter().any(|e| e.id == id && e.alive)) {
                selected = None;
                follow = false;
            }
            if let (true, Some(e)) = (follow, selected.and_then(|id| world.ents.iter().find(|e| e.id == id))) {
                view.x = e.pos.0 - (cols * view.zoom) as f32 / 2.0;
                view.y = e.pos.1 - (rows * view.zoom) as f32 / 2.0;
            }
            if director {
                shot -= FRAME.as_secs_f32();
                let fresh = world.news.back().filter(|n| n.time > news_seen && n.pos.is_some());
                let mut target = fresh.and_then(|n| n.pos);
                if let Some(n) = fresh {
                    news_seen = n.time;
                    shot = DIRECTOR_SHOT;
                    subject = None;
                }
                if target.is_none() && shot <= 0.0 {
                    subject = pick_subject(&mut world);
                    shot = DIRECTOR_SHOT;
                }
                if let Some(e) = subject.and_then(|id| world.ents.iter().find(|e| e.id == id)) {
                    target = Some(e.pos);
                }
                if let Some(p) = target.or(director_hold) {
                    director_hold = Some(p);
                    let tx = p.0 - (cols * view.zoom) as f32 / 2.0;
                    let ty = p.1 - (rows * view.zoom) as f32 / 2.0;
                    view.x += (tx - view.x) * 0.12;
                    view.y += (ty - view.y) * 0.12;
                }
            }
            view.clamp(&world, cols, rows);

            terminal.draw(|f| {
                let area = f.area();
                let world_area = Rect { height: area.height.saturating_sub(2), ..area };
                draw_world(&world, f.buffer_mut(), world_area, &view, selected);
                let ticker_area = Rect { y: area.y + world_area.height, height: 1, ..area };
                ui::draw_ticker(&world, f.buffer_mut(), ticker_area);
                if show_map {
                    ui::draw_minimap(&world, f.buffer_mut(), world_area, &view);
                }
                if show_log {
                    ui::draw_log(&world, f.buffer_mut(), world_area);
                }
                if let Some(id) = selected {
                    ui::draw_inspector(&world, f.buffer_mut(), world_area, id);
                }
                if menu_open {
                    let tools = world.scenario.as_ref().map_or(&[][..], |s| s.tools());
                    ui::draw_tools(f.buffer_mut(), world_area, tools, armed);
                }
                if show_help {
                    ui::draw_help(f.buffer_mut(), world_area);
                }
                let hud_area = Rect { y: area.y + world_area.height + 1, height: 1, ..area };
                let extra = world.scenario.as_ref().map_or(String::new(), |s| s.hud(&world.globals, world.daylight));
                let line = format!(
                    " {} | {}{}{} {:.1}x | z{} {} | {} ents | {} | a:add ?:help",
                    world.scenario_name(),
                    if paused { "PAUSED " } else { "" },
                    if turbo { "TURBO " } else { "" },
                    if director { "DIRECTOR " } else { "" },
                    effective,
                    view.zoom,
                    if view.three_d { format!("3d {}", if view.pixel { "pixel" } else { "ascii" }) } else { (if view.pixel { "pixel" } else { "ascii" }).to_string() },
                    world.ents.len(),
                    extra
                );
                f.render_widget(Paragraph::new(line).style(hud_style()), hud_area);
            })?;

            let timeout = FRAME.saturating_sub(last.elapsed());
            if event::poll(timeout)? {
                match event::read()? {
                    Event::Key(k) if k.kind == KeyEventKind::Press => {
                        let fast = if k.modifiers.contains(KeyModifiers::SHIFT) { 6.0 } else { 1.0 };
                        let (sx, sy) = (view.zoom as f32 * 4.0 * fast, view.zoom as f32 * 2.0 * fast);
                        let (mx, my) = (cols / 2, rows / 2);
                        let tools = world.scenario.as_ref().map_or(0, |s| s.tools().len());
                        let ground = world.scenario.as_ref().is_some_and(|s| s.space() == Space::Ground);
                        match k.code {
                            KeyCode::Esc if menu_open => {
                                menu_open = false;
                                armed = None;
                            }
                            KeyCode::Char('a') if tools > 0 => {
                                menu_open = !menu_open;
                                armed = None;
                            }
                            KeyCode::Char(c @ '1'..='9') if menu_open => {
                                let i = c as usize - '1' as usize;
                                if i < tools {
                                    armed = Some(i);
                                    // Open water at screen center is the natural drop point.
                                    let x = view.x + (cols * view.zoom) as f32 / 2.0;
                                    let y = view.y + (rows * view.zoom) as f32 / 2.0;
                                    world.place(i, x, y);
                                }
                            }
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => return Ok(()),
                            KeyCode::Char(' ') | KeyCode::Char('p') => paused = !paused,
                            KeyCode::Char('r') => {
                                world = new_world(index, forced, (cols, rows));
                                selected = None;
                                armed = None;
                                menu_open = false;
                                follow = false;
                                subject = None;
                                news_seen = 0.0;
                            }
                            KeyCode::Tab => {
                                index = (index + 1) % scenarios::NAMES.len();
                                world = new_world(index, forced, (cols, rows));
                                selected = None;
                                armed = None;
                                menu_open = false;
                                follow = false;
                                subject = None;
                                news_seen = 0.0;
                            }
                            KeyCode::Char(c @ '1'..='9') => {
                                let i = c as usize - '1' as usize;
                                if i < scenarios::NAMES.len() {
                                    index = i;
                                    world = new_world(index, forced, (cols, rows));
                                    selected = None;
                                    armed = None;
                                    menu_open = false;
                                    follow = false;
                                    subject = None;
                                    news_seen = 0.0;
                                }
                            }
                            KeyCode::Char('+') | KeyCode::Char('=') => speed = (speed * 2.0).min(1.0e6),
                            KeyCode::Char('-') => speed = (speed / 2.0).max(0.125),
                            KeyCode::Char('t') => turbo = !turbo,
                            KeyCode::Char('m') => view.pixel = !view.pixel,
                            KeyCode::Char('v') => view.three_d = !view.three_d,
                            KeyCode::Char(',') if view.three_d => view.yaw -= ORBIT_STEP,
                            KeyCode::Char('.') if view.three_d => view.yaw += ORBIT_STEP,
                            KeyCode::Char(';') if view.three_d => {
                                view.pitch = (view.pitch - ORBIT_STEP).clamp(PITCH_RANGE.0, PITCH_RANGE.1)
                            }
                            KeyCode::Char('\'') if view.three_d => {
                                view.pitch = (view.pitch + ORBIT_STEP).clamp(PITCH_RANGE.0, PITCH_RANGE.1)
                            }
                            KeyCode::Char('x') | KeyCode::Char(']') => view.zoom_at(view.zoom * 2, mx, my),
                            KeyCode::Char('z') | KeyCode::Char('[') => view.zoom_at(view.zoom / 2, mx, my),
                            KeyCode::Char('g') => {
                                // Fit the whole world on screen.
                                let mut z = 1;
                                while z < MAX_ZOOM && (world.w > cols * z || world.h > rows * z) {
                                    z *= 2;
                                }
                                view.zoom = z;
                                view.x = 0.0;
                                view.y = 0.0;
                            }
                            KeyCode::Char('f') => follow = selected.is_some() && !follow,
                            KeyCode::Char('c') => {
                                director = !director;
                                shot = 0.0;
                                follow = false;
                            }
                            KeyCode::Char('l') => show_log = !show_log,
                            KeyCode::Char('n') => show_map = !show_map,
                            KeyCode::Char('?') | KeyCode::Char('h') => show_help = !show_help,
                            KeyCode::Char('j') => {
                                let spot = world.news.iter().rev().find_map(|n| n.pos);
                                if let Some(p) = spot {
                                    follow = false;
                                    view.x = p.0 - (cols * view.zoom) as f32 / 2.0;
                                    view.y = p.1 - (rows * view.zoom) as f32 / 2.0;
                                }
                            }
                            KeyCode::Left | KeyCode::Char('a') => view.pan(-sx, 0.0, ground),
                            KeyCode::Right | KeyCode::Char('d') => view.pan(sx, 0.0, ground),
                            KeyCode::Up | KeyCode::Char('w') => view.pan(0.0, -sy, ground),
                            KeyCode::Down | KeyCode::Char('s') => view.pan(0.0, sy, ground),
                            _ => {}
                        }
                    }
                    Event::Resize(..) => resized = Some(Instant::now()),
                    Event::Mouse(m) => match m.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            drag = Some((m.column as i32, m.row as i32, false));
                        }
                        MouseEventKind::Drag(MouseButton::Left) => {
                            if let Some((px, py, _)) = drag {
                                let (dx, dy) = (m.column as i32 - px, m.row as i32 - py);
                                let ground = world.scenario.as_ref().is_some_and(|s| s.space() == Space::Ground);
                                view.pan(-(dx * view.zoom) as f32, -(dy * view.zoom) as f32, ground);
                                follow = false;
                                drag = Some((m.column as i32, m.row as i32, true));
                            }
                        }
                        MouseEventKind::Up(MouseButton::Left) => {
                            if let (Some((_, _, false)), Some(tool)) = (drag, armed) {
                                drag = None;
                                let spot = if view.three_d {
                                    unproject(&world, &view, cols, rows, m.column as i32, m.row as i32)
                                } else {
                                    let x = view.x + (m.column as i32 * view.zoom) as f32;
                                    let y = view.y + (m.row as i32 * view.zoom) as f32;
                                    Some((x, y))
                                };
                                if let Some((x, y)) = spot {
                                    world.place(tool, x, y);
                                }
                            } else if let Some((_, _, false)) = drag.take() {
                                selected = if view.three_d {
                                    pick_3d(&world, &view, cols, rows, m.column as i32, m.row as i32)
                                } else {
                                    pick(&world, &view, m.column as i32, m.row as i32)
                                };
                            }
                        }
                        MouseEventKind::ScrollUp => {
                            view.zoom_at(view.zoom / 2, m.column as i32, m.row as i32)
                        }
                        MouseEventKind::ScrollDown => {
                            view.zoom_at(view.zoom * 2, m.column as i32, m.row as i32)
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }

            if last.elapsed() >= FRAME {
                let real = last.elapsed().as_secs_f32().min(0.1);
                last = Instant::now();
                if !paused {
                    let started = Instant::now();
                    let sim_before = world.time;
                    debt += real * speed;
                    // Turbo ignores the requested speed and simulates for the whole budget.
                    while (turbo || debt >= STEP) && started.elapsed() < SIM_BUDGET {
                        world.tick(STEP);
                        debt -= STEP;
                    }
                    // Past the budget the requested speed is unreachable; drop the backlog.
                    debt = if turbo { 0.0 } else { debt.clamp(0.0, 2.0 * STEP) };
                    let achieved = (world.time - sim_before) as f32 / real.max(0.001);
                    effective = effective * 0.8 + achieved * 0.2;
                }
            }
        }
    })();

    let _ = execute!(std::io::stdout(), DisableMouseCapture);
    ratatui::restore();
    result
}

#[cfg(test)]
mod tests {
    use super::pixel_capable;

    #[test]
    fn detects_pixel_support() {
        assert!(pixel_capable("truecolor", "xterm-256color", "", "en_US.UTF-8"));
        assert!(pixel_capable("", "xterm-kitty", "", "es_ES.utf8"));
        assert!(pixel_capable("", "xterm-256color", "iTerm.app", "en_US.UTF-8"));
        assert!(!pixel_capable("", "xterm-256color", "", "en_US.UTF-8"));
        assert!(!pixel_capable("truecolor", "xterm-256color", "", "C"));
        assert!(!pixel_capable("truecolor", "linux", "", "en_US.UTF-8"));
    }
}
