#![warn(clippy::all, clippy::pedantic)]

use crate::app::App;
use crate::components::{Particle, Position, Tetromino, TetrominoType};
use crate::config::{CONFIG, menu::TitleColor};
use crate::menu_types::{Menu, MenuOption, MenuState, OptionsOption};
use crate::sound::AudioState;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::Block,
};
use std::time::Duration;
use std::time::Instant;

use super::main_menu::render_main_menu_options;
use super::options_menu::render_options_menu;
use super::title::{get_title_protection_zone, render_ascii_title};

pub struct MenuRenderer {
    pub particles: Vec<Particle>,
    pub tetrominos: Vec<(Position, Tetromino, (f32, f32), f32)>, // position, tetromino, velocity, lifetime
    pub last_particle_spawn: Instant,
    pub last_tetromino_spawn: Instant,
    pub title_colors: Vec<Color>,
    pub color_change_time: Instant,
}

impl Default for MenuRenderer {
    fn default() -> Self {
        // Create some initial tetrominos right away
        let particles = Vec::new();
        let mut tetrominos = Vec::new();

        // Get configuration
        let config = CONFIG.read().unwrap();
        let renderer_config = &config.menu.renderer;

        // Pre-populate with tetrominos starting above the screen
        for _ in 0..renderer_config.initial_tetromino_count {
            let tetromino_type = match fastrand::u8(0..7) {
                0 => TetrominoType::I,
                1 => TetrominoType::J,
                2 => TetrominoType::L,
                3 => TetrominoType::O,
                4 => TetrominoType::S,
                5 => TetrominoType::T,
                _ => TetrominoType::Z,
            };

            let mut tetromino = Tetromino::new(tetromino_type);
            tetromino.rotation = fastrand::usize(0..4);

            // Spread tetrominos across width, all starting above screen at different heights
            tetrominos.push((
                Position {
                    x: fastrand::i32(
                        renderer_config.tetromino_edge_margin
                            ..(100 - renderer_config.tetromino_edge_margin),
                    ),
                    y: -fastrand::i32(1..renderer_config.tetromino_max_height),
                },
                tetromino,
                (
                    0.0, // No horizontal drift
                    fastrand::f32()
                        * (renderer_config.tetromino_max_fall_speed
                            - renderer_config.tetromino_min_fall_speed)
                        + renderer_config.tetromino_min_fall_speed,
                ),
                fastrand::f32()
                    * (renderer_config.tetromino_max_lifetime
                        - renderer_config.tetromino_min_lifetime)
                    + renderer_config.tetromino_min_lifetime,
            ));
        }

        // Convert configured colors to ratatui colors
        let title_colors = renderer_config
            .title_colors
            .iter()
            .map(|color| match color {
                TitleColor::Red => Color::Red,
                TitleColor::Green => Color::Green,
                TitleColor::Yellow => Color::Yellow,
                TitleColor::Blue => Color::Blue,
                TitleColor::Magenta => Color::Magenta,
                TitleColor::Cyan => Color::Cyan,
                TitleColor::White => Color::White,
                TitleColor::Black => Color::Black,
                TitleColor::DarkGray => Color::DarkGray,
                TitleColor::LightRed => Color::LightRed,
                TitleColor::LightGreen => Color::LightGreen,
                TitleColor::LightYellow => Color::LightYellow,
                TitleColor::LightBlue => Color::LightBlue,
                TitleColor::LightMagenta => Color::LightMagenta,
                TitleColor::LightCyan => Color::LightCyan,
                TitleColor::Gray => Color::Gray,
                TitleColor::Custom(r, g, b) => Color::Rgb(*r, *g, *b),
            })
            .collect();

        Self {
            particles,
            tetrominos,
            last_particle_spawn: Instant::now(),
            last_tetromino_spawn: Instant::now(),
            title_colors,
            color_change_time: Instant::now(),
        }
    }
}

