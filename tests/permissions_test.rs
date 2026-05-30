use kiwi::permissions::{PermissionError, PermissionManager};
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_default_deny_all() {
    let temp_file = NamedTempFile::new().unwrap();
    // Delete file to simulate missing config
    let path = temp_file.path().to_path_buf();
    drop(temp_file);

    let manager = PermissionManager::from_file(&path).unwrap();
    assert!(!manager.is_allowed("ls"));

    let result = manager.execute("ls");
    assert!(matches!(result, Err(PermissionError::PermissionDenied(_))));
}

#[test]
fn test_allow_specific_command() {
    let mut temp_file = NamedTempFile::new().unwrap();
    let toml_content = r#"
allowed_commands = [
    "echo 'hello world'"
]
"#;
    temp_file.write_all(toml_content.as_bytes()).unwrap();

    let manager = PermissionManager::from_file(temp_file.path()).unwrap();

    assert!(manager.is_allowed("echo 'hello world'"));
    assert!(!manager.is_allowed("echo 'goodbye'"));

    let result = manager.execute("echo 'hello world'");
    assert!(result.is_ok());

    let fail_result = manager.execute("echo 'goodbye'");
    assert!(matches!(
        fail_result,
        Err(PermissionError::PermissionDenied(_))
    ));
}

#[test]
fn test_allow_wildcard_command() {
    let mut temp_file = NamedTempFile::new().unwrap();
    let toml_content = r#"
allowed_commands = [
    "echo *"
]
"#;
    temp_file.write_all(toml_content.as_bytes()).unwrap();

    let manager = PermissionManager::from_file(temp_file.path()).unwrap();

    assert!(manager.is_allowed("echo 'hello'"));
    assert!(manager.is_allowed("echo anything"));
    assert!(!manager.is_allowed("ls"));

    let result = manager.execute("echo 'test execution'");
    assert!(result.is_ok());
}

#[test]
fn test_execution_failure() {
    let mut temp_file = NamedTempFile::new().unwrap();
    let toml_content = r#"
allowed_commands = [
    "false"
]
"#;
    temp_file.write_all(toml_content.as_bytes()).unwrap();

    let manager = PermissionManager::from_file(temp_file.path()).unwrap();

    assert!(manager.is_allowed("false"));

    let result = manager.execute("false");
    assert!(matches!(result, Err(PermissionError::ExecutionError(_))));
}
