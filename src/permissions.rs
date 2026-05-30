use glob::Pattern;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PermissionError {
    #[error("Failed to load or parse configuration: {0}")]
    ConfigError(String),
    #[error("Permission denied for command: {0}")]
    PermissionDenied(String),
    #[error("Failed to execute command: {0}")]
    ExecutionError(String),
}

#[derive(Debug, Deserialize, Default)]
pub struct PermissionsConfig {
    #[serde(default)]
    pub allowed_commands: Vec<String>,
}

pub struct PermissionManager {
    config: PermissionsConfig,
}

impl PermissionManager {
    /// Loads the permission configuration from `~/.config/kiwi/permissions.toml`.
    /// If the file does not exist, defaults to an empty list of allowed commands.
    pub fn load() -> Result<Self, PermissionError> {
        let config_path = dirs::config_dir()
            .map(|mut path| {
                path.push("kiwi");
                path.push("permissions.toml");
                path
            })
            .unwrap_or_else(|| {
                let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
                path.push(".config");
                path.push("kiwi");
                path.push("permissions.toml");
                path
            });

        Self::from_file(&config_path)
    }

    /// Loads the configuration from a specific file path.
    pub fn from_file(path: &Path) -> Result<Self, PermissionError> {
        if !path.exists() {
            return Ok(Self {
                config: PermissionsConfig::default(),
            });
        }

        let content = fs::read_to_string(path).map_err(|e| {
            PermissionError::ConfigError(format!("Failed to read {}: {}", path.display(), e))
        })?;

        let config: PermissionsConfig = toml::from_str(&content).map_err(|e| {
            PermissionError::ConfigError(format!("Failed to parse {}: {}", path.display(), e))
        })?;

        Ok(Self { config })
    }

    /// Checks if a command is allowed based on the configured wildcards.
    pub fn is_allowed(&self, cmd: &str) -> bool {
        for allowed in &self.config.allowed_commands {
            #[allow(clippy::collapsible_if)]
            if let Ok(pattern) = Pattern::new(allowed) {
                if pattern.matches(cmd) {
                    return true;
                }
            }
        }
        false
    }

    /// Executes the command via `sh -c` if permitted.
    pub fn execute(&self, cmd: &str) -> Result<(), PermissionError> {
        if !self.is_allowed(cmd) {
            return Err(PermissionError::PermissionDenied(cmd.to_string()));
        }

        let status = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .status()
            .map_err(|e| PermissionError::ExecutionError(e.to_string()))?;

        if status.success() {
            Ok(())
        } else {
            Err(PermissionError::ExecutionError(format!(
                "Command exited with status: {}",
                status
            )))
        }
    }
}
