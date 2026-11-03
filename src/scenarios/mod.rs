mod aquarium;
mod traffic;
mod village;
mod zoo;

use crate::engine::Scenario;

pub const NAMES: [&str; 5] = ["aquarium", "traffic", "zoo", "village", "city"];

pub fn make(index: usize) -> Box<dyn Scenario> {
    match index {
        1 => Box::new(traffic::Traffic::default()),
        2 => Box::new(zoo::Zoo::default()),
        3 => Box::new(village::Village::default()),
        4 => Box::new(traffic::Traffic::city()),
        _ => Box::new(aquarium::Aquarium::default()),
    }
}
