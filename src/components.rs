use bevy_ecs::prelude::*;
use crossterm::event::KeyEvent;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TetrominoType {
    I,
    J,
    L,
    O,
    S,
    T,
    Z,
}

impl TetrominoType {
    pub fn random() -> Self {
        match fastrand::u8(0..7) {
            0 => TetrominoType::I,
            1 => TetrominoType::J,
            2 => TetrominoType::L,
            3 => TetrominoType::O,
            4 => TetrominoType::S,
            5 => TetrominoType::T,
            _ => TetrominoType::Z,
        }
    }

    pub fn get_blocks(&self) -> Vec<(i32, i32)> {
        match self {
            TetrominoType::I => vec![(0, 0), (0, 1), (0, 2), (0, 3)],
            TetrominoType::J => vec![(0, 0), (0, 1), (0, 2), (-1, 2)],
            TetrominoType::L => vec![(0, 0), (0, 1), (0, 2), (1, 2)],
            TetrominoType::O => vec![(0, 0), (0, 1), (1, 0), (1, 1)],
            TetrominoType::S => vec![(0, 0), (0, 1), (1, 1), (1, 2)],
            TetrominoType::T => vec![(0, 0), (0, 1), (0, 2), (1, 1)],
            TetrominoType::Z => vec![(0, 0), (0, 1), (-1, 1), (-1, 2)],
        }
    }

    pub fn get_color(&self) -> ratatui::style::Color {
        match self {
            TetrominoType::I => ratatui::style::Color::Cyan,
            TetrominoType::J => ratatui::style::Color::Blue,
            TetrominoType::L => ratatui::style::Color::LightYellow,
            TetrominoType::O => ratatui::style::Color::Yellow,
            TetrominoType::S => ratatui::style::Color::Green,
            TetrominoType::T => ratatui::style::Color::Magenta,
            TetrominoType::Z => ratatui::style::Color::Red,
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Tetromino {
    pub tetromino_type: TetrominoType,
    pub rotation: usize,
}

impl Tetromino {
    pub fn new(tetromino_type: TetrominoType) -> Self {
        Self {
            tetromino_type,
            rotation: 0,
        }
    }

    pub fn get_blocks(&self) -> Vec<(i32, i32)> {
        let blocks = self.tetromino_type.get_blocks();
        match self.rotation {
            0 => blocks,
            1 => blocks.iter().map(|(x, y)| (-y, *x)).collect(),
            2 => blocks.iter().map(|(x, y)| (-x, -y)).collect(),
            3 => blocks.iter().map(|(x, y)| (*y, -x)).collect(),
            _ => unreachable!(),
        }
    }

    pub fn rotate(&mut self) {
        self.rotation = (self.rotation + 1) % 4;
    }
}

#[derive(Resource, Debug, Clone)]
pub struct Board {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Vec<Option<TetrominoType>>>,
}

impl Board {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![vec![None; height]; width],
        }
    }

    pub fn clear(&mut self) {
        for x in 0..self.width {
            for y in 0..self.height {
                self.cells[x][y] = None;
            }
        }
    }

    pub fn is_valid_position(&self, position: &Position, tetromino: &Tetromino) -> bool {
        let blocks = tetromino.get_blocks();

        for (block_x, block_y) in blocks {
            let x = position.x + block_x;
            let y = position.y + block_y;

            // Check if block is out of bounds
            if x < 0 || x >= self.width as i32 || y < 0 || y >= self.height as i32 {
                return false;
            }

            // Check if space is already occupied
            if self.cells[x as usize][y as usize].is_some() {
                return false;
            }
        }

        true
    }

    pub fn lock_tetromino(&mut self, position: &Position, tetromino: &Tetromino) {
        let blocks = tetromino.get_blocks();

        for (block_x, block_y) in blocks {
            let x = position.x + block_x;
            let y = position.y + block_y;

            if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
                self.cells[x as usize][y as usize] = Some(tetromino.tetromino_type);
            }
        }
    }

    pub fn clear_lines(&mut self) -> usize {
        let mut lines_cleared = 0;

        for y in 0..self.height {
            // Check if line is full
            let mut is_line_full = true;
            for x in 0..self.width {
                if self.cells[x][y].is_none() {
                    is_line_full = false;
                    break;
                }
            }

            if is_line_full {
                // Move all lines above down one
                for y2 in (1..=y).rev() {
                    for x in 0..self.width {
                        self.cells[x][y2] = self.cells[x][y2 - 1];
                    }
                }

                // Clear top line
                for x in 0..self.width {
                    self.cells[x][0] = None;
                }

                lines_cleared += 1;
            }
        }

        lines_cleared
    }
}

