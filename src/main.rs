mod engine;
mod scenarios;

use engine::render::{draw_world, hud_style, pick};
use engine::World;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    MouseButton, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::layout::Rect;
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

/// Headless mode: `--dump <scenario> <seconds> [w h]` prints one frame as plain text.
fn dump(args: &[String]) {
    let index = scenarios::NAMES.iter().position(|n| n.starts_with(args[0].as_str())).unwrap_or(0);
    let secs: f32 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(10.0);
    let w: u16 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(100);
    let h: u16 = args.get(3).and_then(|a| a.parse().ok()).unwrap_or(30);
    let mut world = build(index, w as i32, h as i32);
    for _ in 0..(secs * 30.0) as i32 {
        world.tick(1.0 / 30.0);
    }
    let area = Rect::new(0, 0, w, h);
    let mut buf = ratatui::buffer::Buffer::empty(area);
    draw_world(&world, &mut buf, area, None);
    for y in 0..h {
        let row: String = (0..w).map(|x| buf[(x, y)].symbol().to_string()).collect();
        println!("{}", row.trim_end());
    }
    if let Ok(v) = std::env::var("PICK") {
        let n: Vec<i32> = v.split(',').filter_map(|p| p.parse().ok()).collect();
        let hit = pick(&world, n[0], n[1]).and_then(|id| world.ents.iter().find(|e| e.id == id));
        println!("pick({},{}) = {:?}", n[0], n[1], hit.map(|e| (e.kind, e.id)));
    }
    println!("-- {} ents, globals {:?}", world.ents.len(), world.globals);
}

/// Headless throughput test: `--bench <scenario> <sim-seconds> [w h]`.
fn bench(args: &[String]) {
    let index = scenarios::NAMES.iter().position(|n| n.starts_with(args[0].as_str())).unwrap_or(0);
    let secs: f32 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(60.0);
    let w: i32 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(400);
    let h: i32 = args.get(3).and_then(|a| a.parse().ok()).unwrap_or(120);
    let mut world = build(index, w, h);
    let steps = (secs * 30.0) as u64;
    let start = Instant::now();
    for _ in 0..steps {
        world.tick(1.0 / 30.0);
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
    let arg = args.first().cloned();
    let mut index = arg
        .as_deref()
        .and_then(|a| scenarios::NAMES.iter().position(|n| n.starts_with(a)))
        .unwrap_or(0);

    let mut terminal = ratatui::init();
    execute!(std::io::stdout(), EnableMouseCapture)?;
    let mut selected: Option<engine::Id> = None;
    let size = terminal.size()?;
    let mut world = build(index, size.width as i32, size.height as i32 - 1);
    let mut paused = false;
    let mut speed = 1.0f32;
    let mut turbo = false;
    let mut debt = 0.0f32;
    let mut effective = 1.0f32;
    let mut last = Instant::now();

    let result = (|| -> std::io::Result<()> {
        loop {
            terminal.draw(|f| {
                let area = f.area();
                let world_area = Rect { height: area.height.saturating_sub(1), ..area };
                if selected.is_some_and(|id| !world.ents.iter().any(|e| e.id == id && e.alive)) {
                    selected = None;
                }
                draw_world(&world, f.buffer_mut(), world_area, selected);
                let hud_area = Rect { y: area.y + world_area.height, height: 1, ..area };
                let extra = match selected.and_then(|id| world.ents.iter().find(|e| e.id == id)) {
                    Some(e) => format!(
                        "SELECTED {}#{} at ({:.0},{:.0}) vel ({:.1},{:.1}) data {:.1} state {} age {:.0}s",
                        e.kind, e.id, e.pos.0, e.pos.1, e.vel.0, e.vel.1, e.data, e.state, e.age
                    ),
                    None => world.scenario.as_ref().map_or(String::new(), |s| {
                        s.hud(&world.globals, world.daylight)
                    }),
                };
                let line = format!(
                    " {} | {}{} | {:.1}x | {} ents | {} | tab:next click:inspect r:reset space:pause +/-:speed t:turbo q:quit",
                    world.scenario_name(),
                    if paused { "PAUSED " } else { "" },
                    if turbo { "TURBO" } else { "" },
                    effective,
                    world.ents.len(),
                    extra
                );
                f.render_widget(Paragraph::new(line).style(hud_style()), hud_area);
            })?;

            let timeout = FRAME.saturating_sub(last.elapsed());
            if event::poll(timeout)? {
                match event::read()? {
                    Event::Key(k) if k.kind == KeyEventKind::Press => {
                        let size = terminal.size()?;
                        let (w, h) = (size.width as i32, size.height as i32 - 1);
                        match k.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                                return Ok(());
                            }
                            KeyCode::Char(' ') | KeyCode::Char('p') => paused = !paused,
                            KeyCode::Char('r') => world = build(index, w, h),
                            KeyCode::Tab => {
                                index = (index + 1) % scenarios::NAMES.len();
                                { world = build(index, w, h); selected = None; }
                            }
                            KeyCode::Char(c @ '1'..='9') => {
                                let i = c as usize - '1' as usize;
                                if i < scenarios::NAMES.len() {
                                    index = i;
                                    { world = build(index, w, h); selected = None; }
                                }
                            }
                            KeyCode::Char('+') | KeyCode::Char('=') => speed = (speed * 2.0).min(1.0e6),
                            KeyCode::Char('t') => turbo = !turbo,
                            KeyCode::Char('-') => speed = (speed / 2.0).max(0.125),
                            _ => {}
                        }
                    }
                    Event::Mouse(m) if m.kind == MouseEventKind::Down(MouseButton::Left) => {
                        selected = pick(&world, m.column as i32, m.row as i32);
                    }
                    Event::Resize(w, h) => {
                        world = build(index, w as i32, h as i32 - 1);
                        selected = None;
                    }
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
