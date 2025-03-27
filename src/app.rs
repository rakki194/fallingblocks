#![warn(clippy::all, clippy::pedantic)]
#![allow(
    // Allow truncation when casting from usize to i32 since board dimensions are always small enough to fit in i32
    clippy::cast_possible_truncation,
    // Allow potential wrapping when casting between types as board coordinates are within reasonable ranges
    clippy::cast_possible_wrap
)]

use bevy_ecs::prelude::*;
use std::error;
use std::time::{Duration, Instant};

use crate::Time;
use crate::components::{Board, CoyoteTime, GameState, Input, ScreenShake, TetrominoType};
use crate::game::{BOARD_HEIGHT, BOARD_WIDTH};
use crate::menu::MenuRenderer;
use crate::menu_types::Menu;
use crate::sound::AudioState;
use crate::systems::spawn_tetromino;

pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

pub struct App {
    pub world: World,
    pub should_quit: bool,
    pub level: u32,
    pub lines_cleared: u32,
    pub menu: Menu,
    pub menu_renderer: MenuRenderer,
}

impl App {
    pub fn new() -> Self {
        let mut world = World::new();
        world.insert_resource(Time::new());
        world.insert_resource(AudioState::new());
        world.insert_resource(Input::default());
        world.insert_resource(GameState::default());
        world.insert_resource(ScreenShake::default());
        world.insert_resource(Board::new(BOARD_WIDTH, BOARD_HEIGHT));
        world.insert_resource(CoyoteTime::default());

        // Create the app instance
        let mut app = Self {
            world,
            should_quit: false,
            level: 1,
            lines_cleared: 0,
            menu: Menu::new(),
            menu_renderer: MenuRenderer::new(),
        };

        // Spawn initial tetromino
        spawn_tetromino(&mut app.world);

        app
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

    // Update app state from game state
    pub fn sync_game_state(&mut self) {
        let game_state = self.world.resource::<GameState>();
        self.level = game_state.level;
        self.lines_cleared = game_state.lines_cleared;
    }

    pub fn on_tick(&mut self) {
        // Update last key if needed
        let input = self.world.resource::<Input>();
        if input.left || input.right || input.down || input.rotate || input.hard_drop {
            let mut game_state = self.world.resource_mut::<GameState>();
            game_state.last_move = Instant::now();
        }
    }

    /// Reset the game state
    pub fn reset(&mut self) {
        // Get audio state before resetting world
        let audio_vol = self
            .world
            .get_resource::<AudioState>()
            .map(|audio| audio.get_volume());

        // Save current menu state
        let current_menu_state = self.menu.state.clone();

        // Reset game state
        let game_state = GameState::default();
        self.world.insert_resource(game_state);

        // Reset board
        let mut board = Board::new(BOARD_WIDTH, BOARD_HEIGHT);
        board.clear();
        self.world.insert_resource(board);

        // Reset input handler
        let input = Input::default();
        self.world.insert_resource(input);

        // Reset screen shake
        let screen_shake = ScreenShake::default();
        self.world.insert_resource(screen_shake);

        // Reset coyote time
        let coyote_time = CoyoteTime::default();
        self.world.insert_resource(coyote_time);

        // Restore audio state
        let mut audio_state = AudioState::new();
        if let Some(vol) = audio_vol {
            audio_state.set_volume(vol);
        }
        self.world.insert_resource(audio_state);

        // Reset menu renderer while preserving the menu state
        self.menu_renderer = MenuRenderer::new();
        self.menu = Menu::new();
        self.menu.state = current_menu_state;

        // Reset game stats
        self.level = 1;
        self.lines_cleared = 0;

        // Spawn initial tetromino
        spawn_tetromino(&mut self.world);
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
