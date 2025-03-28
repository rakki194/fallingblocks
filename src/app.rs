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
use crate::config::Config;
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
    pub config: Config,
}

impl App {
    pub fn new() -> Self {
        // Load config first
        let config = Config::load();

        let mut world = World::new();
        world.insert_resource(Time::new());

        // Create AudioState initialized with config values
        let mut audio_state = AudioState::new();
        audio_state.set_music_enabled(config.music_enabled);
        audio_state.set_sound_enabled(config.sound_enabled);
        audio_state.set_volume(config.volume);
        world.insert_resource(audio_state);

        // Create GameState with config values
        let mut game_state = GameState::default();
        game_state.show_grid = config.show_grid;
        world.insert_resource(game_state);

        world.insert_resource(Input::default());
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
            config,
        };

        // Spawn initial tetromino
        spawn_tetromino(&mut app.world);

        app
    }

    // Update config from current settings and save it to disk
    pub fn save_config(&mut self) {
        // Update config from current game state
        if let Some(audio_state) = self.world.get_resource::<AudioState>() {
            self.config.music_enabled = audio_state.is_music_enabled();
            self.config.sound_enabled = audio_state.is_sound_enabled();
            self.config.volume = audio_state.get_volume();
        }

        if let Some(game_state) = self.world.get_resource::<GameState>() {
            self.config.show_grid = game_state.show_grid;
        }

        // Save config to disk
        let _ = self.config.save(); // Ignore errors for now
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

        let audio_music_enabled = self
            .world
            .get_resource::<AudioState>()
            .map(|audio| audio.is_music_enabled());

        let audio_sound_enabled = self
            .world
            .get_resource::<AudioState>()
            .map(|audio| audio.is_sound_enabled());

        let show_grid = self
            .world
            .get_resource::<GameState>()
            .map(|game| game.show_grid);

        // Save current menu state
        let current_menu_state = self.menu.state.clone();

        // Reset game state
        let mut game_state = GameState::default();
        // Restore grid setting
        if let Some(grid) = show_grid {
            game_state.show_grid = grid;
        }
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
        if let Some(music_enabled) = audio_music_enabled {
            audio_state.set_music_enabled(music_enabled);
        }
        if let Some(sound_enabled) = audio_sound_enabled {
            audio_state.set_sound_enabled(sound_enabled);
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
