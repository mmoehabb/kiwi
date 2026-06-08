import sys

def modify_config():
    with open("src/config.rs", "r") as f:
        content = f.read()

    new_method = """
    pub fn wakeword_templates_path() -> Result<PathBuf, String> {
        let mut path = dirs::data_local_dir().ok_or("Could not find user local data directory")?;
        path.push("kiwi");
        path.push("models");
        if !path.exists() {
            std::fs::create_dir_all(&path).map_err(|e| format!("Failed to create models dir: {}", e))?;
        }
        path.push("wakeword_templates.bin");
        Ok(path)
    }

    pub fn load(&mut self) -> Result<(), String> {"""

    content = content.replace("    pub fn load(&mut self) -> Result<(), String> {", new_method)

    with open("src/config.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify_config()
