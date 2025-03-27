#![warn(clippy::all, clippy::pedantic)]

#[cfg(test)]
mod tests {
    use crate::config::menu::{KerningPair, TitleColor};
    use crate::config::{CONFIG, Config};
    use ratatui::style::Color;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    // Helper function to create a temp directory for config tests
    fn setup_temp_config() -> (tempfile::TempDir, PathBuf) {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let config_path = temp_dir.path().join("config.toml");

        // Set environment variable to use this path
        unsafe {
            std::env::set_var("FALLING_BLOCKS_CONFIG", config_path.to_str().unwrap());
        }

        (temp_dir, config_path)
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();

        // Test menu config defaults
        assert_eq!(config.menu.title.title_height, 5);
        assert_eq!(config.menu.title.protection_margin, 3);

        // Test renderer config defaults
        assert_eq!(config.menu.renderer.initial_tetromino_count, 60);
        assert_eq!(config.menu.renderer.tetromino_max_count, 40);
        assert_eq!(config.menu.renderer.particle_max_count, 100);
        assert_eq!(config.menu.renderer.title_color_cycle_interval_ms, 100);

        // Test kerning adjustments
        assert!(!config.menu.title.kerning_adjustments.is_empty());

        // Check F->A kerning adjustment
        let fa_kerning = config
            .menu
            .title
            .kerning_adjustments
            .iter()
            .find(|p| p.letter_index == 0)
            .expect("F->A kerning pair not found");
        assert_eq!(fa_kerning.adjustment, -8);

        // Check B->L kerning adjustment
        let bl_kerning = config
            .menu
            .title
            .kerning_adjustments
            .iter()
            .find(|p| p.letter_index == 7)
            .expect("B->L kerning pair not found");
        assert_eq!(bl_kerning.adjustment, -8);
    }

    #[test]
    fn test_config_serialization() {
        let (_temp_dir, config_path) = setup_temp_config();

        // Create a custom config
        let mut config = Config::default();
        config.menu.title.title_height = 6;
        config.menu.title.protection_margin = 4;
        config.menu.renderer.initial_tetromino_count = 70;
        config.menu.renderer.particle_max_count = 150;

        // Modify some kerning values
        if let Some(kerning) = config
            .menu
            .title
            .kerning_adjustments
            .iter_mut()
            .find(|p| p.letter_index == 0)
        {
            kerning.adjustment = -10;
        }

        // Add a custom color
        config
            .menu
            .renderer
            .title_colors
            .push(TitleColor::Custom(255, 0, 128));

        // Serialize to TOML
        let config_toml = toml::to_string_pretty(&config).expect("Failed to serialize config");
        fs::write(&config_path, config_toml).expect("Failed to write config file");

        // Deserialize back
        let content = fs::read_to_string(&config_path).expect("Failed to read config file");
        let loaded_config: Config = toml::from_str(&content).expect("Failed to parse config");

        // Verify values
        assert_eq!(loaded_config.menu.title.title_height, 6);
        assert_eq!(loaded_config.menu.title.protection_margin, 4);
        assert_eq!(loaded_config.menu.renderer.initial_tetromino_count, 70);
        assert_eq!(loaded_config.menu.renderer.particle_max_count, 150);

        // Check kerning
        let fa_kerning = loaded_config
            .menu
            .title
            .kerning_adjustments
            .iter()
            .find(|p| p.letter_index == 0)
            .expect("F->A kerning pair not found");
        assert_eq!(fa_kerning.adjustment, -10);

        // Check custom color
        let has_custom_color = loaded_config
            .menu
            .renderer
            .title_colors
            .iter()
            .any(|color| matches!(color, TitleColor::Custom(255, 0, 128)));
        assert!(has_custom_color, "Custom color not found in loaded config");
    }

    #[test]
    fn test_title_color_conversion() {
        // Test all color variants
        let colors = vec![
            TitleColor::Red,
            TitleColor::Green,
            TitleColor::Yellow,
            TitleColor::Blue,
            TitleColor::Magenta,
            TitleColor::Cyan,
            TitleColor::White,
            TitleColor::Black,
            TitleColor::DarkGray,
            TitleColor::LightRed,
            TitleColor::LightGreen,
            TitleColor::LightYellow,
            TitleColor::LightBlue,
            TitleColor::LightMagenta,
            TitleColor::LightCyan,
            TitleColor::Gray,
            TitleColor::Custom(255, 0, 128),
        ];

        // Convert to ratatui colors
        let ratatui_colors: Vec<Color> = colors
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

        // Verify the correct number of colors
        assert_eq!(ratatui_colors.len(), colors.len());

        // Check custom color
        assert_eq!(ratatui_colors[16], Color::Rgb(255, 0, 128));
    }

    #[test]
    fn test_kerning_adjustment_calculation() {
        let kerning_pairs = vec![
            KerningPair {
                letter_index: 0,
                adjustment: -5,
            },
            KerningPair {
                letter_index: 1,
                adjustment: -3,
            },
            KerningPair {
                letter_index: 2,
                adjustment: -2,
            },
        ];

        let total_adjustment = crate::config::menu::get_total_kerning_adjustment(&kerning_pairs);

        // Sum of all negative adjustments converted to positive
        assert_eq!(total_adjustment, 10);

        // Test with positive values (should be 0)
        let positive_pairs = vec![
            KerningPair {
                letter_index: 0,
                adjustment: 5,
            },
            KerningPair {
                letter_index: 1,
                adjustment: 3,
            },
        ];

        let adjustment = crate::config::menu::get_total_kerning_adjustment(&positive_pairs);
        assert_eq!(adjustment, 0);
    }

    #[test]
    fn test_config_hot_reload() {
        let (_temp_dir, config_path) = setup_temp_config();

        // Create initial config
        let mut config = Config::default();
        config.menu.title.title_height = 5;

        // Save config
        let config_toml = toml::to_string_pretty(&config).expect("Failed to serialize config");
        fs::write(&config_path, config_toml).expect("Failed to write config file");

        // Force reload the config
        assert!(Config::force_reload());

        // Verify initial value
        {
            let config = CONFIG.read().unwrap();
            assert_eq!(config.menu.title.title_height, 5);
        }

        // Modify config file
        let mut config = Config::default();
        config.menu.title.title_height = 7;
        let config_toml = toml::to_string_pretty(&config).expect("Failed to serialize config");
        fs::write(&config_path, config_toml).expect("Failed to write config file");

        // Force reload
        assert!(Config::force_reload());

        // Verify updated value
        {
            let config = CONFIG.read().unwrap();
            assert_eq!(config.menu.title.title_height, 7);
        }
    }
}
