mod art;
mod engine;
mod scenarios;

use engine::render::{draw_world, hud_style, pick, View, MAX_ZOOM};
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

fn seed() -> u64 {
    if let Some(v) = std::env::var("SEED").ok().and_then(|v| v.parse().ok()) {
        return v;
    }
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| d.as_nanos() as u64)
}

fn build(index: usize, w: i32, h: i32) -> World {
    World::new(w, h, seed(), scenarios::make(index))
}

/// World for a scenario: an explicit size wins, then the scenario's own preference,
/// then twice the terminal.
fn new_world(index: usize, forced: Option<(i32, i32)>, term: (i32, i32)) -> World {
    let scenario = scenarios::make(index);
    let size = forced
        .or_else(|| scenario.preferred_size())
        .unwrap_or(((term.0 * 2).max(200), (term.1 * 2).max(60)));
    World::new(size.0, size.1, seed(), scenario)
}

fn scenario_index(name: &str) -> usize {
    scenarios::NAMES.iter().position(|n| n.starts_with(name)).unwrap_or(0)
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
    v
}

/// Headless mode: `--dump <scenario> <seconds> [cols rows]` renders one frame.
/// `WORLD=WxH`, `VIEW=x,y,zoom`, `PIXEL=1` and `OUT=file` (cell dump for tools/render.py) are optional.
fn dump(args: &[String]) {
    let index = scenario_index(&args[0]);
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
}

/// Headless throughput test: `--bench <scenario> <sim-seconds> [w h]`.
fn bench(args: &[String]) {
    let index = scenario_index(&args[0]);
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
        w,
        h,
        world.ents.len(),
        steps,
        el,
        steps as f64 / el,
        steps as f64 / el / 30.0,
        rayon::current_num_threads()
    );
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
    let mut index = pos.first().map_or(0, |a| scenario_index(a));

    let mut terminal = ratatui::init();
    execute!(std::io::stdout(), EnableMouseCapture)?;
    let size = terminal.size()?;
    let forced = flag(&args, "--world").and_then(|s| parse_size(&s));
    let term = (size.width as i32, size.height as i32);
    let mut world = new_world(index, forced, term);
    let mut view = View::new();
    view.pixel = args.iter().any(|a| a == "--pixel");
    let mut selected: Option<engine::Id> = None;
    let mut follow = false;
    let mut drag: Option<(i32, i32, bool)> = None;
    let mut paused = false;
    let mut speed = 1.0f32;
    let mut turbo = false;
    let mut debt = 0.0f32;
    let mut effective = 1.0f32;
    let mut last = Instant::now();

    let result = (|| -> std::io::Result<()> {
        loop {
            let size = terminal.size()?;
            let (cols, rows) = (size.width as i32, size.height as i32 - 1);
            if selected.is_some_and(|id| !world.ents.iter().any(|e| e.id == id && e.alive)) {
                selected = None;
                follow = false;
            }
            if let (true, Some(e)) = (follow, selected.and_then(|id| world.ents.iter().find(|e| e.id == id))) {
                view.x = e.pos.0 - (cols * view.zoom) as f32 / 2.0;
                view.y = e.pos.1 - (rows * view.zoom) as f32 / 2.0;
            }
            view.clamp(&world, cols, rows);

            terminal.draw(|f| {
                let area = f.area();
                let world_area = Rect { height: area.height.saturating_sub(1), ..area };
                draw_world(&world, f.buffer_mut(), world_area, &view, selected);
                let hud_area = Rect { y: area.y + world_area.height, height: 1, ..area };
                let extra = match selected.and_then(|id| world.ents.iter().find(|e| e.id == id)) {
                    Some(e) => format!(
                        "SELECTED {}#{} at ({:.0},{:.0}) vel ({:.1},{:.1}) data {:.1} state {}",
                        e.kind, e.id, e.pos.0, e.pos.1, e.vel.0, e.vel.1, e.data, e.state
                    ),
                    None => world.scenario.as_ref().map_or(String::new(), |s| {
                        s.hud(&world.globals, world.daylight)
                    }),
                };
                let line = format!(
                    " {} {}x{} | {}{} {:.1}x | view {:.0},{:.0} z{} {} | {} ents | {} | m:mode z/x:zoom arrows:pan t:turbo",
                    world.scenario_name(),
                    world.w,
                    world.h,
                    if paused { "PAUSED " } else { "" },
                    if turbo { "TURBO" } else { "" },
                    effective,
                    view.x,
                    view.y,
                    view.zoom,
                    if view.pixel { "pixel" } else { "ascii" },
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
                        match k.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => return Ok(()),
                            KeyCode::Char(' ') | KeyCode::Char('p') => paused = !paused,
                            KeyCode::Char('r') => {
                                world = new_world(index, forced, (cols, rows));
                                selected = None;
                            }
                            KeyCode::Tab => {
                                index = (index + 1) % scenarios::NAMES.len();
                                world = new_world(index, forced, (cols, rows));
                                selected = None;
                            }
                            KeyCode::Char(c @ '1'..='9') => {
                                let i = c as usize - '1' as usize;
                                if i < scenarios::NAMES.len() {
                                    index = i;
                                    world = new_world(index, forced, (cols, rows));
                                    selected = None;
                                }
                            }
                            KeyCode::Char('+') | KeyCode::Char('=') => speed = (speed * 2.0).min(1.0e6),
                            KeyCode::Char('-') => speed = (speed / 2.0).max(0.125),
                            KeyCode::Char('t') => turbo = !turbo,
                            KeyCode::Char('m') => view.pixel = !view.pixel,
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
                            KeyCode::Left | KeyCode::Char('a') => view.x -= sx,
                            KeyCode::Right | KeyCode::Char('d') => view.x += sx,
                            KeyCode::Up | KeyCode::Char('w') => view.y -= sy,
                            KeyCode::Down | KeyCode::Char('s') => view.y += sy,
                            _ => {}
                        }
                    }
                    Event::Mouse(m) => match m.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            drag = Some((m.column as i32, m.row as i32, false));
                        }
                        MouseEventKind::Drag(MouseButton::Left) => {
                            if let Some((px, py, _)) = drag {
                                let (dx, dy) = (m.column as i32 - px, m.row as i32 - py);
                                view.x -= (dx * view.zoom) as f32;
                                view.y -= (dy * view.zoom) as f32;
                                follow = false;
                                drag = Some((m.column as i32, m.row as i32, true));
                            }
                        }
                        MouseEventKind::Up(MouseButton::Left) => {
                            if let Some((_, _, moved)) = drag.take() {
                                if !moved {
                                    selected = pick(&world, &view, m.column as i32, m.row as i32);
                                }
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
                    let achieved = (world.time - sim_before) / real.max(0.001);
                    effective = effective * 0.8 + achieved * 0.2;
                }
            }
        }
    })();

    let _ = execute!(std::io::stdout(), DisableMouseCapture);
    ratatui::restore();
    result
}
