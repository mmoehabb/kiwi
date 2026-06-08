//! The Config component manages `config.toml`, `permissions.toml` and general application settings.
//! It enforces the strict, whitelist-based security model.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Represents the permissions granted to Kiwi by the user.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PermissionsConfig {
    /// List of allowed shell commands (can include wildcards like `git *`).
    pub allowed_commands: Vec<String>,
    /// List of directories Kiwi is allowed to read from.
    pub allowed_read_paths: Vec<String>,
    /// List of directories Kiwi is allowed to write to.
    pub allowed_write_paths: Vec<String>,
}

fn default_model_name() -> String {
    "qwen2.5:1.5b".to_string()
}

fn default_wake_word() -> String {
    "hey kiwi".to_string()
}

fn default_wake_word_sensitivity() -> f32 {
    0.5
}

/// The overall configuration for the application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_model_name")]
    pub model_name: String,

    #[serde(default = "default_wake_word")]
    pub wake_word: String,

    #[serde(default = "default_wake_word_sensitivity")]
    pub wake_word_sensitivity: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            model_name: default_model_name(),
            wake_word: default_wake_word(),
            wake_word_sensitivity: default_wake_word_sensitivity(),
        }
    }
}

/// Trait defining how to check if an action is permitted.
pub trait PermissionManager {
    /// Checks if executing a specific shell command is allowed.
    /// TODO: Implement glob/wildcard matching against the whitelist.
    fn is_command_allowed(&self, command: &str) -> bool;

    /// Checks if a specific file path is allowed for reading.
    fn is_read_allowed(&self, path: &str) -> bool;

    /// Checks if a specific file path is allowed for writing.
    fn is_write_allowed(&self, path: &str) -> bool;
}

/// Struct handling the loading and querying of configuration.
#[derive(Debug, Clone, Default)]
pub struct Configuration {
    pub app: AppConfig,
    pub permissions: PermissionsConfig,
}

impl Configuration {
    pub fn new() -> Self {
        let mut config = Self::default();
        if let Err(e) = config.load() {
            eprintln!("Failed to load configuration: {}", e);
        }
        config
    }

    fn config_dir() -> Result<PathBuf, String> {
        let mut path = dirs::config_dir().ok_or("Could not find user config directory")?;
        path.push("kiwi");
        if !path.exists() {
            fs::create_dir_all(&path).map_err(|e| format!("Failed to create config dir: {}", e))?;
        }
        Ok(path)
    }

    pub fn wakeword_templates_path() -> Result<PathBuf, String> {
        let mut path = dirs::data_local_dir().ok_or("Could not find user local data directory")?;
        path.push("kiwi");
        path.push("models");
        if !path.exists() {
            std::fs::create_dir_all(&path)
                .map_err(|e| format!("Failed to create models dir: {}", e))?;
        }
        path.push("wakeword_templates.bin");
        Ok(path)
    }

    pub fn load(&mut self) -> Result<(), String> {
        let dir = Self::config_dir()?;

        // Load AppConfig
        let app_config_path = dir.join("config.toml");
        if app_config_path.exists() {
            let content = fs::read_to_string(&app_config_path)
                .map_err(|e| format!("Failed to read config.toml: {}", e))?;
            self.app = toml::from_str(&content)
                .map_err(|e| format!("Failed to parse config.toml: {}", e))?;
        } else {
            let content = toml::to_string(&self.app)
                .map_err(|e| format!("Failed to serialize default config: {}", e))?;
            fs::write(&app_config_path, content)
                .map_err(|e| format!("Failed to write default config.toml: {}", e))?;
        }

        // Load PermissionsConfig
        let permissions_path = dir.join("permissions.toml");
        if permissions_path.exists() {
            let content = fs::read_to_string(&permissions_path)
                .map_err(|e| format!("Failed to read permissions.toml: {}", e))?;
            self.permissions = toml::from_str(&content)
                .map_err(|e| format!("Failed to parse permissions.toml: {}", e))?;
        } else {
            let content = toml::to_string(&self.permissions)
                .map_err(|e| format!("Failed to serialize default permissions: {}", e))?;
            fs::write(&permissions_path, content)
                .map_err(|e| format!("Failed to write default permissions.toml: {}", e))?;
        }

        Ok(())
    }
}

impl PermissionManager for Configuration {
    fn is_command_allowed(&self, _command: &str) -> bool {
        // TODO: Iterate through `self.permissions.allowed_commands` and check for matches.
        false // Secure by default
    }

    fn is_read_allowed(&self, _path: &str) -> bool {
        // TODO: Verify the path falls within `allowed_read_paths`.
        false
    }

    fn is_write_allowed(&self, _path: &str) -> bool {
        // TODO: Verify the path falls within `allowed_write_paths`.
        false
    }
}