impl MenuRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next_option(&mut self, menu: &mut Menu) {
        match menu.state {
            MenuState::MainMenu => {
                menu.selected_option = match menu.selected_option {
                    MenuOption::NewGame => MenuOption::Options,
                    MenuOption::Options => MenuOption::Quit,
                    MenuOption::Quit => MenuOption::NewGame,
                };
            }
            MenuState::Options => {
                menu.options_selected = match menu.options_selected {
                    OptionsOption::MusicToggle => OptionsOption::SoundToggle,
                    OptionsOption::SoundToggle => OptionsOption::VolumeUp,
                    OptionsOption::VolumeUp => OptionsOption::VolumeDown,
                    OptionsOption::VolumeDown => OptionsOption::Back,
                    OptionsOption::Back => OptionsOption::MusicToggle,
                };
            }
            MenuState::Game => {}
        }
    }

    pub fn prev_option(&mut self, menu: &mut Menu) {
        match menu.state {
            MenuState::MainMenu => {
                menu.selected_option = match menu.selected_option {
                    MenuOption::NewGame => MenuOption::Quit,
                    MenuOption::Options => MenuOption::NewGame,
                    MenuOption::Quit => MenuOption::Options,
                };
            }
            MenuState::Options => {
                menu.options_selected = match menu.options_selected {
                    OptionsOption::MusicToggle => OptionsOption::Back,
                    OptionsOption::SoundToggle => OptionsOption::MusicToggle,
                    OptionsOption::VolumeUp => OptionsOption::SoundToggle,
                    OptionsOption::VolumeDown => OptionsOption::VolumeUp,
                    OptionsOption::Back => OptionsOption::VolumeDown,
                };
            }
            MenuState::Game => {}
        }
    }

    pub fn select(&mut self, menu: &mut Menu, app: &mut App) -> bool {
        match menu.state {
            MenuState::MainMenu => match menu.selected_option {
                MenuOption::NewGame => {
                    menu.state = MenuState::Game;
                    app.reset();
                    true
                }
                MenuOption::Options => {
                    menu.state = MenuState::Options;
                    true
                }
                MenuOption::Quit => true,
            },
            MenuState::Options => match menu.options_selected {
                OptionsOption::MusicToggle => {
                    if let Some(mut audio_state) = app.world.get_resource_mut::<AudioState>() {
                        audio_state.toggle_music();
                    }
                    true
                }
                OptionsOption::SoundToggle => {
                    if let Some(mut audio_state) = app.world.get_resource_mut::<AudioState>() {
                        // Toggle sound enabled state
                        audio_state.toggle_sound();
                    }
                    true
                }
                OptionsOption::VolumeUp => {
                    if let Some(mut audio_state) = app.world.get_resource_mut::<AudioState>() {
                        let volume = audio_state.get_volume();
                        audio_state.set_volume((volume + 0.1).min(1.0));
                    }
                    true
                }
                OptionsOption::VolumeDown => {
                    if let Some(mut audio_state) = app.world.get_resource_mut::<AudioState>() {
                        let volume = audio_state.get_volume();
                        audio_state.set_volume((volume - 0.1).max(0.0));
                    }
                    true
                }
                OptionsOption::Back => {
                    menu.state = MenuState::MainMenu;
                    true
                }
            },
            MenuState::Game => false,
        }
    }

    pub fn update(&mut self) {
        // Get configuration
        let config = CONFIG.read().unwrap();
        let renderer_config = &config.menu.renderer;

        // Update title colors
        if self.color_change_time.elapsed()
            > Duration::from_millis(renderer_config.title_color_cycle_interval_ms)
        {
            self.color_change_time = Instant::now();
            let first_color = self.title_colors.remove(0);
            self.title_colors.push(first_color);
        }

        // Spawn regular particles
        if self.last_particle_spawn.elapsed()
            > Duration::from_millis(renderer_config.particle_spawn_interval_ms)
        {
            self.last_particle_spawn = Instant::now();
            if self.particles.len() < renderer_config.particle_max_count {
                self.particles.push(Particle {
                    position: Position {
                        x: fastrand::i32(0..100),
                        y: 0, // Start at the top
                    },
                    velocity: (
                        0.0,
                        fastrand::f32()
                            * (renderer_config.particle_max_fall_speed
                                - renderer_config.particle_min_fall_speed)
                            + renderer_config.particle_min_fall_speed,
                    ),
                    color: self.title_colors[fastrand::usize(0..self.title_colors.len())],
                    lifetime: fastrand::f32()
                        * (renderer_config.particle_max_lifetime
                            - renderer_config.particle_min_lifetime)
                        + renderer_config.particle_min_lifetime,
                    size: fastrand::f32()
                        * (renderer_config.particle_max_size - renderer_config.particle_min_size)
                        + renderer_config.particle_min_size,
                });
            }
        }

        // Spawn tetromino particles at a steady rate
        if self.last_tetromino_spawn.elapsed()
            > Duration::from_millis(renderer_config.tetromino_spawn_interval_ms)
        {
            self.last_tetromino_spawn = Instant::now();
            // Keep a reasonable number of tetrominos on screen
            if self.tetrominos.len() < renderer_config.tetromino_max_count {
                // Create a random tetromino type
                let tetromino_type = match fastrand::u8(0..7) {
                    0 => TetrominoType::I,
                    1 => TetrominoType::J,
                    2 => TetrominoType::L,
                    3 => TetrominoType::O,
                    4 => TetrominoType::S,
                    5 => TetrominoType::T,
                    _ => TetrominoType::Z,
                };

                // Create a new tetromino with random rotation
                let mut tetromino = Tetromino::new(tetromino_type);
                tetromino.rotation = fastrand::usize(0..4);

                // Spawn across the width of the screen
                let spawn_x = fastrand::i32(
                    renderer_config.tetromino_edge_margin
                        ..(100 - renderer_config.tetromino_edge_margin),
                );

                self.tetrominos.push((
                    Position {
                        x: spawn_x,
                        y: -5, // Start just above the screen
                    },
                    tetromino,
                    (
                        0.0, // No horizontal movement
                        fastrand::f32()
                            * (renderer_config.tetromino_max_fall_speed
                                - renderer_config.tetromino_min_fall_speed)
                            + renderer_config.tetromino_min_fall_speed,
                    ),
                    fastrand::f32()
                        * (renderer_config.tetromino_max_lifetime
                            - renderer_config.tetromino_min_lifetime)
                        + renderer_config.tetromino_min_lifetime,
                ));
            }
        }

        // Update tetrominos
        for (position, _, velocity, lifetime) in &mut self.tetrominos {
            // Update position based on velocity
            position.x += velocity.0 as i32;
            position.y += velocity.1 as i32;

            // Reduce lifetime
            *lifetime -= 0.1;
        }

        // Cleanup tetrominos that are off-screen or have expired
        self.tetrominos.retain(|(pos, _, _, lifetime)| {
            pos.y < renderer_config.tetromino_fall_limit && // Maximum fall distance
            *lifetime > 0.0 // Or when they've expired
        });

        // Update and clean regular particles
        self.particles.retain_mut(|p| {
            // Update position - only move vertically
            p.position.y += (p.velocity.1 * renderer_config.particle_vertical_decay) as i32;
            p.lifetime -= renderer_config.particle_lifetime_decay;

            // Keep particle if still alive and on screen
            p.lifetime > 0.0 && p.position.y < 100
        });
    }

    pub fn render_menu(f: &mut Frame, app: &App, menu: &Menu, renderer: &MenuRenderer) {
        // Get configuration
        let config = CONFIG.read().unwrap();
        let renderer_config = &config.menu.renderer;

        let area = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(renderer_config.menu_title_height),
                Constraint::Min(0),
            ])
            .split(area);

        // First render the tetromino particles so they appear behind everything
        render_tetromino_particles(f, renderer, area);

        // Then render regular particles
        render_menu_particles(f, renderer);

        // Then render title and menu on top
        render_ascii_title(f, chunks[0], &renderer.title_colors);

        match menu.state {
            MenuState::MainMenu => render_main_menu_options(f, chunks[1], menu),
            MenuState::Options => render_options_menu(f, chunks[1], menu, app),
            MenuState::Game => {}
        }
    }
}

