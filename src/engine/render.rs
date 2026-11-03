use super::world::World;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

const NIGHT: (f32, f32, f32) = (8.0, 10.0, 40.0);

/// Blends a color toward a night tint as daylight falls.
fn shade(c: Color, daylight: f32) -> Color {
    let Color::Rgb(r, g, b) = c else { return c };
    let k = (1.0 - daylight) * 0.7;
    let mix = |v: u8, n: f32| (v as f32 * (1.0 - k) + n * k) as u8;
    Color::Rgb(mix(r, NIGHT.0), mix(g, NIGHT.1), mix(b, NIGHT.2))
}

pub fn draw_world(world: &World, buf: &mut Buffer, area: Rect) {
    let bg = world.scenario.as_ref().map_or(Color::Black, |s| s.background());
    let bg = shade(bg, world.daylight);
    for y in 0..area.height {
        for x in 0..area.width {
            let c = &mut buf[(area.x + x, area.y + y)];
            c.set_char(' ');
            c.set_bg(bg);
        }
    }

    // Far-to-near: higher z is drawn first.
    let mut order: Vec<&_> = world.ents.iter().filter(|e| e.alive).collect();
    order.sort_by_key(|e| (-e.z, e.id));

    let (aw, ah) = (area.width as i32, area.height as i32);
    for e in order {
        let f = &e.sprite.frames[e.frame.min(e.sprite.frames.len() - 1)];
        let ox = e.pos.0.round() as i32;
        let oy = e.pos.1.round() as i32;
        if ox >= aw || oy >= ah || ox + f.w as i32 <= 0 || oy + f.h as i32 <= 0 {
            continue;
        }
        for fy in 0..f.h {
            let y = oy + fy as i32;
            if y < 0 || y >= ah {
                continue;
            }
            for fx in 0..f.w {
                let x = ox + fx as i32;
                if x < 0 || x >= aw {
                    continue;
                }
                let i = fy * f.w + fx;
                let ch = f.ch[i];
                let cell_bg = f.bg[i];
                if ch == ' ' && cell_bg.is_none() {
                    continue;
                }
                let cell = &mut buf[(area.x + x as u16, area.y + y as u16)];
                if ch != ' ' {
                    cell.set_char(ch);
                    cell.set_fg(shade(f.fg[i], world.daylight));
                }
                if let Some(b) = cell_bg {
                    cell.set_bg(shade(b, world.daylight));
                }
            }
        }
    }
}

pub fn hud_style() -> Style {
    Style::default().fg(Color::Rgb(180, 180, 190)).bg(Color::Rgb(20, 20, 28))
}
