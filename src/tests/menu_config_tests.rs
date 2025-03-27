#![warn(clippy::all, clippy::pedantic)]

#[cfg(test)]
mod tests {
    use crate::components::{Particle, Position, Tetromino, TetrominoType};
    use crate::config::CONFIG;
    use crate::config::menu::{MenuConfig, RendererConfig, TitleColor, TitleConfig};
    use crate::menu::renderer::MenuRenderer;
    use ratatui::style::Color;
    use std::time::Instant;

    #[test]
    fn test_menu_config_defaults() {
        let menu_config = MenuConfig::default();

        // Test title config defaults
        assert_eq!(menu_config.title.title_height, 5);
        assert_eq!(menu_config.title.protection_margin, 3);

        // Test renderer config defaults
        let renderer = &menu_config.renderer;
        assert_eq!(renderer.initial_tetromino_count, 60);
        assert_eq!(renderer.tetromino_max_count, 40);
        assert_eq!(renderer.tetromino_edge_margin, 5);
        assert_eq!(renderer.particle_max_count, 100);
        assert_eq!(renderer.title_color_cycle_interval_ms, 100);

        // Test title colors
        assert_eq!(renderer.title_colors.len(), 6); // Default has 6 colors

        // Test kerning adjustments
        assert!(!menu_config.title.kerning_adjustments.is_empty());
    }

    #[test]
    fn test_title_config() {
        let title_config = TitleConfig::default();

        // Test defaults
        assert_eq!(title_config.title_height, 5);
        assert_eq!(title_config.protection_margin, 3);

        // Test kerning adjustments for specific letter pairs
        assert_eq!(title_config.kerning_adjustments.len(), 12); // Should be 12 pairs for "FALLINGBLOCKS"

        // Test specific kerning adjustments
        let fa = title_config
            .kerning_adjustments
            .iter()
            .find(|k| k.letter_index == 0);
        assert!(fa.is_some());
        assert_eq!(fa.unwrap().adjustment, -8); // F->A adjustment

        let bl = title_config
            .kerning_adjustments
            .iter()
            .find(|k| k.letter_index == 7);
        assert!(bl.is_some());
        assert_eq!(bl.unwrap().adjustment, -8); // B->L adjustment
    }

    #[test]
    fn test_renderer_config() {
        let renderer_config = RendererConfig::default();

        // Test tetromino settings
        assert_eq!(renderer_config.initial_tetromino_count, 60);
        assert_eq!(renderer_config.tetromino_max_count, 40);
        assert_eq!(renderer_config.tetromino_min_lifetime, 20.0);
        assert_eq!(renderer_config.tetromino_max_lifetime, 30.0);
        assert_eq!(renderer_config.tetromino_edge_margin, 5);
        assert_eq!(renderer_config.tetromino_max_height, 100);
        assert_eq!(renderer_config.tetromino_min_fall_speed, 0.15);
        assert_eq!(renderer_config.tetromino_max_fall_speed, 0.3);

        // Test particle settings
        assert_eq!(renderer_config.particle_max_count, 100);
        assert_eq!(renderer_config.particle_min_lifetime, 0.5);
        assert_eq!(renderer_config.particle_max_lifetime, 1.7);
        assert_eq!(renderer_config.particle_min_size, 0.2);
        assert_eq!(renderer_config.particle_max_size, 1.0);

        // Test color settings
        assert_eq!(renderer_config.title_color_cycle_interval_ms, 100);
        assert_eq!(renderer_config.title_colors.len(), 6);

        // Test layout settings
        assert_eq!(renderer_config.menu_title_height, 10);
        assert_eq!(renderer_config.menu_option_width, 20);
    }

    #[test]
    fn test_menu_renderer_initialization() {
        // Override config values for the test
        {
            let mut config = CONFIG.write().unwrap();
            config.menu.renderer.initial_tetromino_count = 10; // Set to smaller value for test
            config.menu.renderer.tetromino_max_count = 20;
            config.menu.renderer.particle_max_count = 30;

            // Customize title colors
            config.menu.renderer.title_colors =
                vec![TitleColor::Red, TitleColor::Blue, TitleColor::Green];
        }

        // Create menu renderer, which should use our config
        let renderer = MenuRenderer::default();

        // Check that the tetrominos were initialized according to config
        assert_eq!(renderer.tetrominos.len(), 10); // Should match initial_tetromino_count

        // Check that title_colors are correctly mapped from config
        assert_eq!(renderer.title_colors.len(), 3);
        assert_eq!(renderer.title_colors[0], Color::Red);
        assert_eq!(renderer.title_colors[1], Color::Blue);
        assert_eq!(renderer.title_colors[2], Color::Green);

        // Check particles empty at start
        assert!(renderer.particles.is_empty());
    }

    #[test]
    fn test_title_color_mapping() {
        // Test the mapping between TitleColor enum and ratatui Color
        let title_colors = vec![
            TitleColor::Red,
            TitleColor::Green,
            TitleColor::Blue,
            TitleColor::Custom(255, 0, 128),
        ];

        let expected_colors = vec![
            Color::Red,
            Color::Green,
            Color::Blue,
            Color::Rgb(255, 0, 128),
        ];

        // Map TitleColor to ratatui Color (similar to how MenuRenderer does it)
        let mapped_colors: Vec<Color> = title_colors
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

        // Verify the mapping is correct
        assert_eq!(mapped_colors, expected_colors);
    }

    #[test]
    fn test_custom_menu_renderer() {
        // Create a renderer with custom values for testing
        let renderer = MenuRenderer {
            particles: vec![Particle {
                position: Position { x: 10, y: 20 },
                velocity: (0.0, 1.0),
                color: Color::Red,
                lifetime: 2.0,
                size: 1.0,
            }],
            tetrominos: vec![(
                Position { x: 5, y: 10 },
                Tetromino::new(TetrominoType::I),
                (0.0, 0.2),
                30.0,
            )],
            last_particle_spawn: Instant::now(),
            last_tetromino_spawn: Instant::now(),
            title_colors: vec![Color::Red, Color::Blue],
            color_change_time: Instant::now(),
        };

        // Verify the custom values
        assert_eq!(renderer.particles.len(), 1);
        assert_eq!(renderer.tetrominos.len(), 1);
        assert_eq!(renderer.title_colors.len(), 2);

        // Check the particle
        let particle = &renderer.particles[0];
        assert_eq!(particle.position.x, 10);
        assert_eq!(particle.position.y, 20);
        assert_eq!(particle.velocity.1, 1.0);
        assert_eq!(particle.color, Color::Red);

        // Check the tetromino
        let (pos, tetromino, velocity, lifetime) = &renderer.tetrominos[0];
        assert_eq!(pos.x, 5);
        assert_eq!(pos.y, 10);
        assert_eq!(tetromino.tetromino_type, TetrominoType::I);
        assert_eq!(velocity.1, 0.2);
        assert_eq!(*lifetime, 30.0);
    }
}
