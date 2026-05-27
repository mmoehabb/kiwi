use kiwi_core::plugin::PluginManager;
use rhai::{Engine, Scope};

/// The core engine struct handling Rhai integration.
pub struct RhaiEngine {
    pub engine: Engine,
    pub scope: Scope<'static>,
    // TODO: Maintain a list of loaded plugin scripts or an AST cache.
}

impl Default for RhaiEngine {
    fn default() -> Self {
        Self::new()
    }
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
