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

    pub fn load_from_file(&mut self, _filepath: &str) -> Result<(), String> {
        // TODO: Read the TOML file and deserialize it into `AppConfig`.
        Ok(())
    }
}

impl PermissionManager for Configuration {
    fn is_command_allowed(&self, _command: &str) -> bool {
        // TODO: Iterate through `self.config.permissions.allowed_commands` and check for matches.
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
