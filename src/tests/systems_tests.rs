#[cfg(test)]
mod tests {
    use crate::Time;
    use crate::components::*;
    use crate::game::{BOARD_HEIGHT, BOARD_WIDTH};
    use crate::systems::*;
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
    fn test_gravity_system() {
        let mut world = setup_test_world();

        // Spawn a tetromino
        spawn_tetromino(&mut world);

        // Store the initial position
        let initial_position = *world.query::<&Position>().iter(&world).next().unwrap();

        // Run the gravity system with a large delta time
        gravity_system(&mut world, 10.0);

        // Get the new position
        let new_position = *world.query::<&Position>().iter(&world).next().unwrap();

        // The tetromino should have moved down
        assert!(new_position.y < initial_position.y);
    }

    #[test]
    fn test_collision_system() {
        let mut world = setup_test_world();

        // Spawn a tetromino
        spawn_tetromino(&mut world);

        // Get the tetromino entity
        let entity = world
            .query_filtered::<Entity, With<Tetromino>>()
            .iter(&world)
            .next()
            .unwrap();

        // Move the tetromino to the bottom
        {
            let mut position = world.get_mut::<Position>(entity).unwrap();
            position.y = 0;
        }

        // Run the collision system
        collision_system(&mut world);

        // The tetromino should have collided and been removed
        // A new tetromino should have been spawned
        let tetromino_count = world.query::<&Tetromino>().iter(&world).count();

        assert_eq!(tetromino_count, 1);

        // The old entity should have been despawned
        assert!(!world.contains_entity(entity));

        // There should be blocks on the board
        let board = world.resource::<Board>();
        let has_blocks = board.cells.iter().flatten().any(|cell| cell.is_some());
        assert!(has_blocks);
    }

    #[test]
    fn test_rotation_system() {
        let mut world = setup_test_world();

        // Spawn a tetromino
        spawn_tetromino(&mut world);

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
    fn test_screen_shake_system() {
        let mut world = setup_test_world();

        // Activate screen shake
        {
            let mut shake = world.resource_mut::<ScreenShake>();
            shake.intensity = 5.0;
            shake.duration = 1.0;
            shake.is_active = true;
        }

        // Initial offset should be (0, 0)
        assert_eq!(world.resource::<ScreenShake>().current_offset, (0, 0));

        // Run the screen shake system
        screen_shake_system(&mut world, 0.5);

        // The offset should have changed
        let current_offset = world.resource::<ScreenShake>().current_offset;
        assert_ne!(current_offset, (0, 0));

        // Run again to complete the duration
        screen_shake_system(&mut world, 0.6);

        // The screen shake should be inactive now
        assert!(!world.resource::<ScreenShake>().is_active);
    }
}