/// Renders the falling tetromino particles
fn render_tetromino_particles(f: &mut Frame, renderer: &MenuRenderer, area: Rect) {
    // Get configuration
    let config = CONFIG.read().unwrap();
    let renderer_config = &config.menu.renderer;

    // Get title protection zone
    let title_protection = get_title_protection_zone(area);

    // Define menu options area
    let menu_area = Rect::new(
        area.width / 2 - renderer_config.menu_option_width / 2,
        title_protection.height,
        renderer_config.menu_option_width,
        10,
    );

    // Now render tetrominos with square blocks
    for (position, tetromino, _, _) in &renderer.tetrominos {
        let blocks = tetromino.get_blocks();
        let color = tetromino.tetromino_type.get_color();

        // Scale factor - keep blocks square (2x1 in terminal characters)
        let x_scale: i32 = 2; // Width (terminal chars are about 2:1 width:height ratio)
        let y_scale: i32 = 1; // Height

        // Render each block of the tetromino
        for &(block_x, block_y) in &blocks {
            // Calculate the base position for this block
            let base_x = position.x + block_x * x_scale;
            let base_y = position.y + block_y * y_scale;

            // Skip if the y-coordinate is in the title protection zone
            if base_y >= 0 && base_y < title_protection.height as i32 {
                continue;
            }

            // Skip if it overlaps with the menu
            let block_rect =
                Rect::new(base_x as u16, base_y as u16, x_scale as u16, y_scale as u16);

            if overlaps(block_rect, menu_area) {
                continue;
            }

            // Make sure it's within the screen bounds
            if base_x >= 0
                && base_x < (area.width as i32 - x_scale)
                && base_y >= 0
                && base_y < (area.height as i32 - y_scale)
            {
                // Render as a properly scaled block (2×1 for aspect ratio)
                let block_rect =
                    Rect::new(base_x as u16, base_y as u16, x_scale as u16, y_scale as u16);

                let block = Block::default().style(Style::default().fg(color).bg(color));
                f.render_widget(block, block_rect);
            }
        }
    }
}

/// Renders the small particle effects
fn render_menu_particles(f: &mut Frame, renderer: &MenuRenderer) {
    for particle in &renderer.particles {
        let x = particle.position.x as u16;
        let y = particle.position.y as u16;
        let area = Rect::new(x, y, 1, 1);
        let block = Block::default().style(Style::default().fg(particle.color));
        f.render_widget(block, area);
    }
}

/// Helper function to check if two rectangles overlap
pub fn overlaps(r1: Rect, r2: Rect) -> bool {
    r1.x < r2.x + r2.width
        && r1.x + r1.width > r2.x
        && r1.y < r2.y + r2.height
        && r1.y + r1.height > r2.y
}

/// Helper function to create a centered rectangle inside another rectangle
pub fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;

    Rect::new(x, y, width.min(r.width), height.min(r.height))
}
