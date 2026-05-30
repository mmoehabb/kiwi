//! The Plugin component provides extensibility to Kiwi via the Rhai scripting engine.
//! It allows users to write scripts (`.rhai` files) that can add new commands or alter behaviors.

use rhai::{Engine, Scope};

/// Trait defining the plugin management capabilities.
pub trait PluginManager {
    /// Loads all `.rhai` scripts from the given directory into the engine.
    fn load_plugins_from_dir(&mut self, dir_path: &str) -> Result<(), String>;

    /// Loads a `.rhai` script from the given file path into the engine.
    /// TODO: Implement directory watching to auto-reload plugins.
    fn load_script(&mut self, filepath: &str) -> Result<(), String>;

    /// Executes a specific function defined in a loaded plugin script.
    /// TODO: Make sure the execution environment is safely sandboxed.
    fn execute_function(&mut self, func_name: &str, args: Vec<String>) -> Result<String, String>;
}

use std::collections::HashMap;

/// The core engine struct handling Rhai integration.
pub struct RhaiEngine {
    pub engine: Engine,
    pub scope: Scope<'static>,
    /// A mapping of plugin names (derived from filename) to their compiled ASTs.
    pub asts: HashMap<String, rhai::AST>,
}

impl Default for RhaiEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RhaiEngine {
    pub fn new() -> Self {
        let mut engine = Engine::new();
        let scope = Scope::new();

        // Register Rust functions that plugins are allowed to call.
        engine.register_fn("log", |message: String| {
            println!("[Plugin Log] {}", message);
        });

        engine.register_fn("speak", |text: String| {
            println!("[Plugin Speak] {}", text);
        });

        Self {
            engine,
            scope,
            asts: HashMap::new(),
        }
    }
}

impl PluginManager for RhaiEngine {
    fn load_plugins_from_dir(&mut self, dir_path: &str) -> Result<(), String> {
        let paths = std::fs::read_dir(dir_path).map_err(|e| e.to_string())?;

        for entry in paths.flatten() {
            let filepath = entry.path();
            if filepath.is_file()
                && filepath.extension().and_then(|s| s.to_str()) == Some("rhai")
                && let Some(path_str) = filepath.to_str()
                && let Err(e) = self.load_script(path_str)
            {
                println!("[PluginManager] Failed to load script {}: {}", path_str, e);
            }
        }

        Ok(())
    }

    fn load_script(&mut self, filepath: &str) -> Result<(), String> {
        let path = std::path::Path::new(filepath);
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(filepath)
            .to_string();

        let ast = self
            .engine
            .compile_file(filepath.into())
            .map_err(|e| e.to_string())?;

        self.asts.insert(name, ast);
        Ok(())
    }

    fn execute_function(&mut self, func_name: &str, args: Vec<String>) -> Result<String, String> {
        let dynamic_args: Vec<rhai::Dynamic> = args.into_iter().map(rhai::Dynamic::from).collect();

        for ast in self.asts.values() {
            let options = rhai::CallFnOptions::new().eval_ast(true).rewind_scope(true);
            let result: Result<rhai::Dynamic, _> = self.engine.call_fn_with_options(
                options,
                &mut self.scope,
                ast,
                func_name,
                dynamic_args.clone(),
            );

            match result {
                Ok(val) => {
                    // Try to convert result to String.
                    return Ok(val.to_string());
                }
                Err(e) => {
                    if let rhai::EvalAltResult::ErrorFunctionNotFound(_, _) = *e {
                        // Function not found in this AST, try the next one.
                        continue;
                    } else {
                        // Function found but execution failed
                        return Err(e.to_string());
                    }
                }
            }
        }

        Err(format!("Function '{}' not found in any loaded plugin.", func_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

        let result = engine.execute_function("greet", vec!["World".to_string()]).unwrap();
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
        writeln!(file, "fn test_bindings() {{ log(\"Testing log\"); speak(\"Testing speak\"); return true; }}").unwrap();

        let path_str = file_path.to_str().unwrap();
        engine.load_script(path_str).unwrap();

        let result = engine.execute_function("test_bindings", vec![]).unwrap();
        assert_eq!(result, "true"); // Rhai boolean true converts to "true" string
    }
}
