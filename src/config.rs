/// The Config component manages `permissions.toml` and general application settings.
/// It enforces the strict, whitelist-based security model.
use serde::{Deserialize, Serialize};

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

/// The overall configuration for the application.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub permissions: PermissionsConfig,
    // TODO: Add other settings (e.g., model_path, wake_word_sensitivity).
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

use std::fs;
use std::path::Path;

/// Struct handling the loading and querying of configuration.
pub struct Configuration {
    pub config: AppConfig,
}

impl Configuration {
    pub fn new() -> Self {
        Self {
            config: AppConfig::default(),
        }
    }

    pub fn load_from_file(&mut self, filepath: &str) -> Result<(), String> {
        // Expand tilde if present
        let expanded_path = if filepath.starts_with("~/") {
            let home = std::env::var("HOME").map_err(|e| e.to_string())?;
            filepath.replacen("~", &home, 1)
        } else {
            filepath.to_string()
        };

        let path = Path::new(&expanded_path);

        if !path.exists() {
            // Automatically generate a default one
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create config dir: {}", e))?;
            }
            let default_toml = toml::to_string_pretty(&self.config)
                .map_err(|e| format!("Failed to serialize default config: {}", e))?;
            fs::write(path, default_toml)
                .map_err(|e| format!("Failed to write default config: {}", e))?;
            return Ok(());
        }

        let contents =
            fs::read_to_string(path).map_err(|e| format!("Failed to read config file: {}", e))?;
        self.config =
            toml::from_str(&contents).map_err(|e| format!("Failed to parse TOML: {}", e))?;

        Ok(())
    }
}

fn match_pattern(pattern: &str, target: &str) -> bool {
    let mut p_idx = 0;
    let mut t_idx = 0;
    let mut star_idx = None;
    let mut match_idx = 0;

    let p_chars: Vec<char> = pattern.chars().collect();
    let t_chars: Vec<char> = target.chars().collect();

    while t_idx < t_chars.len() {
        if p_idx < p_chars.len() && p_chars[p_idx] == t_chars[t_idx] {
            p_idx += 1;
            t_idx += 1;
        } else if p_idx < p_chars.len() && p_chars[p_idx] == '*' {
            star_idx = Some(p_idx);
            match_idx = t_idx;
            p_idx += 1;
        } else if let Some(star) = star_idx {
            p_idx = star + 1;
            match_idx += 1;
            t_idx = match_idx;
        } else {
            return false;
        }
    }

    while p_idx < p_chars.len() && p_chars[p_idx] == '*' {
        p_idx += 1;
    }

    p_idx == p_chars.len()
}

impl PermissionManager for Configuration {
    fn is_command_allowed(&self, command: &str) -> bool {
        self.config
            .permissions
            .allowed_commands
            .iter()
            .any(|pattern| match_pattern(pattern, command))
    }

    fn is_read_allowed(&self, path: &str) -> bool {
        self.config
            .permissions
            .allowed_read_paths
            .iter()
            .any(|pattern| match_pattern(pattern, path))
    }

    fn is_write_allowed(&self, path: &str) -> bool {
        self.config
            .permissions
            .allowed_write_paths
            .iter()
            .any(|pattern| match_pattern(pattern, path))
    }
}
