use super::sprite::Sprite;
use super::world::Ctx;

pub type Id = u64;

/// Kinds that are decoration by convention; they are skipped by neighbor queries
/// (override with `.scenery()` or by assigning the field).
const SCENERY_KINDS: &[&str] = &[
    "road", "ground", "building", "tree", "pen", "tank", "lamp", "well", "bench", "seaweed", "emote",
    "smoke", "rain", "snow", "trail", "house", "field", "chimney", "bubble", "kiosk", "label",
];

/// Per-entity logic. Every hook is optional; entities without a behavior simply drift with `vel`.
pub trait Behavior: Send {
    fn update(&mut self, _e: &mut Entity, _cx: &mut Ctx, _dt: f32) {}
    fn on_collide(&mut self, _e: &mut Entity, _other: &Info, _cx: &mut Ctx) {}
    fn on_death(&mut self, _e: &Entity, _cx: &mut Ctx) {}
}

/// Attaches an entity to a parent so compound objects (shark + teeth) move and die together.
#[derive(Clone, Copy)]
pub struct Follow {
    pub parent: Id,
    pub dx: f32,
    pub dy: f32,
}

pub struct Entity {
    pub id: Id,
    pub kind: &'static str,
    pub tags: u32,
    pub pos: (f32, f32),
    pub vel: (f32, f32),
    pub z: i32,
    pub sprite: Sprite,
    pub frame: usize,
    pub anim_fps: f32,
    pub anim_acc: f32,
    pub physical: bool,
    /// Collision box as (dx, dy, w, h) relative to `pos`; defaults to the sprite bounds.
    pub hitbox: Option<(i32, i32, i32, i32)>,
    pub die_offscreen: bool,
    pub lifetime: Option<f32>,
    pub age: f32,
    /// Free-form numeric/state slots exposed to neighbors through `Info`.
    pub data: f32,
    pub state: i32,
    pub follow: Option<Follow>,
    /// Decoration that no behavior needs to discover; kept out of the neighbor index.
    pub scenery: bool,
    pub alive: bool,
    pub behavior: Option<Box<dyn Behavior>>,
}

impl Entity {
    pub fn new(kind: &'static str, sprite: Sprite) -> Entity {
        Entity {
            id: 0,
            kind,
            tags: 0,
            pos: (0.0, 0.0),
            vel: (0.0, 0.0),
            z: 10,
            sprite,
            frame: 0,
            anim_fps: 0.0,
            anim_acc: 0.0,
            physical: false,
            hitbox: None,
            die_offscreen: false,
            lifetime: None,
            age: 0.0,
            data: 0.0,
            state: 0,
            follow: None,
            scenery: SCENERY_KINDS.contains(&kind),
            alive: true,
            behavior: None,
        }
    }

    pub fn at(mut self, x: f32, y: f32) -> Self {
        self.pos = (x, y);
        self
    }
    pub fn vel(mut self, vx: f32, vy: f32) -> Self {
        self.vel = (vx, vy);
        self
    }
    pub fn z(mut self, z: i32) -> Self {
        self.z = z;
        self
    }
    pub fn tags(mut self, t: u32) -> Self {
        self.tags = t;
        self
    }
    pub fn fps(mut self, fps: f32) -> Self {
        self.anim_fps = fps;
        self
    }
    pub fn physical(mut self) -> Self {
        self.physical = true;
        self
    }
    pub fn hitbox(mut self, dx: i32, dy: i32, w: i32, h: i32) -> Self {
        self.hitbox = Some((dx, dy, w, h));
        self.physical = true;
        self
    }
    pub fn die_offscreen(mut self) -> Self {
        self.die_offscreen = true;
        self
    }
    pub fn lifetime(mut self, secs: f32) -> Self {
        self.lifetime = Some(secs);
        self
    }
    pub fn follow(mut self, parent: Id, dx: f32, dy: f32) -> Self {
        self.follow = Some(Follow { parent, dx, dy });
        self
    }
    pub fn scenery(mut self) -> Self {
        self.scenery = true;
        self
    }
    pub fn with(mut self, b: impl Behavior + 'static) -> Self {
        self.behavior = Some(Box::new(b));
        self
    }

    pub fn set_sprite(&mut self, s: Sprite) {
        self.sprite = s;
        self.frame = 0;
    }

    pub fn w(&self) -> f32 {
        self.sprite.w() as f32
    }
    pub fn h(&self) -> f32 {
        self.sprite.h() as f32
    }
    pub fn cx(&self) -> f32 {
        self.pos.0 + self.w() / 2.0
    }
    pub fn cy(&self) -> f32 {
        self.pos.1 + self.h() / 2.0
    }
    pub fn kill(&mut self) {
        self.alive = false;
    }

    pub fn rect(&self) -> (i32, i32, i32, i32) {
        let (dx, dy, w, h) = self
            .hitbox
            .unwrap_or((0, 0, self.sprite.w() as i32, self.sprite.h() as i32));
        (self.pos.0.round() as i32 + dx, self.pos.1.round() as i32 + dy, w, h)
    }
}

/// Read-only view of an entity as of the start of the tick, handed to neighbors.
#[derive(Clone, Copy)]
pub struct Info {
    pub id: Id,
    pub kind: &'static str,
    pub tags: u32,
    pub pos: (f32, f32),
    pub vel: (f32, f32),
    pub rect: (i32, i32, i32, i32),
    pub z: i32,
    pub data: f32,
    pub state: i32,
    pub physical: bool,
}

impl Info {
    pub fn of(e: &Entity) -> Info {
        Info {
            id: e.id,
            kind: e.kind,
            tags: e.tags,
            pos: e.pos,
            vel: e.vel,
            rect: e.rect(),
            z: e.z,
            data: e.data,
            state: e.state,
            physical: e.physical,
        }
    }
}
