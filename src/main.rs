mod engine;
mod scenarios;

use engine::render::{draw_world, hud_style};
use engine::World;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;
use std::time::{Duration, Instant};

const FRAME: Duration = Duration::from_millis(33);

fn seed() -> u64 {
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
    draw_world(&world, &mut buf, area);
    for y in 0..h {
        let row: String = (0..w).map(|x| buf[(x, y)].symbol().to_string()).collect();
        println!("{}", row.trim_end());
    }
    println!("-- {} ents, globals {:?}", world.ents.len(), world.globals);
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
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
    let size = terminal.size()?;
    let mut world = build(index, size.width as i32, size.height as i32 - 1);
    let mut paused = false;
    let mut speed = 1.0f32;
    let mut last = Instant::now();

    let result = (|| -> std::io::Result<()> {
        loop {
            terminal.draw(|f| {
                let area = f.area();
                let world_area = Rect { height: area.height.saturating_sub(1), ..area };
                draw_world(&world, f.buffer_mut(), world_area);
                let hud_area = Rect { y: area.y + world_area.height, height: 1, ..area };
                let extra = world.scenario.as_ref().map_or(String::new(), |s| {
                    s.hud(&world.globals, world.daylight)
                });
                let line = format!(
                    " {} | {}{} x{:.1} | {} ents | {} | tab:next r:reset space:pause +/-:speed q:quit",
                    world.scenario_name(),
                    if paused { "PAUSED " } else { "" },
                    "",
                    speed,
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
                                world = build(index, w, h);
                            }
                            KeyCode::Char(c @ '1'..='9') => {
                                let i = c as usize - '1' as usize;
                                if i < scenarios::NAMES.len() {
                                    index = i;
                                    world = build(index, w, h);
                                }
                            }
                            KeyCode::Char('+') | KeyCode::Char('=') => speed = (speed * 1.5).min(16.0),
                            KeyCode::Char('-') => speed = (speed / 1.5).max(0.1),
                            _ => {}
                        }
                    }
                    Event::Resize(w, h) => world = build(index, w as i32, h as i32 - 1),
                    _ => {}
                }
            }

            if last.elapsed() >= FRAME {
                let dt = last.elapsed().as_secs_f32().min(0.1);
                last = Instant::now();
                if !paused {
                    // Sub-step at high speed so fast-forward stays stable.
                    let steps = speed.ceil().max(1.0) as i32;
                    for _ in 0..steps {
                        world.tick(dt * speed / steps as f32);
                    }
                }
            }
        }
    })();

    ratatui::restore();
    result
}
