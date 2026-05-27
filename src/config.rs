use kiwi_core::config::{AppConfig, PermissionManager};

/// Struct handling the loading and querying of configuration.
pub struct Configuration {
    pub config: AppConfig,
}

impl Default for Configuration {
    fn default() -> Self {
        Self::new()
    }
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
