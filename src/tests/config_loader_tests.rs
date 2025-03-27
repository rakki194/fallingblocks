#![warn(clippy::all, clippy::pedantic)]

#[cfg(test)]
mod tests {
    use crate::config::Config;
    use crate::config::loader::{ConfigError, load_config_from_file, save_config_to_file};
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    // Helper function to create a test config path
    fn create_test_config_path() -> (tempfile::TempDir, PathBuf) {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let config_path = temp_dir.path().join("test_config.toml");

        // Set environment variable to use this path
        unsafe {
            std::env::set_var("FALLING_BLOCKS_CONFIG", config_path.to_str().unwrap());
        }

        (temp_dir, config_path)
    }

    #[test]
    fn test_load_nonexistent_config() {
        let (_temp_dir, config_path) = create_test_config_path();

        // Ensure file doesn't exist
        if config_path.exists() {
            fs::remove_file(&config_path).expect("Failed to remove existing test config");
        }

        // Loading a non-existent config should create a default one
        let config = load_config_from_file().expect("Failed to load default config");

        // Verify the file was created
        assert!(config_path.exists(), "Config file should have been created");

        // Check default values are set
        assert_eq!(config.menu.title.title_height, 5);
        assert_eq!(config.menu.renderer.particle_max_count, 100);
    }

    #[test]
    fn test_save_and_load_config() {
        let (_temp_dir, _config_path) = create_test_config_path();

        // Create a custom config
        let mut config = Config::default();
        config.menu.title.title_height = 8;
        config.menu.renderer.particle_max_count = 200;

        // Save config
        save_config_to_file(&config).expect("Failed to save config");

        // Load the config back
        let loaded_config = load_config_from_file().expect("Failed to load config");

        // Verify values
        assert_eq!(loaded_config.menu.title.title_height, 8);
        assert_eq!(loaded_config.menu.renderer.particle_max_count, 200);
    }

    #[test]
    fn test_malformed_config() {
        let (_temp_dir, config_path) = create_test_config_path();

        // Write invalid TOML
        fs::write(&config_path, "invalid toml content ! @ #")
            .expect("Failed to write invalid config");

        // Attempt to load should return an error
        let result = load_config_from_file();

        match result {
            Err(ConfigError::Parse(_)) => {
                // Expected error
            }
            Ok(_) => panic!("Expected error when loading invalid config"),
            Err(e) => panic!("Unexpected error type: {:?}", e),
        }

        // Attempting to load an invalid config should not crash
    }

    #[test]
    fn test_partial_config() {
        let (_temp_dir, config_path) = create_test_config_path();

        // Write a partial config with only some fields
        let partial_config = r#"
            [menu.title]
            title_height = 9
            protection_margin = 5
        "#;

        fs::write(&config_path, partial_config).expect("Failed to write partial config");

        // Load the config - it should successfully fill in missing values with defaults
        let loaded_config = load_config_from_file().expect("Failed to load partial config");

        // Check explicitly set values
        assert_eq!(loaded_config.menu.title.title_height, 9);
        assert_eq!(loaded_config.menu.title.protection_margin, 5);

        // Check default values for missing fields
        assert_eq!(loaded_config.menu.renderer.particle_max_count, 100);
        assert_eq!(loaded_config.menu.renderer.tetromino_max_count, 40);
    }

    #[test]
    fn test_config_env_var_override() {
        // Create a temp directory and config path
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let config_path = temp_dir.path().join("override_config.toml");

        // Set environment variable
        unsafe {
            std::env::set_var("FALLING_BLOCKS_CONFIG", config_path.to_str().unwrap());
        }

        // Create and save a config
        let mut config = Config::default();
        config.menu.title.title_height = 10;

        // Save config
        save_config_to_file(&config).expect("Failed to save config");

        // Load the config back
        let loaded_config = load_config_from_file().expect("Failed to load config");

        // Verify values
        assert_eq!(loaded_config.menu.title.title_height, 10);

        // Clean up
        unsafe {
            std::env::remove_var("FALLING_BLOCKS_CONFIG");
        }
    }
}
