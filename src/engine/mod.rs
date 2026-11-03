#![allow(dead_code, unused_imports)]

pub mod entity;
pub mod render;
pub mod rng;
pub mod sprite;
pub mod world;

pub use entity::{Behavior, Entity, Id, Info};
pub use rng::Rng;
pub use sprite::{Frame, Sprite, rgb};
pub use world::{Ctx, Globals, News, Scenario, World};
