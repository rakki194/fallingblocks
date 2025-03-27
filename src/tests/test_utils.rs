use crate::components::{Board, CoyoteTime, GameState, Input, ScreenShake};
use bevy_ecs::prelude::*;

/// Creates a test world with standard game resources initialized
pub fn create_test_world() -> World {
    let mut world = World::new();

    // Initialize standard resources
    world.insert_resource(Board::new(10, 20));
    world.insert_resource(GameState::default());
    world.insert_resource(Input::default());
    world.insert_resource(CoyoteTime::default());
    world.insert_resource(ScreenShake::default());

    world
}
