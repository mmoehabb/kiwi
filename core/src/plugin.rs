pub trait PluginManager {
    /// Loads all `.rhai` scripts from the given directory into the engine.
    fn load_plugins_from_dir(&mut self, dir_path: &str) -> Result<(), String>;
    /// Loads a `.rhai` script from the given file path into the engine.
    fn load_script(&mut self, filepath: &str) -> Result<(), String>;
    /// Executes a specific function defined in a loaded plugin script.
    fn execute_function(&mut self, func_name: &str, args: Vec<String>) -> Result<String, String>;
}
