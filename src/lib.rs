pub mod app;
pub mod components;
pub mod game;
pub mod particles;
pub mod systems;
pub mod ui;

use bevy_ecs::prelude::Resource;
use std::time::{Duration, Instant};

#[derive(Resource, Debug, Clone)]
pub struct Time {
    delta: Duration,
    last_update: Instant,
}

impl Time {
    pub fn new() -> Self {
        Self {
            delta: Duration::default(),
            last_update: Instant::now(),
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        self.delta = now.duration_since(self.last_update);
        self.last_update = now;
    }

    pub fn delta_seconds(&self) -> f32 {
        self.delta.as_secs_f32()
    }
}
