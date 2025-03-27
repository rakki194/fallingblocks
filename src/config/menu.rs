use serde::{Deserialize, Serialize};

// Configuration for menu visual elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuConfig {
    pub title: TitleConfig,
    pub renderer: RendererConfig,
}

impl Default for MenuConfig {
    fn default() -> Self {
        Self {
            title: TitleConfig::default(),
            renderer: RendererConfig::default(),
        }
    }
}

// Title-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitleConfig {
    pub kerning_adjustments: Vec<KerningPair>,
    pub title_height: usize,
    pub protection_margin: usize,
}

impl Default for TitleConfig {
    fn default() -> Self {
        Self {
            // Default kerning adjustments for "FALLINGBLOCKS"
            kerning_adjustments: vec![
                KerningPair {
                    letter_index: 0,
                    adjustment: -8,
                }, // F->A: reduce by 8 spaces
                KerningPair {
                    letter_index: 1,
                    adjustment: -4,
                }, // A->L: reduce by 4 spaces
                KerningPair {
                    letter_index: 2,
                    adjustment: -2,
                }, // L->L: reduce by 2 spaces
                KerningPair {
                    letter_index: 3,
                    adjustment: -3,
                }, // L->I: reduce by 3 spaces
                KerningPair {
                    letter_index: 4,
                    adjustment: -6,
                }, // I->N: reduce by 6 spaces
                KerningPair {
                    letter_index: 5,
                    adjustment: -1,
                }, // N->G: reduce by 1 space
                KerningPair {
                    letter_index: 6,
                    adjustment: 0,
                }, // G->B: no adjustment
                KerningPair {
                    letter_index: 7,
                    adjustment: -8,
                }, // B->L: reduce by 8 spaces
                KerningPair {
                    letter_index: 8,
                    adjustment: -1,
                }, // L->O: reduce by 1 space
                KerningPair {
                    letter_index: 9,
                    adjustment: -1,
                }, // O->C: reduce by 1 space
                KerningPair {
                    letter_index: 10,
                    adjustment: -1,
                }, // C->K: reduce by 1 space
                KerningPair {
                    letter_index: 11,
                    adjustment: -1,
                }, // K->S: reduce by 1 space
            ],
            title_height: 5,      // Height of ASCII art letters
            protection_margin: 3, // Extra margin around title for protection zone
        }
    }
}

// Menu renderer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendererConfig {
    // Tetromino background settings
    pub initial_tetromino_count: usize,
    pub tetromino_max_count: usize,
    pub tetromino_min_lifetime: f32,
    pub tetromino_max_lifetime: f32,
    pub tetromino_edge_margin: i32,
    pub tetromino_max_height: i32,
    pub tetromino_min_fall_speed: f32,
    pub tetromino_max_fall_speed: f32,
    pub tetromino_spawn_interval_ms: u64,
    pub tetromino_fall_limit: i32,

    // Particle settings
    pub particle_max_count: usize,
    pub particle_min_lifetime: f32,
    pub particle_max_lifetime: f32,
    pub particle_min_size: f32,
    pub particle_max_size: f32,
    pub particle_min_fall_speed: f32,
    pub particle_max_fall_speed: f32,
    pub particle_spawn_interval_ms: u64,
    pub particle_vertical_decay: f32,
    pub particle_lifetime_decay: f32,

    // Color cycling settings
    pub title_color_cycle_interval_ms: u64,
    pub title_colors: Vec<TitleColor>,

    // Layout settings
    pub menu_title_height: u16,
    pub menu_option_width: u16,
}

// Supported colors for serialization/deserialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TitleColor {
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Black,
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    Gray,
    Custom(u8, u8, u8),
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            // Tetromino background defaults
            initial_tetromino_count: 60,
            tetromino_max_count: 40,
            tetromino_min_lifetime: 20.0,
            tetromino_max_lifetime: 30.0,
            tetromino_edge_margin: 5,
            tetromino_max_height: 100,
            tetromino_min_fall_speed: 0.15,
            tetromino_max_fall_speed: 0.3,
            tetromino_spawn_interval_ms: 500,
            tetromino_fall_limit: 150,

            // Particle defaults
            particle_max_count: 100,
            particle_min_lifetime: 0.5,
            particle_max_lifetime: 1.7,
            particle_min_size: 0.2,
            particle_max_size: 1.0,
            particle_min_fall_speed: 1.0,
            particle_max_fall_speed: 4.0,
            particle_spawn_interval_ms: 50,
            particle_vertical_decay: 0.5,
            particle_lifetime_decay: 0.2,

            // Color cycling defaults
            title_color_cycle_interval_ms: 100,
            title_colors: vec![
                TitleColor::Red,
                TitleColor::Yellow,
                TitleColor::Green,
                TitleColor::Blue,
                TitleColor::Magenta,
                TitleColor::Cyan,
            ],

            // Layout defaults
            menu_title_height: 10,
            menu_option_width: 20,
        }
    }
}

// Kerning adjustment for a specific letter pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KerningPair {
    pub letter_index: usize, // Index of the first letter in the pair
    pub adjustment: i16,     // Adjustment value (negative = closer)
}

// Get total kerning adjustment (positive value)
pub fn get_total_kerning_adjustment(kerning_pairs: &[KerningPair]) -> usize {
    let sum = kerning_pairs
        .iter()
        .map(|pair| pair.adjustment)
        .sum::<i16>();

    // Convert to positive value for width calculation
    if sum < 0 {
        sum.abs() as usize
    } else {
        0 // If total adjustment is positive (shouldn't happen normally)
    }
}
