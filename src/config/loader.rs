#![warn(clippy::all, clippy::pedantic)]

use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::time::SystemTime;
use toml;

use super::Config;

// Default config file path
const CONFIG_FILE_PATH: &str = "config/falling_blocks.toml";

// Last modified time of the config file
static mut LAST_MODIFIED: Option<SystemTime> = None;

// Load the configuration from the file system
pub fn load_config_from_file() -> Result<Config, ConfigError> {
    let config_path = get_config_file_path();

    // Create default config directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    // Check if config file exists
    if !config_path.exists() {
        // Create default config file if it doesn't exist
        let default_config = Config::default();
        save_config_to_file(&default_config)?;
        return Ok(default_config);
    }

    // Check if file has been modified
    let metadata = fs::metadata(&config_path)?;
    let last_modified = metadata.modified()?;

    unsafe {
        if let Some(previous_modified) = LAST_MODIFIED {
            if previous_modified == last_modified {
                // File hasn't changed, return current config
                return Ok(super::CONFIG.read().unwrap().clone());
            }
        }
        LAST_MODIFIED = Some(last_modified);
    }

    // Read and parse config file
    let mut file = fs::File::open(&config_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}

// Save the configuration to the file system
pub fn save_config_to_file(config: &Config) -> Result<(), ConfigError> {
    let config_path = get_config_file_path();

    // Create parent directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    // Serialize config to TOML
    let toml_string = toml::to_string_pretty(config)?;

    // Write to file
    fs::write(&config_path, toml_string)?;

    // Update last modified time
    let metadata = fs::metadata(&config_path)?;
    unsafe {
        LAST_MODIFIED = metadata.modified().ok();
    }

    Ok(())
}

// Get the path to the config file
fn get_config_file_path() -> PathBuf {
    // Check for environment variable override
    if let Ok(path) = std::env::var("FALLING_BLOCKS_CONFIG") {
        return PathBuf::from(path);
    }

    // Otherwise use default path in user's config directory
    if let Some(config_dir) = dirs::config_dir() {
        config_dir.join("falling_blocks").join("config.toml")
    } else {
        // Fallback to local directory
        PathBuf::from(CONFIG_FILE_PATH)
    }
}

// Custom error type for configuration operations
#[derive(Debug)]
pub enum ConfigError {
    Io(io::Error),
    Parse(toml::de::Error),
    Serialize(toml::ser::Error),
}

impl From<io::Error> for ConfigError {
    fn from(err: io::Error) -> Self {
        ConfigError::Io(err)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(err: toml::de::Error) -> Self {
        ConfigError::Parse(err)
    }
}

impl From<toml::ser::Error> for ConfigError {
    fn from(err: toml::ser::Error) -> Self {
        ConfigError::Serialize(err)
    }
}
