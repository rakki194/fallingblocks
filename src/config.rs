use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub music_enabled: bool,
    pub sound_enabled: bool,
    pub volume: f32,
    pub show_grid: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            music_enabled: true,
            sound_enabled: true,
            volume: 0.5,
            show_grid: false,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        // Try to load config from file
        match Self::load_from_file() {
            Ok(config) => config,
            Err(_) => {
                // If loading fails, create a default config and save it
                let default_config = Self::default();
                let _ = default_config.save(); // Ignore errors on initial save
                default_config
            }
        }
    }

    pub fn save(&self) -> io::Result<()> {
        // Create config directory if it doesn't exist
        let config_dir = Self::get_config_dir()?;
        fs::create_dir_all(&config_dir)?;

        // Create config file path
        let config_path = config_dir.join("config.json");

        // Serialize config to JSON
        let config_json = serde_json::to_string_pretty(self)?;

        // Write config to file
        fs::write(config_path, config_json)?;

        Ok(())
    }

    fn load_from_file() -> io::Result<Self> {
        let config_dir = Self::get_config_dir()?;
        let config_path = config_dir.join("config.json");

        // Read config file
        let config_json = fs::read_to_string(config_path)?;

        // Deserialize config from JSON
        let config = serde_json::from_str(&config_json)?;

        Ok(config)
    }

    fn get_config_dir() -> io::Result<std::path::PathBuf> {
        // Get home directory
        let home_dir = dirs::home_dir().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "Could not find home directory")
        })?;

        // Create config directory path (platform-specific)
        #[cfg(target_os = "linux")]
        let config_dir = home_dir.join(".config").join("fallingblocks");
        #[cfg(target_os = "macos")]
        let config_dir = home_dir
            .join("Library")
            .join("Application Support")
            .join("fallingblocks");
        #[cfg(target_os = "windows")]
        let config_dir = home_dir
            .join("AppData")
            .join("Roaming")
            .join("fallingblocks");
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        let config_dir = home_dir.join(".fallingblocks");

        Ok(config_dir)
    }
}
