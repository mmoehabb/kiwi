/// The Plugin component provides extensibility to Kiwi via the Rhai scripting engine.
/// It allows users to write scripts (`.rhai` files) that can add new commands or alter behaviors.

use rhai::{Engine, Scope};

/// Trait defining the plugin management capabilities.
pub trait PluginManager {
    /// Loads a `.rhai` script from the given file path into the engine.
    /// TODO: Implement directory watching to auto-reload plugins.
    fn load_script(&mut self, filepath: &str) -> Result<(), String>;

    /// Executes a specific function defined in a loaded plugin script.
    /// TODO: Make sure the execution environment is safely sandboxed.
    fn execute_function(&mut self, func_name: &str, args: Vec<String>) -> Result<String, String>;
}

/// The core engine struct handling Rhai integration.
pub struct RhaiEngine {
    pub engine: Engine,
    pub scope: Scope<'static>,
    // TODO: Maintain a list of loaded plugin scripts or an AST cache.
}

impl RhaiEngine {
    pub fn new() -> Self {
        let engine = Engine::new();
        let scope = Scope::new();

        // TODO: Register Rust functions that plugins are allowed to call (e.g., `speak`, `search`).
        // engine.register_fn("say", |text: String| { ... });

        Self { engine, scope }
    }
}

impl PluginManager for RhaiEngine {
    fn load_script(&mut self, _filepath: &str) -> Result<(), String> {
        // TODO: Read the file, compile it to an AST using `self.engine.compile()`.
        Ok(())
    }

    fn execute_function(&mut self, _func_name: &str, _args: Vec<String>) -> Result<String, String> {
        // TODO: Evaluate the AST function utilizing `self.engine.call_fn()`.
        Ok("Plugin executed.".to_string())
    }
}
