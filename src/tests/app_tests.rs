#![warn(clippy::all, clippy::pedantic)]

#[cfg(test)]
mod tests {
    use crate::app::App;
    use crate::components::{Board, GameState, Position, Tetromino, TetrominoType};
    use crate::game::{BOARD_HEIGHT, BOARD_WIDTH};

    #[test]
    fn test_app_creation() {
        let app = App::new();

        // Verify initial state
        assert_eq!(app.should_quit, false);
        assert_eq!(app.game_over, false);
        assert_eq!(app.score, 0);
        assert_eq!(app.level, 1);
        assert_eq!(app.lines_cleared, 0);

        // Check world was initialized with required resources
        assert!(app.world.contains_resource::<GameState>());
        assert!(app.world.contains_resource::<Board>());
        assert!(app.world.contains_resource::<crate::Time>());
    }

    #[test]
    fn test_board_dimensions() {
        let app = App::new();
        let board = app.world.resource::<Board>();

        assert_eq!(board.width, BOARD_WIDTH);
        assert_eq!(board.height, BOARD_HEIGHT);

        // Check board is initially cleared
        for x in 0..board.width {
            for y in 0..board.height {
                assert_eq!(board.cells[x][y], None);
            }
        }
    }

    #[test]
    fn test_get_render_blocks() {
        let mut app = App::new();

        // First check: initial state should have 4 blocks (active tetromino)
        let initial_blocks = app.get_render_blocks();
        assert!(
            !initial_blocks.is_empty(),
            "Should have initial tetromino blocks"
        );

        // Add a test tetromino to the world
        let tetromino = Tetromino::new(TetrominoType::I);
        let position = Position { x: 5, y: 5 };
        let tetromino_entity = app.world.spawn((tetromino, position)).id();

        // Add a block to the board at a known position
        {
            let mut board = app.world.resource_mut::<Board>();
            board.cells[2][3] = Some(TetrominoType::O);
        }

        // Now get all blocks
        let blocks = app.get_render_blocks();

        // We should have at least the board block plus the active tetromino blocks
        assert!(
            blocks.len() >= 5,
            "Should have board block + tetromino blocks"
        );

        // Clean up
        app.world.despawn(tetromino_entity);
    }

    #[test]
    fn test_game_state_resource() {
        let app = App::new();
        let game_state = app.world.resource::<GameState>();

        // Check default game state
        assert_eq!(game_state.score, 0);
        assert_eq!(game_state.level, 1);
        assert_eq!(game_state.lines_cleared, 0);
        assert_eq!(game_state.game_over, false);
        assert_eq!(game_state.tetris_count, 0);
        assert_eq!(game_state.t_spin_count, 0);
        assert_eq!(game_state.perfect_clear_count, 0);
        assert_eq!(game_state.combo_count, 0);
        assert_eq!(game_state.back_to_back, false);
    }
}
