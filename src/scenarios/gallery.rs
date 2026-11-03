use crate::engine::*;
use ratatui::style::Color;

/// Static showcase of every sprite, for inspecting art in both render modes.
#[derive(Default)]
pub struct Gallery;

impl Scenario for Gallery {
    fn name(&self) -> &'static str {
        "gallery"
    }

    fn background(&self) -> Color {
        Color::Rgb(46, 52, 64)
    }

    fn setup(&mut self, cx: &mut Ctx) {
        let mut items: Vec<(String, Sprite)> = vec![];
        for i in 0..super::zoo::species_count() {
            let (name, awake, asleep) = super::zoo::species_sprites(i);
            items.push((name.to_string(), awake));
            items.push((format!("{name} zz"), asleep));
        }
        let cell_w = items.iter().map(|(_, s)| s.w()).max().unwrap_or(8) as i32 + 4;
        let cell_h = items.iter().map(|(_, s)| s.h()).max().unwrap_or(4) as i32 + 3;
        let per_row = ((cx.w as i32) / cell_w).max(1);
        for (i, (label, sprite)) in items.into_iter().enumerate() {
            let (col, row) = (i as i32 % per_row, i as i32 / per_row);
            let (x, y) = (col * cell_w + 2, row * cell_h + 1);
            cx.spawn(Entity::new("exhibit", sprite).at(x as f32, y as f32).z(10));
            let tag = Sprite::text(&label, Color::Rgb(180, 190, 205));
            cx.spawn(Entity::new("label", tag).at(x as f32, (y + cell_h - 2) as f32).z(11));
        }
    }
}
