use kiwi::plugin::RhaiEngine;
use kiwi_core::plugin::PluginManager;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_load_script_and_execute() {
    let mut engine = RhaiEngine::new();

    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_plugin.rhai");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "fn greet(name) {{ return \"Hello, \" + name; }}").unwrap();

    let path_str = file_path.to_str().unwrap();
    engine.load_script(path_str).unwrap();

    assert!(engine.asts.contains_key("test_plugin"));

    let result = engine
        .execute_function("greet", vec!["World".to_string()])
        .unwrap();
    assert_eq!(result, "Hello, World");
}

#[test]
fn test_load_plugins_from_dir() {
    let mut engine = RhaiEngine::new();

    let dir = tempdir().unwrap();
    let file_path1 = dir.path().join("plugin1.rhai");
    let mut file1 = File::create(&file_path1).unwrap();
    writeln!(file1, "fn do_something() {{ return \"Done 1\"; }}").unwrap();

    let file_path2 = dir.path().join("plugin2.rhai");
    let mut file2 = File::create(&file_path2).unwrap();
    writeln!(file2, "fn do_other() {{ return \"Done 2\"; }}").unwrap();

    let dir_str = dir.path().to_str().unwrap();
    engine.load_plugins_from_dir(dir_str).unwrap();

    assert!(engine.asts.contains_key("plugin1"));
    assert!(engine.asts.contains_key("plugin2"));

    let res1 = engine.execute_function("do_something", vec![]).unwrap();
    assert_eq!(res1, "Done 1");

    let res2 = engine.execute_function("do_other", vec![]).unwrap();
    assert_eq!(res2, "Done 2");
}

#[test]
fn test_bindings_execution() {
    let mut engine = RhaiEngine::new();

    let dir = tempdir().unwrap();
    let file_path = dir.path().join("binding_test.rhai");
    let mut file = File::create(&file_path).unwrap();
    // Since we cannot easily capture stdout from Rhai in this simple test,
    // we just ensure calling log and speak does not panic.
    writeln!(
        file,
        "fn test_bindings() {{ log(\"Testing log\"); speak(\"Testing speak\"); return true; }}"
    )
    .unwrap();

    let path_str = file_path.to_str().unwrap();
    engine.load_script(path_str).unwrap();

    let result = engine.execute_function("test_bindings", vec![]).unwrap();
    assert_eq!(result, "true"); // Rhai boolean true converts to "true" string
}
