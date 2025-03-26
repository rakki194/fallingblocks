#[cfg(test)]
mod tests {
    use crate::Time;
    use crate::components::*;
    use crate::game::{BOARD_HEIGHT, BOARD_WIDTH};
    use crate::systems::{input_system, spawn_tetromino};
    use bevy_ecs::prelude::*;

    // Helper function to create a test world
    fn setup_test_world() -> World {
        let mut world = World::new();

        // Initialize resources
        world.init_resource::<GameState>();

        let mut board = Board::new(BOARD_WIDTH, BOARD_HEIGHT);
        board.clear();
        world.insert_resource(board);

        let time = Time::new();
        world.insert_resource(time);

        let input = Input::default();
        world.insert_resource(input);

        let coyote_time = CoyoteTime::default();
        world.insert_resource(coyote_time);

        let screen_shake = ScreenShake {
            intensity: 0.0,
            duration: 0.0,
            current_offset: (0, 0),
            is_active: false,
        };
        world.insert_resource(screen_shake);

        world
    }

    #[test]
    fn test_spawn_tetromino() {
        let mut world = setup_test_world();

        // Spawn a tetromino
        spawn_tetromino(&mut world);

        // Check if a tetromino was spawned
        let tetromino_count = world.query::<&Tetromino>().iter(&world).count();

        assert_eq!(tetromino_count, 1);

        // Check if the tetromino has a position
        let position_count = world.query::<&Position>().iter(&world).count();

        assert_eq!(position_count, 1);
    }

    #[test]
    fn test_input_system() {
        let mut world = setup_test_world();

        // Spawn a tetromino
        spawn_tetromino(&mut world);

        // Store the initial position
        let initial_position = *world.query::<&Position>().iter(&world).next().unwrap();

        // Set the left input
        {
            let mut input = world.resource_mut::<Input>();
            input.left = true;
        }

        // Run the input system
        input_system(&mut world);

        // Get the new position
        let new_position = *world.query::<&Position>().iter(&world).next().unwrap();

        // The tetromino should have moved left
        assert_eq!(new_position.x, initial_position.x - 1);
        assert_eq!(new_position.y, initial_position.y);
    }

    #[test]
    fn test_rotation_system() {
        let mut world = setup_test_world();

        // Spawn a tetromino (not I, as it might look the same after rotation)
        world.spawn((
            Tetromino::new(TetrominoType::T),
            Position { x: 5, y: 5 },
            Ghost {
                position: Position { x: 5, y: 5 },
            },
        ));

        // Get the tetromino entity
        let entity = world
            .query_filtered::<Entity, With<Tetromino>>()
            .iter(&world)
            .next()
            .unwrap();

        // Get the initial rotation
        let initial_rotation = world.get::<Tetromino>(entity).unwrap().rotation;

        // Set the rotate input
        {
            let mut input = world.resource_mut::<Input>();
            input.rotate = true;
        }

        // Run the input system to handle rotation
        input_system(&mut world);

        // Get the new rotation
        let new_rotation = world.get::<Tetromino>(entity).unwrap().rotation;

        // The rotation should have changed
        assert_ne!(new_rotation, initial_rotation);
    }

    #[test]
    fn test_next_tetromino_preview() {
        let mut world = setup_test_world();

        // Initialize the next tetromino in GameState
        {
            let mut game_state = world.resource_mut::<GameState>();
            game_state.next_tetromino = Some(TetrominoType::T);
        }

        // Spawn a tetromino, which should use the next_tetromino we just set
        spawn_tetromino(&mut world);

        // Check if the active tetromino is of type T
        let active_tetromino_type = world
            .query::<&Tetromino>()
            .iter(&world)
            .next()
            .unwrap()
            .tetromino_type;

        assert_eq!(active_tetromino_type, TetrominoType::T);

        // Verify that a new next_tetromino was generated
        {
            let game_state = world.resource::<GameState>();
            assert!(game_state.next_tetromino.is_some());
        }

        // Get the entity ID of the first tetromino
        let entity = world
            .query_filtered::<Entity, With<Tetromino>>()
            .iter(&world)
            .next()
            .unwrap();

        // Despawn the first tetromino before spawning the second one
        world.despawn(entity);

        // Spawn another tetromino
        spawn_tetromino(&mut world);

        // Now only one tetromino should exist
        let tetromino_count = world.query::<&Tetromino>().iter(&world).count();
        assert_eq!(tetromino_count, 1);
    }
}
