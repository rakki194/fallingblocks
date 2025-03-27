#![warn(clippy::all, clippy::pedantic)]

// Test modules
pub mod app_tests;
pub mod components_tests;
pub mod game_tests;
pub mod integration_tests;
//pub mod particles_tests;
//pub mod screenshake_tests;
pub mod sound_tests;
pub mod systems_tests;
pub mod time_tests;
pub mod ui_tests;

// Import test utilities
#[cfg(test)]
pub mod test_utils {
    use crate::app::App;
    use crate::components::{Board, GameState, Position, TetrominoType};
    use bevy_ecs::prelude::*;

    // Helper function to create a test world
    #[must_use]
    pub fn create_test_world() -> World {
        let mut world = World::new();
        world.init_resource::<GameState>();

        // Initialize game board
        let mut board = Board::new(crate::game::BOARD_WIDTH, crate::game::BOARD_HEIGHT);
        board.clear();
        world.insert_resource(board);

        // Initialize time resource
        world.insert_resource(crate::Time::new());

        world
    }

    // Helper function to create a test app
    #[must_use]
    pub fn create_test_app() -> App {
        App::new()
    }

    // Helper to check if a position is within board bounds
    #[must_use]
    pub fn is_within_bounds(pos: &Position) -> bool {
        pos.x >= 0
            && pos.x < crate::game::BOARD_WIDTH as i32
            && pos.y >= 0
            && pos.y < crate::game::BOARD_HEIGHT as i32
    }

    // Helper to fill a board with a specific pattern for testing
    pub fn fill_test_board(board: &mut Board, pattern: &[(usize, usize, TetrominoType)]) {
        for (x, y, tetromino_type) in pattern {
            if *x < board.width && *y < board.height {
                board.cells[*x][*y] = Some(*tetromino_type);
            }
        }
    }
}
