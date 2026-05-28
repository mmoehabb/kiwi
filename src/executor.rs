use crate::config::PermissionManager;
use std::process::Command;

pub struct CommandExecutor<'a> {
    permission_manager: &'a dyn PermissionManager,
}

impl<'a> CommandExecutor<'a> {
    pub fn new(permission_manager: &'a dyn PermissionManager) -> Self {
        Self { permission_manager }
    }

    pub fn execute(&self, command: &str) -> Result<String, String> {
        if !self.permission_manager.is_command_allowed(command) {
            return Err("Permission Denied".to_string());
        }

        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .map_err(|e| format!("Failed to execute command: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }
}
