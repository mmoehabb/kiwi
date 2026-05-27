pub trait PluginManager {
    fn load_script(&mut self, filepath: &str) -> Result<(), String>;
    fn execute_function(&mut self, func_name: &str, args: Vec<String>) -> Result<String, String>;
}
