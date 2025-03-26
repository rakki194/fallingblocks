#![warn(clippy::all, clippy::pedantic)]

use bevy_ecs::prelude::*;
use std::error;

use crate::components::{Board, CoyoteTime, GameState, Input, ScreenShake, TetrominoType};
use crate::game::{BOARD_HEIGHT, BOARD_WIDTH};
use crate::systems::spawn_tetromino;

pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

pub struct App {
    pub world: World,
    pub should_quit: bool,
    pub game_over: bool,
    pub score: u32,
    pub level: u32,
    pub lines_cleared: u32,
}

impl App {
    #[must_use] pub fn new() -> Self {
        let mut world = World::new();

        // Register components
        world.init_resource::<GameState>();

        // Initialize screen shake effect
        world.init_resource::<ScreenShake>();

        // Initialize game board
        let mut board = Board::new(BOARD_WIDTH, BOARD_HEIGHT);
        board.clear();
        world.insert_resource(board);

        // Initialize time resource
        world.insert_resource(crate::Time::new());

        // Initialize game state
        let game_state = GameState {
            next_tetromino: Some(TetrominoType::random()),
            ..Default::default()
        };
        world.insert_resource(game_state);

        // Initialize screen shake
        let screen_shake = ScreenShake {
            intensity: 0.0,
            duration: 0.0,
            current_offset: (0, 0),
            is_active: false,
        };
        world.insert_resource(screen_shake);

        // Initialize input state
        let input = Input::default();
        world.insert_resource(input);

        // Initialize coyote time
        let coyote_time = CoyoteTime::default();
        world.insert_resource(coyote_time);

        // Spawn initial tetromino
        spawn_tetromino(&mut world);

        Self {
            world,
            should_quit: false,
            game_over: false,
            score: 0,
            level: 1,
            lines_cleared: 0,
        }
    }

    pub fn get_render_blocks(
        &mut self,
    ) -> Vec<(
        crate::components::Position,
        crate::components::TetrominoType,
    )> {
        let mut blocks = Vec::new();

        // Get blocks from the board
        if let Some(board) = self.world.get_resource::<crate::components::Board>() {
            for x in 0..board.width {
                for y in 0..board.height {
                    if let Some(tetromino_type) = board.cells[x][y] {
                        blocks.push((
                            crate::components::Position {
                                x: x as i32,
                                y: y as i32,
                            },
                            tetromino_type,
                        ));
                    }
                }
            }
        }

        // Get blocks from active tetrominos
        let tetromino_blocks: Vec<_> = self
            .world
            .query::<(&crate::components::Tetromino, &crate::components::Position)>()
            .iter(&self.world)
            .flat_map(|(tetromino, pos)| {
                tetromino.get_blocks().into_iter().map(move |(dx, dy)| {
                    let block_pos = crate::components::Position {
                        x: pos.x + dx,
                        y: pos.y + dy,
                    };
                    (block_pos, tetromino.tetromino_type)
                })
            })
            .collect();

        blocks.extend(tetromino_blocks);
        blocks
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