#[derive(Resource, Debug)]
pub struct GameState {
    pub last_key: Option<KeyEvent>,
    pub last_move: Instant,
    pub game_over: bool,
    pub score: u32,
    pub level: u32,
    pub lines_cleared: u32,
    pub drop_timer: f32,
    pub was_paused_for_resize: bool, // Track if the game was paused for resize
    pub coyote_time_active: bool, // Track if coyote time is active, for last chance to move/rotate
    pub coyote_time_timer: f32,   // Track coyote time duration
    pub combo_count: u32,         // Track consecutive line clears
    pub back_to_back: bool,       // Track back-to-back tetris/t-spin
    pub last_clear_was_difficult: bool, // Track if last clear was a tetris or t-spin
    pub soft_drop_distance: u32,  // Track soft drop distance for current piece
    pub hard_drop_distance: u32,  // Track hard drop distance for current piece
    pub t_spin_count: u32,        // Track total t-spins performed
    pub tetris_count: u32,        // Track total tetris clears
    pub perfect_clear_count: u32, // Track total perfect clears
    pub next_tetromino: Option<TetrominoType>, // Store the next tetromino to be spawned
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            last_key: None,
            last_move: Instant::now(),
            game_over: false,
            score: 0,
            level: crate::game::STARTING_LEVEL,
            lines_cleared: 0,
            drop_timer: 0.0,
            was_paused_for_resize: false,
            coyote_time_active: false,
            coyote_time_timer: 0.0,
            // Initialize new metrics
            combo_count: 0,
            back_to_back: false,
            last_clear_was_difficult: false,
            soft_drop_distance: 0,
            hard_drop_distance: 0,
            t_spin_count: 0,
            tetris_count: 0,
            perfect_clear_count: 0,
            next_tetromino: None,
        }
    }
}

impl GameState {
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    // Check if board is completely clear (perfect clear)
    #[allow(dead_code)]
    pub fn is_perfect_clear(&self, board: &Board) -> bool {
        for y in 0..board.height {
            for x in 0..board.width {
                if board.cells[x][y].is_some() {
                    return false; // Found a block, so not a perfect clear
                }
            }
        }
        true
    }

    // Check if the last move was a T-spin
    // The simplified algorithm checks if:
    // 1. The piece is a T tetromino
    // 2. The piece was just rotated (not moved)
    // 3. At least 3 of the 4 corners around the T center are blocked
    pub fn is_t_spin(&self, board: &Board, position: &Position, tetromino: &Tetromino) -> bool {
        // Check if this is a T tetromino
        if tetromino.tetromino_type != TetrominoType::T {
            return false;
        }

        // Get the center of the T (assuming the center of T is at offset (1, 1) from piece origin)
        let center_x = position.x + 1;
        let center_y = position.y + 1;

        // Check the 4 corners around the T center
        let corners = [
            (center_x - 1, center_y - 1), // Top-left
            (center_x + 1, center_y - 1), // Top-right
            (center_x - 1, center_y + 1), // Bottom-left
            (center_x + 1, center_y + 1), // Bottom-right
        ];

        // Count how many corners are blocked (either by a block or the boundary)
        let mut blocked_corners = 0;
        for (x, y) in corners.iter() {
            // Check if corner is outside the board
            if *x < 0 || *x >= board.width as i32 || *y < 0 || *y >= board.height as i32 {
                blocked_corners += 1;
                continue;
            }

            // Check if corner has a block
            if board.cells[*x as usize][*y as usize].is_some() {
                blocked_corners += 1;
            }
        }

        // A T-spin requires at least 3 corners to be blocked
        blocked_corners >= 3
    }

