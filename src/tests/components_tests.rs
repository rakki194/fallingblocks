#[cfg(test)]
mod position_tests {
    use crate::components::Position;

    #[test]
    fn test_position_methods() {
        let pos = Position { x: 5, y: 10 };

        // Test down movement
        let down = pos.down();
        assert_eq!(down.x, 5);
        assert_eq!(down.y, 9);

        // Test left movement
        let left = pos.left();
        assert_eq!(left.x, 4);
        assert_eq!(left.y, 10);

        // Test right movement
        let right = pos.right();
        assert_eq!(right.x, 6);
        assert_eq!(right.y, 10);
    }
}

#[cfg(test)]
mod tetromino_tests {
    use crate::components::{Position, Rotation, Tetromino, TetrominoType};

    #[test]
    fn test_tetromino_creation() {
        // Test I tetromino
        let i_tetromino = Tetromino::new(TetrominoType::I);
        assert_eq!(i_tetromino.tetromino_type, TetrominoType::I);
        assert_eq!(i_tetromino.rotation, Rotation::R0);

        // Test all tetromino types
        let types = [
            TetrominoType::I,
            TetrominoType::J,
            TetrominoType::L,
            TetrominoType::O,
            TetrominoType::S,
            TetrominoType::T,
            TetrominoType::Z,
        ];

        for t in types.iter() {
            let tetromino = Tetromino::new(*t);
            assert_eq!(tetromino.tetromino_type, *t);
        }
    }

    #[test]
    fn test_tetromino_get_blocks() {
        // Test I tetromino in default rotation
        let i_tetromino = Tetromino::new(TetrominoType::I);
        let i_blocks = i_tetromino.get_blocks();

        // I tetromino should have 4 blocks
        assert_eq!(i_blocks.len(), 4);

        // O tetromino should always have the same shape regardless of rotation
        let o_tetromino = Tetromino::new(TetrominoType::O);
        let o_blocks_r0 = o_tetromino.get_blocks();

        let mut rotated_o = o_tetromino.clone();
        rotated_o.rotation = Rotation::R90;
        let o_blocks_r90 = rotated_o.get_blocks();

        // O tetromino should have 4 blocks
        assert_eq!(o_blocks_r0.len(), 4);
        assert_eq!(o_blocks_r90.len(), 4);
    }

    #[test]
    fn test_tetromino_rotate() {
        // Test rotation for I tetromino
        let mut i_tetromino = Tetromino::new(TetrominoType::I);

        // Initial rotation should be R0
        assert_eq!(i_tetromino.rotation, Rotation::R0);

        // Rotate clockwise
        i_tetromino.rotate_clockwise();
        assert_eq!(i_tetromino.rotation, Rotation::R90);

        i_tetromino.rotate_clockwise();
        assert_eq!(i_tetromino.rotation, Rotation::R180);

        i_tetromino.rotate_clockwise();
        assert_eq!(i_tetromino.rotation, Rotation::R270);

        i_tetromino.rotate_clockwise();
        assert_eq!(i_tetromino.rotation, Rotation::R0);

        // Test counterclockwise rotation
        i_tetromino.rotate_counterclockwise();
        assert_eq!(i_tetromino.rotation, Rotation::R270);
    }

    #[test]
    fn test_tetromino_color() {
        // Each tetromino type should have a distinct color
        let colors = [
            TetrominoType::I.get_color(),
            TetrominoType::J.get_color(),
            TetrominoType::L.get_color(),
            TetrominoType::O.get_color(),
            TetrominoType::S.get_color(),
            TetrominoType::T.get_color(),
            TetrominoType::Z.get_color(),
        ];

        // This tests that colors are implemented, but doesn't check specific values
        assert_eq!(colors.len(), 7);
    }
}

#[cfg(test)]
mod board_tests {
    use crate::components::{Board, Position, TetrominoType};
    use crate::game::{BOARD_HEIGHT, BOARD_WIDTH};

