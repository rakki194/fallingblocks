#![warn(clippy::all, clippy::pedantic)]

// Game board dimensions
pub const BOARD_WIDTH: usize = 10;
pub const BOARD_HEIGHT: usize = 20;

// Game timing
pub const COYOTE_TIME_DURATION: f32 = 0.05; // Time in seconds for coyote time (last chance to move/rotate)

// Basic line clear scoring (level 1 values, will be multiplied by level)
pub const POINTS_SINGLE: u32 = 40;
pub const POINTS_DOUBLE: u32 = 100;
pub const POINTS_TRIPLE: u32 = 300;
pub const POINTS_TETRIS: u32 = 1200;

// Advanced scoring mechanics
pub const COMBO_MULTIPLIER: f32 = 0.5; // Multiplier for each consecutive line clear (e.g., 50% bonus per combo)
pub const PERFECT_CLEAR_BONUS: u32 = 3000; // Bonus for clearing the entire board
pub const BACK_TO_BACK_MULTIPLIER: f32 = 1.5; // Multiplier for consecutive difficult clears (tetris or t-spin)
pub const SOFT_DROP_POINTS: u32 = 1; // Points per cell soft dropped
pub const HARD_DROP_POINTS: u32 = 2; // Points per cell hard dropped

// T-spin bonuses
pub const TSPIN_SINGLE: u32 = 800; // T-spin with single line clear
pub const TSPIN_DOUBLE: u32 = 1200; // T-spin with double line clear
pub const TSPIN_TRIPLE: u32 = 1600; // T-spin with triple line clear

// Level progression
pub const LINES_PER_LEVEL: u32 = 10;
pub const MAX_LEVEL: u32 = 30; // Maximum level
pub const STARTING_LEVEL: u32 = 1; // Starting level

// Level thresholds - faster progression at higher scores
pub const LEVEL_SCORE_THRESHOLDS: &[(u32, u32)] = &[
    (5_000, 2),  // Reach 5,000 points to reach level 2
    (15_000, 3), // Reach 15,000 points to reach level 3
    (40_000, 4), // etc.
    (70_000, 5),
    (100_000, 6),
    (150_000, 7),
    (250_000, 8),
    (400_000, 9),
    (500_000, 10),
    (600_000, 11),
    (700_000, 12),
    (800_000, 13),
    (900_000, 14),
    (1_000_000, 15),
    (1_150_000, 16),
    (1_300_000, 17),
    (1_500_000, 18),
    (1_700_000, 19),
    (2_000_000, 20),
    (2_500_000, 21),
    (3_000_000, 22),
    (3_500_000, 23),
    (4_000_000, 24),
    (4_500_000, 25),
    (5_000_000, 26),
    (5_500_000, 27),
    (6_000_000, 28),
    (7_000_000, 29),
    (10_000_000, 30),
];