    // Calculate score based on advanced mechanics
    pub fn update_score(&mut self, lines_cleared: usize, is_t_spin: bool, is_perfect_clear: bool) {
        if lines_cleared == 0 {
            // Reset combo count if no lines were cleared
            self.combo_count = 0;
            return;
        }

        // Increment combo count for consecutive line clears
        self.combo_count += 1;

        // Determine base points based on clear type
        let mut base_points = if is_t_spin {
            // T-spin line clears
            match lines_cleared {
                1 => {
                    self.t_spin_count += 1;
                    crate::game::TSPIN_SINGLE
                }
                2 => {
                    self.t_spin_count += 1;
                    crate::game::TSPIN_DOUBLE
                }
                3 => {
                    self.t_spin_count += 1;
                    crate::game::TSPIN_TRIPLE
                }
                _ => 0, // T-spins with more than 3 lines cleared are not standard
            }
        } else {
            // Standard line clears
            match lines_cleared {
                1 => crate::game::POINTS_SINGLE,
                2 => crate::game::POINTS_DOUBLE,
                3 => crate::game::POINTS_TRIPLE,
                4 => {
                    self.tetris_count += 1;
                    crate::game::POINTS_TETRIS
                }
                _ => 0,
            }
        };

        // Check if the clear qualifies as a difficult clear
        let is_difficult_clear = lines_cleared == 4 || is_t_spin;

        // Apply back-to-back bonus if applicable
        if is_difficult_clear && self.last_clear_was_difficult && self.back_to_back {
            base_points = (base_points as f32 * crate::game::BACK_TO_BACK_MULTIPLIER) as u32;
        }

        // Update back-to-back status
        self.back_to_back = is_difficult_clear;
        self.last_clear_was_difficult = is_difficult_clear;

        // Apply combo bonus
        let combo_bonus = if self.combo_count > 1 {
            ((self.combo_count - 1) as f32 * crate::game::COMBO_MULTIPLIER * base_points as f32)
                as u32
        } else {
            0
        };

        // Apply perfect clear bonus
        let perfect_clear_bonus = if is_perfect_clear {
            self.perfect_clear_count += 1;
            crate::game::PERFECT_CLEAR_BONUS
        } else {
            0
        };

        // Apply soft drop and hard drop bonuses
        let drop_bonus = (self.soft_drop_distance * crate::game::SOFT_DROP_POINTS)
            + (self.hard_drop_distance * crate::game::HARD_DROP_POINTS);

        // Reset drop distances
        self.soft_drop_distance = 0;
        self.hard_drop_distance = 0;

        // Calculate total points with level multiplier
        let level_multiplier = self.level;
        let total_points =
            (base_points * level_multiplier) + combo_bonus + perfect_clear_bonus + drop_bonus;

        // Update score
        self.score += total_points;
        self.lines_cleared += lines_cleared as u32;

        // Update level based on lines cleared and score
        self.update_level();
    }

    // Update level based on both lines cleared and score thresholds
    pub fn update_level(&mut self) {
        // Traditional level progression based on lines cleared
        let lines_level =
            (self.lines_cleared / crate::game::LINES_PER_LEVEL) + crate::game::STARTING_LEVEL;

        // Level progression based on score thresholds
        let mut score_level = crate::game::STARTING_LEVEL;
        for &(threshold, level) in crate::game::LEVEL_SCORE_THRESHOLDS.iter() {
            if self.score >= threshold {
                score_level = level;
            } else {
                break;
            }
        }

        // Take the maximum of the two approaches, but cap at MAX_LEVEL
        self.level = std::cmp::min(
            std::cmp::max(lines_level, score_level),
            crate::game::MAX_LEVEL,
        );
    }

    pub fn get_drop_delay(&self) -> f32 {
        // Speed increases with level (faster drops as level increases)
        // More aggressive curve with higher levels
        if self.level < 10 {
            // Levels 1-9: linear decrease
            0.8 - (self.level as f32 - 1.0) * 0.07
        } else if self.level < 20 {
            // Levels 10-19: steeper decrease
            0.2 - (self.level as f32 - 10.0) * 0.01
        } else {
            // Levels 20+: minimum delay
            0.1
        }
    }

    pub fn update_hard_drop_score(&mut self, drop_distance: u32) {
        // Calculate points based on the hard drop distance and add to score
        self.hard_drop_distance += drop_distance;
        // Hard drop points are calculated in the get_hard_drop_score method
        // which is called when updating the total score
    }
}

// Particle system for visual effects
#[derive(Debug, Clone, Component)]
pub struct Particle {
    pub position: Position,
    pub velocity: (f32, f32),
    pub color: ratatui::style::Color,
    pub lifetime: f32,
    pub size: f32,
}

// Screen shake effect
#[derive(Debug, Clone, Resource, Default)]
pub struct ScreenShake {
    pub intensity: f32,
    pub duration: f32,
    pub current_offset: (i16, i16),
    pub is_active: bool,
}

// Input state for keyboard controls
#[derive(Debug, Clone, Resource, Default)]
pub struct Input {
    pub left: bool,
    pub right: bool,
    pub down: bool,
    pub rotate: bool,
    pub hard_drop: bool,
    pub hard_drop_released: bool, // Track if the hard drop key has been released
}

// Coyote time mechanic (last chance to move after landing)
#[derive(Debug, Clone, Resource, Default)]
pub struct CoyoteTime {
    pub active: bool,
    pub timer: f32,
}

// Ghost piece that shows where the tetromino will land
#[derive(Debug, Clone, Component)]
pub struct Ghost {
    pub position: Position,
}
