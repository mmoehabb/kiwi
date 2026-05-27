use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PermissionsConfig {
    pub allowed_commands: Vec<String>,
    pub allowed_read_paths: Vec<String>,
    pub allowed_write_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub permissions: PermissionsConfig,
}

pub trait PermissionManager {
    fn is_command_allowed(&self, command: &str) -> bool;
    fn is_read_allowed(&self, path: &str) -> bool;
    fn is_write_allowed(&self, path: &str) -> bool;
}
