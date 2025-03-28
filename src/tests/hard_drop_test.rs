use crate::Time;
use crate::components::{
    Board, CoyoteTime, GameState, Ghost, Input, Position, ScreenShake, Tetromino,
};
use crate::sound::AudioState;
use crate::systems::{game_tick_system, input_system, spawn_tetromino};
use bevy_ecs::prelude::*;

#[test]
fn test_hard_drop() {
    // Create a world
    let mut world = World::new();

    // Set up the resources
    world.insert_resource(GameState::default());
    world.insert_resource(Board::new(10, 20)); // Standard 10x20 board
    world.insert_resource(Input::default());
    world.insert_resource(ScreenShake::default());
    world.insert_resource(CoyoteTime::default());
    world.insert_resource(AudioState::new());
    world.insert_resource(Time::new());

    // Set the hard_drop_released flag to true initially
    {
        let mut input = world.resource_mut::<Input>();
        input.hard_drop_released = true;
    }

    // Spawn a tetromino
    spawn_tetromino(&mut world);

    // Verify the tetromino was spawned
    let has_tetromino = !world
        .query::<(&Tetromino, &Position, &Ghost)>()
        .iter(&world)
        .collect::<Vec<_>>()
        .is_empty();
    assert!(has_tetromino, "Tetromino should be spawned");

    // Save the initial position of the tetromino
    let initial_position = {
        let mut query = world.query::<&Position>();
        *query.iter(&world).next().unwrap()
    };

    // Run one game tick to update the ghost position
    game_tick_system(&mut world, 0.016);

    // Verify the ghost is properly positioned
    let ghost_position = {
        let mut query = world.query::<&Ghost>();
        query.iter(&world).next().unwrap().position
    };

    // Ghost should have the same X position but lower Y position
    assert_eq!(
        ghost_position.x, initial_position.x,
        "Ghost X position should match tetromino X position"
    );
    assert!(
        ghost_position.y > initial_position.y,
        "Ghost Y position should be lower than tetromino"
    );

    // Simulate pressing the hard drop key
    {
        let mut input = world.resource_mut::<Input>();
        input.hard_drop = true;
    }

    // Run the input system
    input_system(&mut world);

    // Verify that a new tetromino was spawned (old one was locked and removed)
    let has_new_tetromino = !world
        .query::<(&Tetromino, &Position, &Ghost)>()
        .iter(&world)
        .collect::<Vec<_>>()
        .is_empty();
    assert!(
        has_new_tetromino,
        "A new tetromino should be spawned after hard drop"
    );

    // Check that there are some blocks in the board (from the locked tetromino)
    let board = world.resource::<Board>();
    let has_blocks =
        (0..board.width).any(|x| (0..board.height).any(|y| board.cells[x][y].is_some()));
    assert!(has_blocks, "Board should have blocks after hard drop");

    // Check that the hard drop score was updated
    let game_state = world.resource::<GameState>();
    assert!(
        game_state.hard_drop_distance > 0,
        "Hard drop distance should be recorded"
    );
}

#[test]
fn test_ghost_position_update() {
    // Create a world
    let mut world = World::new();

    // Set up the resources
    world.insert_resource(GameState::default());
    world.insert_resource(Board::new(10, 20)); // Standard 10x20 board
    world.insert_resource(Input::default());
    world.insert_resource(ScreenShake::default());
    world.insert_resource(CoyoteTime::default());
    world.insert_resource(AudioState::new());
    world.insert_resource(Time::new());

    // Spawn a tetromino
    spawn_tetromino(&mut world);

    // Run game tick to update ghost position
    game_tick_system(&mut world, 0.016);

    // Get the tetromino and ghost positions
    let (tetromino_pos, ghost_pos) = {
        let mut query = world.query::<(&Position, &Ghost)>();
        let (pos, ghost) = query.iter(&world).next().unwrap();
        (*pos, ghost.position)
    };

    // Ghost should have the same X position but lower Y position
    assert_eq!(
        ghost_pos.x, tetromino_pos.x,
        "Ghost X position should match tetromino X position"
    );
    assert!(
        ghost_pos.y > tetromino_pos.y,
        "Ghost Y position should be lower than tetromino"
    );

    // Move the tetromino horizontally
    {
        let mut query = world.query::<(Entity, &Position)>();
        let (entity, _) = query.iter(&world).next().unwrap();
        let new_position = Position {
            x: tetromino_pos.x + 1,
            y: tetromino_pos.y,
        };
        world.entity_mut(entity).insert(new_position);
    }

    // Run game tick again to update ghost position
    game_tick_system(&mut world, 0.016);

    // Get the updated tetromino and ghost positions
    let (updated_tetromino_pos, updated_ghost_pos) = {
        let mut query = world.query::<(&Position, &Ghost)>();
        let (pos, ghost) = query.iter(&world).next().unwrap();
        (*pos, ghost.position)
    };

    // Ghost should have moved horizontally with the tetromino
    assert_eq!(
        updated_ghost_pos.x, updated_tetromino_pos.x,
        "Ghost X should move with tetromino X"
    );
    assert!(
        updated_ghost_pos.y > updated_tetromino_pos.y,
        "Ghost Y position should still be lower than tetromino"
    );
}
