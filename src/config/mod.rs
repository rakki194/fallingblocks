pub mod loader;
pub mod menu;

use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

// Global configuration instance with thread-safe access
pub static CONFIG: once_cell::sync::Lazy<Arc<RwLock<Config>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(Config::default())));

// Time to wait between checking for config file changes
const CONFIG_CHECK_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub menu: menu::MenuConfig,
    #[serde(skip)]
    last_modified: Option<Instant>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            menu: menu::MenuConfig::default(),
            last_modified: Some(Instant::now()),
        }
    }
}

impl Config {
    // Check if the config needs to be reloaded and reload if necessary
    pub fn check_and_reload() -> bool {
        let mut reloaded = false;

        // Check if enough time has passed since last check
        let now = Instant::now();
        let should_check = {
            let config = CONFIG.read().unwrap();
            if let Some(last_modified) = config.last_modified {
                now.duration_since(last_modified) > CONFIG_CHECK_INTERVAL
            } else {
                true
            }
        };

        if should_check {
            if let Ok(new_config) = loader::load_config_from_file() {
                let mut config = CONFIG.write().unwrap();
                *config = new_config;
                config.last_modified = Some(now);
                reloaded = true;
            }
        }

        reloaded
    }

    // Force reload the configuration from file
    pub fn force_reload() -> bool {
        if let Ok(new_config) = loader::load_config_from_file() {
            let mut config = CONFIG.write().unwrap();
            *config = new_config;
            config.last_modified = Some(Instant::now());
            true
        } else {
            false
        }
    }
}