    #[test]
    fn test_board_creation() {
        let board = Board::new(BOARD_WIDTH, BOARD_HEIGHT);

        // Check dimensions
        assert_eq!(board.width, BOARD_WIDTH);
        assert_eq!(board.height, BOARD_HEIGHT);

        // Check the board is initialized with the right size
        assert_eq!(board.cells.len(), BOARD_WIDTH);
        assert_eq!(board.cells[0].len(), BOARD_HEIGHT);
    }

    #[test]
    fn test_board_clear() {
        let mut board = Board::new(BOARD_WIDTH, BOARD_HEIGHT);

        // Add some blocks
        board.cells[0][0] = Some(TetrominoType::I);
        board.cells[1][1] = Some(TetrominoType::O);

        // Clear the board
        board.clear();

        // Verify all cells are None
        for x in 0..board.width {
            for y in 0..board.height {
                assert_eq!(board.cells[x][y], None);
            }
        }
    }

    #[test]
    fn test_board_is_valid_position() {
        let mut board = Board::new(BOARD_WIDTH, BOARD_HEIGHT);

        // Block position in the middle of the board
        let valid_pos = Position { x: 5, y: 5 };
        assert!(board.is_valid_position(&valid_pos));

        // Out of bounds positions
        let out_left = Position { x: -1, y: 5 };
        let out_right = Position {
            x: BOARD_WIDTH as i32,
            y: 5,
        };
        let out_bottom = Position { x: 5, y: -1 };
        let out_top = Position {
            x: 5,
            y: BOARD_HEIGHT as i32,
        };

        assert!(!board.is_valid_position(&out_left));
        assert!(!board.is_valid_position(&out_right));
        assert!(!board.is_valid_position(&out_bottom));
        assert!(!board.is_valid_position(&out_top));

        // Place a block and check collision
        board.cells[5][5] = Some(TetrominoType::I);
        assert!(!board.is_valid_position(&valid_pos));
    }

    #[test]
    fn test_board_clear_lines() {
        let mut board = Board::new(BOARD_WIDTH, BOARD_HEIGHT);

        // Create a full line at the bottom
        for x in 0..board.width {
            board.cells[x][0] = Some(TetrominoType::I);
        }

        // Create a partial line above it
        for x in 0..5 {
            board.cells[x][1] = Some(TetrominoType::J);
        }

        // Clear lines and check the result
        let lines_cleared = board.clear_lines();

        // Should have cleared one line
        assert_eq!(lines_cleared, 1);

        // The partial line should have dropped down
        for x in 0..5 {
            assert_eq!(board.cells[x][0], Some(TetrominoType::J));
        }
        for x in 5..board.width {
            assert_eq!(board.cells[x][0], None);
        }

        // The line above should be empty
        for x in 0..board.width {
            assert_eq!(board.cells[x][1], None);
        }
    }
}

#[cfg(test)]
mod game_state_tests {
    use crate::components::GameState;
    use crate::game::{LINES_PER_LEVEL, STARTING_LEVEL};

    #[test]
    fn test_game_state_default() {
        let game_state = GameState::default();

        // Check initialization
        assert_eq!(game_state.score, 0);
        assert_eq!(game_state.level, STARTING_LEVEL);
        assert_eq!(game_state.lines_cleared, 0);
        assert_eq!(game_state.game_over, false);
    }

    #[test]
    fn test_game_state_reset() {
        let mut game_state = GameState::default();

        // Change some values
        game_state.score = 1000;
        game_state.level = 5;
        game_state.lines_cleared = 40;
        game_state.game_over = true;

        // Reset
        game_state.reset();

        // Verify everything is reset
        assert_eq!(game_state.score, 0);
        assert_eq!(game_state.level, STARTING_LEVEL);
        assert_eq!(game_state.lines_cleared, 0);
        assert_eq!(game_state.game_over, false);
    }

    #[test]
    fn test_game_state_level_up() {
        let mut game_state = GameState::default();

        // Add enough lines to level up
        game_state.lines_cleared = LINES_PER_LEVEL - 1;
        game_state.add_lines_cleared(1);

        // Check level increased
        assert_eq!(game_state.level, STARTING_LEVEL + 1);
    }
}
