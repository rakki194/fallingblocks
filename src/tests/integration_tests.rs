#[cfg(test)]
mod tests {
    use crate::Time;
    use crate::app::App;
    use crate::components::{Board, GameState, Input, Position, Tetromino};
    use crate::systems::{clear_lines_system, collision_system, gravity_system, input_system};

    #[test]
    fn test_game_cycle() {
        // Create a new app
        let mut app = App::new();

        // Set up game state and time
        let mut time = Time::new();
        app.world.insert_resource(time);

        // Get initial score
        let initial_score = app.world.resource::<GameState>().score;

        // Run a game cycle
        // 1. Move tetromino down with hard drop
        {
            let mut input = app.world.resource_mut::<Input>();
            input.hard_drop = true;
        }

        input_system(&mut app.world);

        // 2. Run collision detection
        collision_system(&mut app.world);

        // 3. Clear lines
        clear_lines_system(&mut app.world);

        // 4. Run another cycle with a new tetromino
        {
            let mut input = app.world.resource_mut::<Input>();
            input.hard_drop = true;
        }

        input_system(&mut app.world);
        collision_system(&mut app.world);
        clear_lines_system(&mut app.world);

        // Check if the score increased
        let final_score = app.world.resource::<GameState>().score;
        assert!(final_score > initial_score);
    }

    #[test]
    fn test_game_over_state() {
        // Create a new app
        let mut app = App::new();

        // Fill most of the board to cause game over
        {
            let mut board = app.world.resource_mut::<Board>();
            // Fill all but the top row with blocks
            for x in 0..board.width {
                for y in 0..board.height - 1 {
                    board.cells[x][y] = Some(crate::components::TetrominoType::I);
                }
            }
        }

        // Run collision detection - should trigger game over
        collision_system(&mut app.world);

        // Check game over state
        let game_state = app.world.resource::<GameState>();
        assert!(game_state.game_over);
    }

    #[test]
    fn test_line_clearing() {
        // Create a new app
        let mut app = App::new();

        // Create a line to be cleared
        {
            let mut board = app.world.resource_mut::<Board>();
            // Fill the bottom row completely
            for x in 0..board.width {
                board.cells[x][0] = Some(crate::components::TetrominoType::I);
            }

            // Add some blocks in the next row
            for x in 0..board.width / 2 {
                board.cells[x][1] = Some(crate::components::TetrominoType::O);
            }
        }

        // Initial lines cleared
        let initial_lines = app.world.resource::<GameState>().lines_cleared;

        // Run line clearing
        clear_lines_system(&mut app.world);

        // Check that a line was cleared
        let final_lines = app.world.resource::<GameState>().lines_cleared;
        assert_eq!(final_lines, initial_lines + 1);

        // Check that the partial line moved down
        let board = app.world.resource::<Board>();
        let bottom_row_has_blocks = (0..board.width / 2).any(|x| board.cells[x][0].is_some());
        assert!(bottom_row_has_blocks);
    }

    #[test]
    fn test_pause_resume() {
        // Create a new app
        let mut app = App::new();

        // Pause the game for resize
        {
            let mut game_state = app.world.resource_mut::<GameState>();
            game_state.was_paused_for_resize = true;
        }

        // Store the initial position
        let initial_position = app
            .world
            .query::<&Position>()
            .iter(&app.world)
            .next()
            .unwrap()
            .clone();

        // Try to move the tetromino - should not move while paused
        {
            let mut input = app.world.resource_mut::<Input>();
            input.left = true;
        }

        input_system(&mut app.world);

        // Position should not have changed
        let paused_position = app
            .world
            .query::<&Position>()
            .iter(&app.world)
            .next()
            .unwrap()
            .clone();

        assert_eq!(initial_position.x, paused_position.x);

        // Resume the game
        {
            let mut game_state = app.world.resource_mut::<GameState>();
            game_state.was_paused_for_resize = false;
        }

        // Try moving again
        {
            let mut input = app.world.resource_mut::<Input>();
            input.left = true;
        }

        input_system(&mut app.world);

        // Position should now change
        let resumed_position = app
            .world
            .query::<&Position>()
            .iter(&app.world)
            .next()
            .unwrap()
            .clone();

        assert_eq!(resumed_position.x, initial_position.x - 1);
    }
}
